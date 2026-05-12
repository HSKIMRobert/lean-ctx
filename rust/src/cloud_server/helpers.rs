use axum::http::StatusCode;

pub(super) fn internal_error(e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!("internal server error: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        r#"{"error":"internal_error"}"#.to_string(),
    )
}
