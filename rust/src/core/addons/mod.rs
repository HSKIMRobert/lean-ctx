//! Addon ecosystem: community extensions for lean-ctx (#858).
//!
//! An **addon** packages an external MCP server (+ metadata) behind a small
//! [`lean-ctx-addon.toml`](manifest) manifest, so a third-party tool plugs into
//! lean-ctx's MCP gateway with a single `lean-ctx addon add` — no fork, no
//! recompile. Addons are user-global and reuse the gateway trust model
//! (`[gateway]` is global-only and opt-in; see [`crate::core::gateway`]).
//!
//! Layers:
//! - [`manifest`] — the `lean-ctx-addon.toml` contract (also the registry entry shape).
//! - [`registry`] — the curated catalog (bundled, with optional user override).
//! - [`store`]    — what is installed locally (`<data_dir>/addons/installed.json`).
//! - [`install`]  — wires an addon into the gateway and records it in the store.
//!
//! Security (#863):
//! - [`trust`]    — trust tier (`verified`) + static risk assessment of the wiring.
//! - [`policy`]   — the global-only `[addons]` install policy floor + the gate.
//! - [`signing`]  — Ed25519 signing for the user-override registry.
//! - [`sandbox`]  — opt-in OS sandbox for spawned stdio servers.
//! - [`runtime`]  — redaction + audit of untrusted addon tool output.

pub mod install;
pub mod manifest;
pub mod policy;
pub mod registry;
pub mod runtime;
pub mod sandbox;
pub mod signing;
pub mod store;
pub mod trust;

pub use manifest::{AddonManifest, AddonMcp, AddonMeta};
pub use policy::{AddonPolicy, AddonsConfig};
pub use sandbox::SandboxMode;
pub use store::{InstalledAddon, InstalledStore};
pub use trust::{RiskFinding, RiskLevel, TrustTier};
