# `POST /tts/synthesize`

Synthesize `text` with the active model and answer with the audio: a WAV file for
the client to keep, played nowhere. A clip an app plays again later, or saves.

It is spoken the way [`POST /tts/speak`](./speak.md) would speak it: the same
normalizing, the same sentence splitting, the user's voice and language. What
differs is where the audio goes.

**It is not an utterance.** It does not interrupt what is playing, nothing
playing interrupts it, and it publishes no `speaking_state` or
`speech_progress`. [`/tts/speak/stop`](./speak/stop.md) does not stop it; a
client that hangs up does.

The answer comes once the whole text is synthesized, which for a long text takes
as long as the model needs. While it runs the model counts as in use: idle
unload ([`/settings/idle_unload`](../settings/idle_unload.md)) does not put it
to sleep partway, and counts its idle time from the end. A request that finds
the selected model unloaded loads it first, as `speak` does.

One file is one model's voice: a model switched partway fails the request
(`500`) rather than finishing it in another voice.

## Auth

- **Required scope:** [`synthesize`](../../../scopes/synthesize.md), not
  `speak`: the audio leaves the daemon in the user's voice, cloned or not, and
  making the machine talk does not grant that.
- `Authorization: Bearer <session_token>` is required.
- Tokens without the `synthesize` scope get `403 scope_denied`.

## Request

```http
POST /tts/synthesize HTTP/1.1
Host: voice.local
Authorization: Bearer …64hex…
Content-Type: application/json

{
  "text": "The kettle is boiling."
}
```

| Field  | Type   | Required | Meaning                                                        |
|--------|--------|----------|-----------------------------------------------------------------|
| `text` | string | yes      | What to say. Empty or whitespace-only is `400 invalid_value`.  |

**That is the whole request**, as on [`POST /tts/speak`](./speak.md#request): the
voice and the language are the user's settings, and `voice`, `language`,
`speed` and `instructions` are refused by name rather than ignored. `text` has
the same cap, 50 000 characters or the model's own `max_input_chars`.

## Response (200)

```http
HTTP/1.1 200 OK
Content-Type: audio/wav

RIFF…WAVEfmt …data…
```

The body is the WAV file itself, not JSON: **mono, 16-bit PCM, at the rate the
model speaks in** (24 kHz for most). Every sentence is in it, end to end, each
seam faded as playback fades it so two sentences synthesized apart do not click
where they meet. A model that answers in stereo, or changes rate between
sentences, is brought to one channel at the first sentence's rate.

## Errors

Errors answer with the house JSON envelope.

| HTTP | `error_code`        | Meaning                                                                                     |
|------|---------------------|---------------------------------------------------------------------------------------------|
| 400  | `invalid_value`     | `text` missing, empty, or over the cap; one of the removed fields; or a malformed body.     |
| 401  | `invalid_session`   | Token unknown / expired / `exe_changed` — re-auth and retry.                                |
| 403  | `scope_denied`      | Token lacks the `synthesize` scope.                                                         |
| 409  | `model_not_loaded`  | No model is selected, or it failed to load.                                                 |
| 409  | `voice_required`    | The selected model speaks only in a voice the user chooses, and none is chosen. Set one with [`POST /tts/pipeline/{stage}/model/{model}/voice`](./pipeline/voice.md). |
| 429  | `rate_limited`      | Too many requests; back off.                                                                |
| 500  | —                   | The backend failed, produced no audio, or the model changed partway. `message` carries the reason. |

Unlike `speak`, a refusal raises no desktop notification: the caller is an app
waiting on the answer, not a keyboard shortcut with nowhere to show it.
