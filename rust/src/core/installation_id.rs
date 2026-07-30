//! Persistent anonymous installation identifier for opt-in telemetry.
//!
//! Generates a random UUID v4 on first call and persists it as a plain-text
//! file in the data directory. The ID is never derived from hardware, OS
//! fingerprints, or user identity — it is pure randomness.

use std::path::PathBuf;

fn id_path() -> Result<PathBuf, String> {
    crate::core::paths::data_dir().map(|d| d.join("installation_id"))
}

/// Return the existing installation ID or generate a fresh UUID v4.
pub(crate) fn get_or_create() -> Result<String, String> {
    let path = id_path()?;
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim().to_string();
        if is_valid_uuid(&trimmed) {
            return Ok(trimmed);
        }
    }
    let id = generate_uuid_v4();
    persist(&path, &id)?;
    Ok(id)
}

/// Regenerate the installation ID (for `lean-ctx telemetry reset-id`).
pub(crate) fn reset() -> Result<String, String> {
    let path = id_path()?;
    let id = generate_uuid_v4();
    persist(&path, &id)?;
    Ok(id)
}

fn persist(path: &std::path::Path, id: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create data dir: {e}"))?;
    }
    std::fs::write(path, format!("{id}\n"))
        .map_err(|e| format!("Cannot write installation ID: {e}"))
}

fn generate_uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("getrandom failed");
    // RFC 4122 variant and version bits
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_be_bytes([bytes[4], bytes[5]]),
        u16::from_be_bytes([bytes[6], bytes[7]]),
        u16::from_be_bytes([bytes[8], bytes[9]]),
        u64::from_be_bytes([
            0, 0, bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        ]),
    )
}

fn is_valid_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && s.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

/// Return the masked form `abcd…wxyz` for display.
pub(crate) fn masked(id: &str) -> String {
    if id.len() < 8 {
        return "********".to_string();
    }
    let clean: String = id.chars().filter(|c| *c != '-').collect();
    format!("{}…{}", &clean[..4], &clean[clean.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_uuid_is_valid() {
        let id = generate_uuid_v4();
        assert!(is_valid_uuid(&id), "invalid UUID: {id}");
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4", "version nibble must be 4");
        let variant_nibble = u8::from_str_radix(&id[19..20], 16).unwrap();
        assert!(
            (0x8..=0xb).contains(&variant_nibble),
            "variant must be 8-b, got {variant_nibble:x}"
        );
    }

    #[test]
    fn get_or_create_roundtrip() {
        let _iso = crate::core::data_dir::isolated_data_dir();
        let a = get_or_create().unwrap();
        let b = get_or_create().unwrap();
        assert_eq!(a, b, "must return the same ID on repeated calls");
    }

    #[test]
    fn reset_changes_id() {
        let _iso = crate::core::data_dir::isolated_data_dir();
        let a = get_or_create().unwrap();
        let b = reset().unwrap();
        assert_ne!(a, b, "reset must produce a new ID");
        let c = get_or_create().unwrap();
        assert_eq!(b, c, "get_or_create after reset must return the new ID");
    }

    #[test]
    fn masked_format() {
        let id = "abcdef01-2345-4678-9abc-def012345678";
        assert_eq!(masked(id), "abcd…5678");
    }
}
