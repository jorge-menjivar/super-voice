// SPDX-License-Identifier: GPL-3.0-only
//! Loading the pipeline on demand when models are unloaded on idle. See
//! [`crate::config::IdleUnloadConfig`]. Each stage's load and unload is the
//! engine's ([`StageExt`]), the same for every stage: this is only which
//! stages a request loads, and when it waits for them.

use futures::FutureExt as _;
use tokio::task::{JoinError, JoinHandle};

use crate::daemon::device_management::PipelineStage;
use crate::daemon::types::SuperSTTDaemon;
use crate::stt_models::ModelDefinition;
use super_engine_daemon::stage::StageExt;

/// The models a recording loads while it captures: started with the
/// recording, and waited for only once its transcript needs them.
pub(crate) struct PipelineLoad {
    /// The transcription model's load. The later stages' run on their own:
    /// each waits for its model when the transcript reaches it.
    transcription: JoinHandle<Result<(), String>>,
    /// How the load ended, once that has been seen.
    outcome: Option<Result<(), String>>,
}

impl PipelineLoad {
    /// How the transcription model's load ended, or `None` while it runs.
    pub(crate) fn finished(&mut self) -> Option<&Result<(), String>> {
        if self.outcome.is_none() {
            // Polled once more only while it has not answered: a task's
            // handle is not to be polled after it has.
            self.outcome = (&mut self.transcription).now_or_never().map(joined);
        }
        self.outcome.as_ref()
    }

    /// Why the transcription model failed to load, once it has.
    pub(crate) fn failure(&mut self) -> Option<String> {
        match self.finished() {
            Some(Err(detail)) => Some(detail.clone()),
            _ => None,
        }
    }

    /// Wait for the transcription model's load to end.
    ///
    /// # Errors
    /// It failed: which model, and why.
    pub(crate) async fn wait(mut self) -> Result<(), String> {
        match self.outcome.take() {
            Some(outcome) => outcome,
            None => joined(self.transcription.await),
        }
    }
}

/// A load task's answer, or why it gave none.
fn joined(result: Result<Result<(), String>, JoinError>) -> Result<(), String> {
    result.unwrap_or_else(|e| Err(format!("the load stopped before it finished: {e}")))
}

impl SuperSTTDaemon {
    /// Record a use of every stage a final transcript runs through and, when
    /// models are unloaded on idle, load those not in memory, all at once.
    ///
    /// # Errors
    /// The transcription model failed to load: which, and why. A later stage
    /// is best-effort, as it is on every transcript: one that failed to load
    /// is logged and announced, and the transcript passes through it
    /// unchanged.
    pub(crate) async fn ensure_pipeline_loaded(&self) -> Result<(), String> {
        let loads = PipelineStage::ALL.map(|stage| {
            let stage = self.stage(stage);
            async move { stage.ensure_loaded().await }
        });
        // In stage order: the first transcribes, and without it there is no
        // transcript to pass on.
        futures::future::join_all(loads)
            .await
            .into_iter()
            .next()
            .unwrap_or(Ok(()))
    }

    /// Start what [`ensure_pipeline_loaded`](Self::ensure_pipeline_loaded)
    /// does for a recording, and return at once: the recording captures while
    /// the models load, so the user speaks while they would otherwise wait.
    ///
    /// `None` when there is no transcription model to load: none is in
    /// memory, and none loads on demand. The recording would end in a
    /// discarded take, so it is refused before capture.
    ///
    /// The loads are not cancelled with the recording: the models are then in
    /// memory for the next one.
    pub(crate) async fn load_for_recording(&self) -> Option<PipelineLoad> {
        if self.model.read().await.is_none()
            && !self
                .stage(PipelineStage::Transcription)
                .loads_on_demand()
                .await
        {
            return None;
        }
        let load = |stage| {
            let stage = self.stage(stage);
            tokio::spawn(async move { stage.ensure_loaded().await })
        };
        let transcription = load(PipelineStage::Transcription);
        for stage in PipelineStage::ALL {
            if stage != PipelineStage::Transcription {
                // Detached: the stage waits for its own model when the
                // transcript reaches it.
                drop(load(stage));
            }
        }
        Some(PipelineLoad {
            transcription,
            outcome: None,
        })
    }

    /// Count every stage's idle time from now: a request that just ended
    /// used them all, a recording or a transcription of uploaded audio.
    ///
    /// Each counts its use when it starts, and the idle loop counts a
    /// recording in progress as use only when it checks, every few seconds.
    /// Without this a model went to sleep up to one check early: counted from
    /// the last check during a take, not from its end.
    pub(crate) fn finished_using_pipeline(&self) {
        self.model.touch();
        self.post_processor.touch();
    }

    /// The transcription model's definition: the loaded model's, or while it
    /// is not in memory, the selected one's. What a recording needs to know
    /// about its model before the model has loaded.
    pub(crate) async fn transcription_definition(&self) -> Option<ModelDefinition> {
        if let Some(loaded) = self.model.read().await.as_ref() {
            return Some(loaded.definition.clone());
        }
        let (model, source) = self
            .selected_model(PipelineStage::Transcription)
            .await
            .ok()?;
        self.find_model_definition(&source, &model).await
    }
}
