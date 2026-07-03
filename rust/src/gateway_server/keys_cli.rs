//! `lean-ctx gateway keys` (enterprise#48) — per-person key management for
//! `gateway-keys.toml`, replacing the manual `openssl rand | shasum` dance.
//!
//! Storage rule is unchanged (enterprise#11): the file holds **only** SHA-256
//! hashes; the plaintext key is printed exactly once at creation and never
//! touches disk. Writes are atomic (temp file + rename) so a concurrent
//! gateway restart never sees a half-written key set.

use std::path::{Path, PathBuf};

use crate::proxy::gateway_identity::{GatewayKeys, sha256_hex};

/// Prefix of generated keys — recognizable in client configs and log redaction.
const KEY_PREFIX: &str = "gk";

/// Random bytes per generated key (hex-encoded → 48 chars of entropy).
const KEY_RANDOM_BYTES: usize = 24;

/// A parsed identity row for `list` (no hash material beyond a short prefix).
#[derive(Debug, PartialEq, Eq)]
pub struct KeyListEntry {
    pub person: String,
    pub team: Option<String>,
    pub default_project: Option<String>,
    /// First 8 hex chars of the stored hash — enough to correlate with the
    /// file when revoking, useless for authentication.
    pub sha_prefix: String,
}

/// Generates a new bearer key: `gk-<person-slug>-<48 hex chars>`.
///
/// # Errors
/// Fails only if the OS CSPRNG is unavailable.
pub fn generate_key(person: &str) -> anyhow::Result<String> {
    let slug: String = person
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.is_empty() { "key" } else { &slug };
    let mut buf = [0u8; KEY_RANDOM_BYTES];
    getrandom::fill(&mut buf).map_err(|e| anyhow::anyhow!("CSPRNG unavailable: {e}"))?;
    let hex: String = buf.iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    Ok(format!("{KEY_PREFIX}-{slug}-{hex}"))
}

/// Appends a `[[keys]]` entry. Preserves existing content (comments included)
/// by appending; refuses a duplicate person unless `allow_multiple`.
///
/// Returns the plaintext key (print once, never store).
///
/// # Errors
/// Fails on unreadable/unparsable files, duplicate person, or write errors.
pub fn add_key(
    path: &Path,
    person: &str,
    team: Option<&str>,
    default_project: Option<&str>,
    allow_multiple: bool,
) -> anyhow::Result<String> {
    let person = person.trim();
    anyhow::ensure!(!person.is_empty(), "person must not be empty");

    // Validate current file first: never append to a broken key set.
    let existing = GatewayKeys::load(path)
        .map_err(|e| anyhow::anyhow!("existing key file is invalid — fix it first: {e}"))?;
    if !allow_multiple
        && list_keys(path)?
            .iter()
            .any(|k| k.person.eq_ignore_ascii_case(person))
    {
        anyhow::bail!(
            "person '{person}' already has a key (revoke it first, or pass --allow-multiple \
             for an intentional second key)"
        );
    }
    drop(existing);

    let key = generate_key(person)?;
    let sha = sha256_hex(&key);

    let mut body = if path.exists() {
        std::fs::read_to_string(path)?
    } else {
        String::from(
            "# lean-ctx gateway keys — SHA-256 hashes only, plaintext keys are never stored.\n\
             # Managed by `lean-ctx gateway keys`; manual edits are fine (same format).\n",
        )
    };
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    body.push_str("\n[[keys]]\n");
    body.push_str(&format!("sha256_hex = \"{sha}\"\n"));
    body.push_str(&format!("person = \"{}\"\n", toml_escape(person)));
    if let Some(team) = team.map(str::trim).filter(|t| !t.is_empty()) {
        body.push_str(&format!("team = \"{}\"\n", toml_escape(team)));
    }
    if let Some(project) = default_project.map(str::trim).filter(|p| !p.is_empty()) {
        body.push_str(&format!("default_project = \"{}\"\n", toml_escape(project)));
    }

    write_atomic(path, &body)?;
    // Sanity: the file we just wrote must parse and contain the new key.
    let reloaded = GatewayKeys::load(path)?;
    anyhow::ensure!(
        reloaded.lookup(&key).is_some(),
        "post-write validation failed — key not resolvable"
    );
    Ok(key)
}

/// Lists identities (person/team/project + hash prefix), file order.
///
/// # Errors
/// Fails on unreadable or unparsable files.
pub fn list_keys(path: &Path) -> anyhow::Result<Vec<KeyListEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&raw)?;
    let mut out = Vec::new();
    for entry in value
        .get("keys")
        .and_then(|k| k.as_array())
        .unwrap_or(&Vec::new())
    {
        let str_of = |k: &str| {
            entry
                .get(k)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        out.push(KeyListEntry {
            person: str_of("person").unwrap_or_else(|| "?".into()),
            team: str_of("team"),
            default_project: str_of("default_project"),
            sha_prefix: str_of("sha256_hex")
                .map(|s| s.chars().take(8).collect())
                .unwrap_or_default(),
        });
    }
    Ok(out)
}

