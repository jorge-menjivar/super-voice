# synthesize scope

> Scope: **synthesize** (make spoken audio of any text, and take it away as a
> file).

The `synthesize` scope is speech kept rather than played. A `synthesize` token
can send text to [`POST /tts/synthesize`](../endpoints/v1/tts/synthesize.md)
and get back the spoken audio as a WAV file. That is the whole surface.

The audio is spoken the way [`speak`](./speak.md) would speak it: in the voice
and language the user chose. The request carries `text` and nothing else, so a
`synthesize` token cannot pick another voice either; an app that genuinely needs
to asks for [`settings`](./settings.md) too.

## Why it is not part of `speak`

`speak` buys the speakers: what an app says with it is heard in the room, once,
and is gone. `synthesize` hands the audio to the app, to keep, copy and send
anywhere. And it is in the user's chosen voice, which may be a voice cloned from
a recording of the user or of someone else. Audio of a person saying whatever an
app wrote is not something making the machine talk should imply.

So it gets its own scope, the consent prompt names it in those terms ("make
spoken audio of any text, and keep it as a file"; "that audio is in your chosen
voice, even one cloned from a recording"), and a client that wants to do both
asks for both. Scopes never imply one another: a `synthesize` token cannot make
the machine speak, and a `speak` token cannot take audio away.

It grants nothing over the cloned-voice library: the recordings themselves stay
behind [`voices`](./voices.md).

## Endpoint reference

| Endpoint                                                  | Methods | Notes                                                        |
|-----------------------------------------------------------|---------|--------------------------------------------------------------|
| [`/tts/synthesize`](../endpoints/v1/tts/synthesize.md)    | POST    | Speak complete text into a WAV file; plays nothing. `text` is the whole request |

All traffic is HTTP/1.1 over the Unix domain socket at
`<runtime dir>/voice/super-voice-http.sock`. See [transport.md](../transport.md)
for the wire-level details.
