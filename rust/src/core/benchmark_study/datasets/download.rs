//! Dataset auto-download: fetches HumanEval/MBPP NDJSON if not present.

use std::path::{Path, PathBuf};

const HUMANEVAL_URL: &str =
    "https://raw.githubusercontent.com/openai/human-eval/master/data/HumanEval.jsonl.gz";
const MBPP_URL: &str =
    "https://raw.githubusercontent.com/google-research-datasets/mbpp/master/mbpp.jsonl";

/// Ensure the HumanEval dataset exists at the expected path.
/// Downloads and decompresses if missing.
pub(crate) fn ensure_humaneval(data_dir: &Path) -> Result<PathBuf, String> {
    let target = data_dir.join("humaneval.ndjson");
    if target.exists() {
        return Ok(target);
    }
    std::fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;

    tracing::info!("downloading HumanEval dataset...");
    let agent = build_agent();
    let resp = agent
        .get(HUMANEVAL_URL)
        .call()
        .map_err(|e| format!("download HumanEval: {e}"))?;

    let compressed = resp
        .into_body()
        .read_to_vec()
        .map_err(|e| format!("read HumanEval: {e}"))?;

    let mut decoder = flate2::read::GzDecoder::new(compressed.as_slice());
    let mut content = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut content)
        .map_err(|e| format!("decompress HumanEval: {e}"))?;

    std::fs::write(&target, &content).map_err(|e| format!("write HumanEval: {e}"))?;
    tracing::info!(
        tasks = content.lines().filter(|l| !l.trim().is_empty()).count(),
        "HumanEval ready"
    );
    Ok(target)
}

/// Ensure the MBPP dataset exists at the expected path.
/// Downloads if missing.
pub(crate) fn ensure_mbpp(data_dir: &Path) -> Result<PathBuf, String> {
    let target = data_dir.join("mbpp.ndjson");
    if target.exists() {
        return Ok(target);
    }
    std::fs::create_dir_all(data_dir).map_err(|e| format!("create dir: {e}"))?;

    tracing::info!("downloading MBPP dataset...");
    let agent = build_agent();
    let resp = agent
        .get(MBPP_URL)
        .call()
        .map_err(|e| format!("download MBPP: {e}"))?;

    let content = resp
        .into_body()
        .read_to_string()
        .map_err(|e| format!("read MBPP: {e}"))?;

    std::fs::write(&target, &content).map_err(|e| format!("write MBPP: {e}"))?;
    tracing::info!(
        tasks = content.lines().filter(|l| !l.trim().is_empty()).count(),
        "MBPP ready"
    );
    Ok(target)
}

fn build_agent() -> ureq::Agent {
    crate::core::http_client::ureq_agent(
        ureq::config::Config::builder()
            .tls_config(crate::core::http_client::platform_tls_config())
            .timeout_global(Some(std::time::Duration::from_mins(1)))
            .build(),
    )
}