/// The result of a key rotation: the fresh plaintext key plus the identity it
/// kept and how many old entries it replaced.
#[derive(Debug)]
pub struct RotatedKey {
    pub key: String,
    pub team: Option<String>,
    pub default_project: Option<String>,
    pub replaced: usize,
}

/// Rotates `person`'s key (enterprise#67): mints a fresh key, drops every old
/// entry of that person and writes the replacement **in one atomic swap** —
/// there is no intermediate state where the person has zero valid keys on
/// disk. Team and default project carry over from the person's first entry.
///
/// # Errors
/// Fails when the person has no key (use `add`), on unreadable/unparsable
/// files, or on write errors.
pub fn rotate_key(path: &Path, person: &str) -> anyhow::Result<RotatedKey> {
    let person = person.trim();
    anyhow::ensure!(!person.is_empty(), "person must not be empty");

    let existing = list_keys(path)?;
    let current: Vec<&KeyListEntry> = existing
        .iter()
        .filter(|k| k.person.eq_ignore_ascii_case(person))
        .collect();
    anyhow::ensure!(
        !current.is_empty(),
        "no key for '{person}' in {} — use: lean-ctx gateway keys add --person={person}",
        path.display()
    );
    // Keep the ledger identity exactly as stored — the caller may have typed
    // a different case, but usage_events attribution must not fork.
    let person = current[0].person.clone();
    let person = person.as_str();
    let team = current[0].team.clone();
    let default_project = current[0].default_project.clone();
    let replaced = current.len();

    let key = generate_key(person)?;
    let sha = sha256_hex(&key);

    // Rebuild the file: keep everyone else's entries, replace this person's.
    let raw = std::fs::read_to_string(path)?;
    let mut value: toml::Value = toml::from_str(&raw)?;
    let keys = value
        .get_mut("keys")
        .and_then(|k| k.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("no [[keys]] entries in {}", path.display()))?;
    keys.retain(|entry| {
        entry
            .get("person")
            .and_then(|p| p.as_str())
            .is_none_or(|p| !p.trim().eq_ignore_ascii_case(person))
    });
    let mut fresh = toml::value::Table::new();
    fresh.insert("sha256_hex".into(), toml::Value::String(sha));
    fresh.insert("person".into(), toml::Value::String(person.to_string()));
    if let Some(team) = team.as_deref() {
        fresh.insert("team".into(), toml::Value::String(team.to_string()));
    }
    if let Some(project) = default_project.as_deref() {
        fresh.insert(
            "default_project".into(),
            toml::Value::String(project.to_string()),
        );
    }
    keys.push(toml::Value::Table(fresh));

    let mut body = String::from(
        "# lean-ctx gateway keys — SHA-256 hashes only, plaintext keys are never stored.\n\
         # Managed by `lean-ctx gateway keys`; manual edits are fine (same format).\n",
    );
    body.push_str(&toml::to_string_pretty(&value)?);
    write_atomic(path, &body)?;

    // Post-write validation: the new key must resolve with the old identity.
    let reloaded = GatewayKeys::load(path)?;
    let tags = reloaded
        .lookup(&key)
        .ok_or_else(|| anyhow::anyhow!("post-write validation failed — key not resolvable"))?;
    anyhow::ensure!(
        tags.person.as_deref() == Some(person),
        "post-write validation failed — identity mismatch"
    );

    Ok(RotatedKey {
        key,
        team,
        default_project,
        replaced,
    })
}

/// Removes all keys of `person` (rewrites the file). Returns how many entries
/// were removed.
///
/// # Errors
/// Fails on unreadable/unparsable files or write errors.
pub fn revoke_keys(path: &Path, person: &str) -> anyhow::Result<usize> {
    anyhow::ensure!(path.exists(), "no key file at {}", path.display());
    let raw = std::fs::read_to_string(path)?;
    let mut value: toml::Value = toml::from_str(&raw)?;
    let keys = value
        .get_mut("keys")
        .and_then(|k| k.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("no [[keys]] entries in {}", path.display()))?;
    let before = keys.len();
    keys.retain(|entry| {
        entry
            .get("person")
            .and_then(|p| p.as_str())
            .is_none_or(|p| !p.trim().eq_ignore_ascii_case(person.trim()))
    });
    let removed = before - keys.len();
    if removed > 0 {
        let mut body = String::from(
            "# lean-ctx gateway keys — SHA-256 hashes only, plaintext keys are never stored.\n\
             # Managed by `lean-ctx gateway keys`; manual edits are fine (same format).\n",
        );
        body.push_str(&toml::to_string_pretty(&value)?);
        write_atomic(path, &body)?;
        GatewayKeys::load(path)?; // post-write validation
    }
    Ok(removed)
}

/// Creates a valid, empty key file (deploy mounts require the file to exist).
///
/// # Errors
/// Fails on I/O errors; refuses to touch an existing file.
pub fn write_empty(path: &Path) -> anyhow::Result<()> {
    anyhow::ensure!(!path.exists(), "{} already exists", path.display());
    write_atomic(
        path,
        "# lean-ctx gateway keys — SHA-256 hashes only, plaintext keys are never stored.\n\
         # Add people: lean-ctx gateway keys add --person alice@example.com --file <this file>\n\
         keys = []\n",
    )
}

fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Temp-file + rename in the target directory (same-filesystem atomic swap).
fn write_atomic(path: &Path, contents: &str) -> anyhow::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(dir) = dir {
        std::fs::create_dir_all(dir)?;
    }
    let tmp: PathBuf = path.with_extension("toml.tmp");
    std::fs::write(&tmp, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_unique_and_well_formed() {
        let a = generate_key("Alice Meier").unwrap();
        let b = generate_key("Alice Meier").unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with("gk-alice-meier-"), "got {a}");
        let hex = a.rsplit('-').next().unwrap();
        assert_eq!(hex.len(), KEY_RANDOM_BYTES * 2);
        assert!(hex.bytes().all(|b| b.is_ascii_hexdigit()));
        // Degenerate person names still produce a usable slug.
        assert!(generate_key("!!!").unwrap().starts_with("gk-key-"));
    }

    #[test]
    fn add_list_revoke_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("gateway-keys.toml");

        let key1 = add_key(
            &path,
            "alice@zuehlke.com",
            Some("platform"),
            Some("checkout"),
            false,
        )
        .unwrap();
        let key2 = add_key(&path, "bob@zuehlke.com", None, None, false).unwrap();

        // Plaintext resolves through the real auth loader.
        let keys = GatewayKeys::load(&path).unwrap();
        let alice = keys.lookup(&key1).expect("alice key resolves");
        assert_eq!(alice.person.as_deref(), Some("alice@zuehlke.com"));
        assert_eq!(alice.team.as_deref(), Some("platform"));
        assert_eq!(alice.project.as_deref(), Some("checkout"));
        assert!(keys.lookup(&key2).is_some());

        // list shows identities, not hashes.
        let listed = list_keys(&path).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].person, "alice@zuehlke.com");
        assert_eq!(listed[0].sha_prefix.len(), 8);

        // Duplicate person is refused unless explicitly allowed.
        assert!(add_key(&path, "alice@zuehlke.com", None, None, false).is_err());
        assert!(add_key(&path, "alice@zuehlke.com", None, None, true).is_ok());

        // Revoke removes all of alice's keys, bob survives.
        let removed = revoke_keys(&path, "ALICE@zuehlke.com").unwrap();
        assert_eq!(removed, 2);
        let keys = GatewayKeys::load(&path).unwrap();
        assert!(keys.lookup(&key1).is_none());
        assert!(keys.lookup(&key2).is_some());
    }

    #[test]
    fn rotate_replaces_key_atomically_and_keeps_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("gateway-keys.toml");

        let old_key = add_key(
            &path,
            "alice@zuehlke.com",
            Some("platform"),
            Some("checkout"),
            false,
        )
        .unwrap();
        let bob_key = add_key(&path, "bob@zuehlke.com", None, None, false).unwrap();

        let rotated = rotate_key(&path, "ALICE@zuehlke.com").unwrap();
        assert_eq!(rotated.replaced, 1);
        assert_eq!(rotated.team.as_deref(), Some("platform"));
        assert_eq!(rotated.default_project.as_deref(), Some("checkout"));
        assert_ne!(rotated.key, old_key);

        let keys = GatewayKeys::load(&path).unwrap();
        // Old key is dead, new key carries the identical identity, bob intact.
        assert!(keys.lookup(&old_key).is_none());
        let alice = keys.lookup(&rotated.key).expect("new key resolves");
        assert_eq!(alice.person.as_deref(), Some("alice@zuehlke.com"));
        assert_eq!(alice.team.as_deref(), Some("platform"));
        assert_eq!(alice.project.as_deref(), Some("checkout"));
        assert!(keys.lookup(&bob_key).is_some());

        // Rotating an unknown person is a hard error, not a silent add.
        assert!(rotate_key(&path, "carol@zuehlke.com").is_err());
    }

    #[test]
    fn rotate_collapses_multiple_keys_into_one() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("gateway-keys.toml");
        let k1 = add_key(&path, "alice", Some("platform"), None, false).unwrap();
        let k2 = add_key(&path, "alice", None, None, true).unwrap();

        let rotated = rotate_key(&path, "alice").unwrap();
        assert_eq!(rotated.replaced, 2);
        // Both old keys die; exactly one entry remains for alice.
        let keys = GatewayKeys::load(&path).unwrap();
        assert!(keys.lookup(&k1).is_none());
        assert!(keys.lookup(&k2).is_none());
        assert!(keys.lookup(&rotated.key).is_some());
        let listed = list_keys(&path).unwrap();
        assert_eq!(
            listed.iter().filter(|e| e.person == "alice").count(),
            1,
            "rotation must collapse duplicates"
        );
    }

    #[test]
    fn file_permissions_are_owner_only_on_unix() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let tmp = tempfile::tempdir().unwrap();
            let path = tmp.path().join("gateway-keys.toml");
            add_key(&path, "alice", None, None, false).unwrap();
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "keys file must be owner-only");
        }
    }
}
