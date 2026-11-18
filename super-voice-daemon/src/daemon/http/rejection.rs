// SPDX-License-Identifier: GPL-3.0-only
//! The house error envelope for requests axum refuses before a handler runs.
//!
//! A body that is not the JSON a route takes, a path segment that does not
//! parse, an upload over a route's size limit: axum answers each from its
//! extractor, as `text/plain` — `Failed to deserialize the JSON body into the
//! target type: volume: invalid type: floating point `2.0`, expected u8` with
//! a `422`. Every other error the daemon gives is a JSON envelope with an
//! `error_code`, and the client's error mapping reads that envelope; a bare
//! sentence reached a user as `daemon returned HTTP 422`. The handlers never
//! answer `text/plain`, so a client error in it is one of these, and
//! [`envelope_rejection`] rewrites it.

use axum::http::{StatusCode, header};
use axum::response::Response;

/// How much of a rejection's text to read: axum's are one line.
const MAX_REJECTION_BYTES: usize = 64 * 1024;

/// Rewrite a `text/plain` client error into the envelope, keeping axum's
/// sentence as the `message`.
///
/// `422` becomes `400`: the pages document a malformed body as a `400`, and a
/// value of the wrong type is as malformed as one out of range, which the
/// handlers already answer `400 invalid_value`. An upload over its limit keeps
/// its `413` and is `payload_too_large`; anything else keeps its status and is
/// `invalid_value`.
pub(crate) async fn envelope_rejection(response: Response) -> Response {
    let status = response.status();
    let plain_text = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("text/plain"));
    if !status.is_client_error() || !plain_text {
        return response;
    }
    let text = axum::body::to_bytes(response.into_body(), MAX_REJECTION_BYTES)
        .await
        .map_or_else(
            |_| String::new(),
            |bytes| String::from_utf8_lossy(&bytes).trim().to_string(),
        );
    let (status, error_code) = match status {
        StatusCode::UNPROCESSABLE_ENTITY => (StatusCode::BAD_REQUEST, "invalid_value"),
        StatusCode::PAYLOAD_TOO_LARGE => (status, "payload_too_large"),
        _ => (status, "invalid_value"),
    };
    super::v1::wire::json_error_msg(status, error_code, &text)
}

#[cfg(test)]
mod tests {
    use super::envelope_rejection;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};

    async fn parts(resp: Response) -> (StatusCode, String, serde_json::Value) {
        let status = resp.status();
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("collect");
        (
            status,
            ct,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    /// axum's answer to `{"volume": 2.0}` on a route taking a `u8`.
    #[tokio::test]
    async fn a_body_of_the_wrong_type_is_a_400_with_an_error_code() {
        let rejection = (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Failed to deserialize the JSON body into the target type: volume: invalid type",
        )
            .into_response();

        let (status, ct, body) = parts(envelope_rejection(rejection).await).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(ct, "application/json");
        assert_eq!(body["status"], "error");
        assert_eq!(body["error_code"], "invalid_value");
        assert_eq!(
            body["message"],
            "Failed to deserialize the JSON body into the target type: volume: invalid type"
        );
    }

    #[tokio::test]
    async fn an_oversized_upload_keeps_its_413() {
        let rejection = (StatusCode::PAYLOAD_TOO_LARGE, "length limit exceeded").into_response();

        let (status, _, body) = parts(envelope_rejection(rejection).await).await;

        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body["error_code"], "payload_too_large");
    }

    /// What the handlers answer is theirs: a JSON error, and anything that is
    /// not a client error, pass through untouched.
    #[tokio::test]
    async fn the_handlers_own_answers_pass_through() {
        let json = super::super::v1::wire::json_error_msg(
            StatusCode::NOT_FOUND,
            "unknown_backend",
            "no such backend",
        );
        let (status, _, body) = parts(envelope_rejection(json).await).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error_code"], "unknown_backend");

        let server_error = (StatusCode::INTERNAL_SERVER_ERROR, "something broke").into_response();
        let resp = envelope_rejection(server_error).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("collect");
        assert_eq!(&bytes[..], b"something broke");
    }
}
