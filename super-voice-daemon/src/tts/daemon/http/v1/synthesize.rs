// SPDX-License-Identifier: GPL-3.0-only
//! `POST /v1/tts/synthesize` — text spoken into a WAV file for the client to
//! keep, played nowhere.
//!
//! Contract: `docs/protocol/endpoints/v1/tts/synthesize.md`.
//!
//! The body is `POST /speak`'s, refused the same way. The work is the speech
//! engine's ([`SpeechEngine::synthesize_wav`]); this answers with its bytes.
//!
//! [`SpeechEngine::synthesize_wav`]: crate::tts::daemon::speech::SpeechEngine::synthesize_wav

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use super_voice_shared::tts::models::protocol::{DaemonResponse, ErrorCode};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::speak::{SpeakBody, refuse_removed_fields};
use crate::tts::daemon::http::internal::helpers::dispatch::json_response;
use crate::tts::daemon::http::state::AppState;
use crate::tts::daemon::http::wire::{ErrorEnvelope, ReasonEnvelope};
use crate::tts::daemon::speech::speech_refusal;

#[utoipa::path(
    post,
    path = "/synthesize",
    tag = "synthesize",
    summary = "Speak a piece of text into a WAV file",
    description = "\
Synthesizes `text` with the loaded model and answers with the audio, for the client \
to keep: a clip to play again later, or to save. Nothing is played.

The response body is `audio/wav`, not JSON: mono 16-bit PCM at the rate the model \
speaks in. Errors answer with the house JSON envelope.

It is spoken as `POST /speak` would speak it: the same normalizing, the same \
sentence splitting, the user's voice and language. **`text` is the whole request**, \
and `voice`, `language`, `speed` and `instructions` are refused as they are there.

It needs the `synthesize` scope, not `speak`: the audio leaves the daemon, in the \
user's voice, cloned or not, and making the machine talk does not grant that.

It is not an utterance. It does not interrupt what is playing, nothing playing \
interrupts it, and it publishes no `speaking_state`. The answer comes once the whole \
text is synthesized, which for a long text takes as long as the model needs; a \
client that hangs up stops it. While it runs, the model counts as in use, so idle \
unload does not put it to sleep partway.",
    request_body(content = SpeakBody, description = "`text`, and nothing else, as on `POST /speak`."),
    security(("session_token" = ["synthesize"])),
    responses(
        (status = 200, description = "The text, spoken: mono 16-bit PCM WAV at the model's rate.",
         content_type = "audio/wav"),
        (status = 400, description = "`text` missing, empty, or over the cap; a request still carrying the removed `voice`/`language`/`speed`/`instructions` fields; or a malformed body.", body = ErrorEnvelope),
        (status = 401, description = "Token unknown, expired, or its binary changed.", body = ReasonEnvelope),
        (status = 403, description = "The token lacks the `synthesize` scope.", body = ErrorEnvelope),
        (status = 409, description = "No model is loaded (`model_not_loaded`), or the model speaks only in a voice the user chooses and none is chosen (`voice_required`).", body = ErrorEnvelope),
        (status = 429, description = "Per-client rate limit hit; back off and retry.", body = ErrorEnvelope),
        (status = 500, description = "The backend failed, produced no audio, or the model changed partway through. `message` carries the reason.", body = ErrorEnvelope),
    ),
)]
/// `POST /v1/tts/synthesize` — `text` spoken into a WAV file.
///
/// Run inline, unlike `POST /speak`: the work is for this client alone, so a
/// client that hangs up should take it with it, and axum dropping the future
/// is what does that.
pub(crate) async fn synthesize(
    State(state): State<AppState>,
    body: Option<axum::Json<Value>>,
) -> Response {
    let Some(axum::Json(body)) = body else {
        return json_response(&DaemonResponse::error_with_code(
            ErrorCode::InvalidValue,
            "missing request body",
        ))
        .into_response();
    };
    let Some(text) = body.get("text").and_then(Value::as_str) else {
        return json_response(&DaemonResponse::error_with_code(
            ErrorCode::InvalidValue,
            "missing text",
        ))
        .into_response();
    };
    if let Some(refusal) = refuse_removed_fields(&body, "/tts/synthesize") {
        return refusal;
    }
    match state.daemon.handle_synthesize(text).await {
        Ok(wav) => (
            StatusCode::OK,
            [("content-type", "audio/wav")],
            axum::body::Bytes::from(wav),
        )
            .into_response(),
        Err(refusal) => json_response(&speech_refusal(refusal)).into_response(),
    }
}

pub(crate) fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(synthesize))
}
