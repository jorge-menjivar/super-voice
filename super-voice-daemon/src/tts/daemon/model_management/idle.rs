// SPDX-License-Identifier: GPL-3.0-only
//! What the voices page says about the model loaded on demand. See
//! [`crate::config::IdleUnloadConfig`]. The load on demand and the idle
//! unload themselves are the engine's
//! ([`StageExt`](super_engine_daemon::stage::StageExt)), the same for every
//! stage.

use std::future::Future;
use std::sync::Arc;

use crate::tts::daemon::events::Topic;
use crate::tts::daemon::types::SuperTTSDaemon;

/// Whether a `speaking_state` payload says the speaking stopped.
fn speaking_stopped(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|event| event.get("is_speaking")?.as_bool())
        == Some(false)
}

impl SuperTTSDaemon {
    /// Count the speech model's idle time from the end of each utterance,
    /// until `shutdown`.
    ///
    /// The idle loop counts an utterance being spoken as use only when it
    /// checks, every few seconds, so the model went to sleep up to one check
    /// early: counted from the last check while it spoke, not from when the
    /// speaking ended. Every end is announced as `speaking_state` going false,
    /// so the model is touched then.
    pub(crate) fn spawn_touch_on_speech_end(
        &self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) {
        let mut speaking = self.events.subscribe(Topic::SpeakingState);
        let model = Arc::clone(&self.model);
        tokio::spawn(async move {
            tokio::pin!(shutdown);
            loop {
                tokio::select! {
                    () = &mut shutdown => break,
                    event = speaking.recv_json_str() => match event {
                        Ok((_, payload)) => {
                            if speaking_stopped(&payload) {
                                model.touch();
                            }
                        }
                        // Fell behind: an end may be among what was missed.
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            model.touch();
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                }
            }
        });
    }

    /// The model the next request loads when it finds none in memory: the
    /// selected one, when models are unloaded on idle. Quiet: the voices page
    /// reads it on every listing.
    pub(crate) async fn model_loaded_on_demand(
        &self,
    ) -> Option<crate::tts::tts_models::ModelDefinition> {
        self.config.read().await.idle_unload.after()?;
        let (name, source) = self.selected_model().await.ok()?;
        self.resolve_definition(&name, &source).await
    }
}

#[cfg(test)]
mod tests {
    use super::speaking_stopped;
    use std::time::Duration;

    /// Only the end of speaking counts: a start is use that the idle loop
    /// sees for itself.
    #[test]
    fn only_a_stop_is_the_end_of_speaking() {
        assert!(speaking_stopped(
            r#"{"is_speaking":false,"utterance_id":"utt_1"}"#
        ));
        assert!(!speaking_stopped(
            r#"{"is_speaking":true,"utterance_id":"utt_1"}"#
        ));
        assert!(!speaking_stopped("not json"));
    }

    /// The speech model's idle time starts again when an utterance ends.
    #[tokio::test]
    async fn the_end_of_an_utterance_restarts_the_models_idle_time() {
        let daemon = crate::tts::daemon::types::test_daemon().await;
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        daemon.spawn_touch_on_speech_end(async move {
            let _ = stopped.await;
        });
        daemon.model.backdate(Duration::from_secs(60));

        daemon
            .events
            .publish_speaking_state(false, Some("utt_1".to_string()));

        let mut touched = false;
        for _ in 0..100 {
            if daemon.model.idle_for() < Duration::from_secs(5) {
                touched = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(touched, "the model still counts its idle time from before");
        let _ = stop.send(());
    }
}
