//! Live, `#[ignore]`d end-to-end proof for the Pro "Personal Cloud" knowledge
//! vault (GL #787). It drives the *real* engine client (`cloud_client`) against a
//! *real* open backend, exercising the whole zero-knowledge path: seal on device,
//! upload ciphertext, store opaquely, download, open on a "fresh machine".
//!
//! It is `#[ignore]`d because it needs a running backend. The one-shot harness
//! `scripts/cloud_vault_e2e.sh` provisions an ephemeral Postgres + backend, sets
//! the two env vars below, runs this test, and asserts ciphertext-at-rest:
//!
//! - `LEAN_CTX_API_URL` — the backend base URL, e.g. `http://127.0.0.1:PORT`
//! - `LEAN_CTX_DATA_DIR` — an isolated data dir with `cloud/credentials.json` (api_key + user_id)
//!
//! Run the whole thing with: `./scripts/cloud_vault_e2e.sh`
//!
//! Ciphertext-at-rest is asserted out-of-band by the harness (it greps the
//! stored `knowledge_blobs.blob` for the secret needle, which must be absent).

use lean_ctx::cloud_client;
use serde_json::json;

/// A distinctive secret the harness also greps for in the database. Keep it in
/// sync with the orchestration script.
const SECRET_NEEDLE: &str = "E2E-SECRET-NEEDLE-7f3a9c21";

#[test]
#[ignore = "live E2E: needs a running backend (LEAN_CTX_API_URL) + creds (LEAN_CTX_DATA_DIR)"]
fn knowledge_vault_round_trips_through_a_running_backend() {
    let entry = json!({
        "category": "decision",
        "key": "e2e-roundtrip",
        "value": SECRET_NEEDLE,
        "updated_by": "e2e@example.com",
        "updated_at": "2026-01-01T00:00:00Z",
    });

    // Seal on-device + upload. The client derives the vault key from the api_key
    // and POSTs XChaCha20-Poly1305 ciphertext; the server stores it verbatim.
    let resp = cloud_client::push_knowledge(std::slice::from_ref(&entry))
        .expect("push_knowledge must seal + upload against the running backend");
    assert!(
        resp.contains("synced"),
        "vault store should report a successful sync; got: {resp}"
    );

    // Pull on a "fresh machine": download ciphertext + open it locally.
    let pulled =
        cloud_client::pull_knowledge().expect("pull_knowledge must fetch + decrypt the vault");

    let found = pulled.iter().any(|e| {
        e.get("value").and_then(|v| v.as_str()) == Some(SECRET_NEEDLE)
            && e.get("key").and_then(|v| v.as_str()) == Some("e2e-roundtrip")
    });
    assert!(
        found,
        "the round-tripped vault must contain the secret entry; got {pulled:#?}"
    );
}
