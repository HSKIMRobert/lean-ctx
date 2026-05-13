use super::extract::extract_signatures_ts;
use crate::core::signatures::Signature;

pub(crate) fn extract_sfc_signatures(content: &str) -> Option<Vec<Signature>> {
    let script_content = extract_script_block(content)?;
    let is_ts = content.contains("lang=\"ts\"") || content.contains("lang=\"typescript\"");
    let ext = if is_ts { "ts" } else { "js" };
    extract_signatures_ts(&script_content, ext)
}

pub(crate) fn extract_script_block(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    let start_tag_pos = lower.find("<script")?;
    let tag_end = content[start_tag_pos..].find('>')? + start_tag_pos + 1;
    let end_tag = "</script>";
    let end_pos = lower[tag_end..].find(end_tag)? + tag_end;
    let script = &content[tag_end..end_pos];
    if script.trim().is_empty() {
        return None;
    }
    Some(script.to_string())
}
