# `/settings/volume`

Read and set the audio cue volume on a 0–100 scale. `0` mutes the
cues without changing the active theme; `100` is full. The theme
itself is read / set via [`/settings/audio_theme`](./audio_theme.md).

## Auth

- **Required scope:** `settings`.
- `Authorization: Bearer <session_token>` is required.
- Tokens without the `settings` scope get `403 scope_denied`.

## `POST /settings/volume`

**Request:**

```http
POST /settings/volume HTTP/1.1
Host: voice.local
Authorization: Bearer …64hex…
Content-Type: application/json

{
  "volume": 75
}
```

| Field    | Type | Required | Notes                              |
|----------|------|----------|------------------------------------|
| `volume` | u8   | yes      | Integer in `0..=100`               |

**Response (200):**

```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status":  "success",
  "message": "Volume set to: 75"
}
```

**Errors:**

| HTTP | `message`         | Meaning                                                       |
|------|-------------------|---------------------------------------------------------------|
| 400  | `error_code: invalid_value` | `volume` outside `0..=100` |
| 401  | `invalid_session` | Token unknown / expired / `exe_changed`                       |
| 403  | `scope_denied`    | Token lacks the `settings` scope                              |

## `GET /settings/volume`

**Request:**

```http
GET /settings/volume HTTP/1.1
Host: voice.local
Authorization: Bearer …64hex…
```

**Response (200):**

```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status":  "success",
  "message": "75"
}
```

The `message` carries the current volume as a bare number, `"75"`, not
as a field of its own, so parse it back out.

**Errors:**

| HTTP | `message`         | Meaning                                                       |
|------|-------------------|---------------------------------------------------------------|
| 401  | `invalid_session` | Token unknown / expired / `exe_changed`                       |
| 403  | `scope_denied`    | Token lacks the `settings` scope                              |
