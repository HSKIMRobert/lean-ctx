//! POST /v1/quality-lab — run Quality Lab analysis on a text pair.

use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::core::quality_lab::orchestrator::{QualityLabReport, run_quality_lab};

#[derive(Deserialize)]
pub(crate) struct QualityLabRequest {
    #[serde(default)]
    pub original: String,
    #[serde(default)]
    pub compressed: String,
    #[serde(default = "default_ext")]
    pub ext: String,
}

fn default_ext() -> String {
    "rs".to_string()
}

#[derive(Serialize)]
pub(crate) struct QualityLabResponse {
    pub report: QualityLabReport,
}

pub(crate) async fn handler(Json(req): Json<QualityLabRequest>) -> Response {
    let report = run_quality_lab(&req.original, &req.compressed, &req.ext);

    Json(QualityLabResponse { report }).into_response()
}

pub(crate) async fn handler_get() -> Response {
    let report = run_quality_lab("", "", "");
    Json(QualityLabResponse { report }).into_response()
}
