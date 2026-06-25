//! Opt-in OS sandbox for the stdio MCP servers an addon spawns (#865).
//!
//! A stdio addon is a child process with the user's full privileges. When
//! `addons.sandbox` is enabled, lean-ctx wraps that child in an OS-native
//! sandbox launcher before spawning it (the single spawn point is
//! [`crate::core::gateway::client`]):
//!
//! - **macOS** → `sandbox-exec` with a generated SBPL profile,
//! - **Linux** → `bwrap` (bubblewrap) with a read-only root + network unshare.
//!
//! Local stdio tools rarely need the network, so the highest-value, lowest-
//! breakage control is **outbound-network isolation** (`auto`); `strict` also
//! makes the filesystem read-only except a scratch tmp and **refuses to spawn**
//! if no launcher is available (fail-closed). Default is [`SandboxMode::Off`]
//! → zero behavioural change. The argv-building is pure + unit-tested; the
//! enforcement is delegated to the OS launcher.

use std::path::Path;

use super::capabilities::AddonCapabilities;

/// The two enforceable dimensions of an OS sandbox profile. Both the legacy
/// global [`SandboxMode`] and a per-addon [`AddonCapabilities`] declaration are
/// projected onto these, so one set of pure profile builders serves both paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dims {
    /// Allow outbound network when `true`; otherwise the sandbox blocks egress.
    pub network_allowed: bool,
    /// Allow filesystem writes when `true`; otherwise read-only (+ scratch tmp).
    pub fs_writable: bool,
}

impl Dims {
    /// Nothing left to enforce at the OS level (everything is permitted).
    #[must_use]
    fn is_noop(self) -> bool {
        self.network_allowed && self.fs_writable
    }
}

/// Project a legacy [`SandboxMode`] onto sandbox [`Dims`]. `Off` is permissive
/// (callers short-circuit before wrapping); `Auto` blocks network; `Strict`
/// also makes the filesystem read-only.
#[must_use]
fn dims_for_mode(mode: SandboxMode) -> Dims {
    match mode {
        SandboxMode::Off => Dims {
            network_allowed: true,
            fs_writable: true,
        },
        SandboxMode::Auto => Dims {
            network_allowed: false,
            fs_writable: true,
        },
        SandboxMode::Strict => Dims {
            network_allowed: false,
            fs_writable: false,
        },
    }
}

/// Project declared [`AddonCapabilities`] onto sandbox [`Dims`].
#[must_use]
fn dims_for_caps(caps: &AddonCapabilities) -> Dims {
    Dims {
        network_allowed: caps.network_allowed(),
        fs_writable: caps.filesystem_writable(),
    }
}

/// How aggressively to sandbox a spawned stdio server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SandboxMode {
    /// No sandbox — spawn the command directly (default).
    #[default]
    Off,
    /// Best-effort: wrap if a launcher exists, else run directly with a warning.
    /// Blocks outbound network.
    Auto,
    /// Network blocked + read-only filesystem; **refuses** to spawn if no
    /// launcher is available.
    Strict,
}

impl SandboxMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Strict => "strict",
        }
    }

    /// Parse from config text; unknown / empty → [`Self::Off`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "strict" => Self::Strict,
            _ => Self::Off,
        }
    }
}

/// An OS sandbox launcher available on this host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launcher {
    /// macOS `sandbox-exec` (SBPL profile via `-p`).
    SandboxExec,
    /// Linux `bwrap` (bubblewrap).
    Bwrap,
}

/// What to do for a given (mode, launcher) pair — pure, so it is fully tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Spawn the command unchanged.
    Direct,
    /// Wrap the command with `launcher`.
    Wrap(Launcher),
    /// Refuse to spawn (strict mode, no launcher). Carries the reason.
    Refuse(String),
}

/// Decide the plan for `mode` given whether a launcher was detected. Pure.
#[must_use]
pub fn plan(mode: SandboxMode, launcher: Option<Launcher>) -> Plan {
    match (mode, launcher) {
        (SandboxMode::Off, _) | (SandboxMode::Auto, None) => Plan::Direct,
        (_, Some(l)) => Plan::Wrap(l),
        (SandboxMode::Strict, None) => Plan::Refuse(
            "addons.sandbox = strict but no OS sandbox launcher (sandbox-exec / bwrap) is available"
                .to_string(),
        ),
    }
}

/// Detect an available launcher for the current OS, or `None`.
#[must_use]
pub fn detect_launcher() -> Option<Launcher> {
    if cfg!(target_os = "macos") && which("sandbox-exec") {
        Some(Launcher::SandboxExec)
    } else if cfg!(target_os = "linux") && which("bwrap") {
        Some(Launcher::Bwrap)
    } else {
        None
    }
}

/// Build the final `(command, args)` for a [`Plan::Wrap`], prefixing the
/// original invocation with the launcher + a profile derived from `mode`. Pure.
#[must_use]
pub fn wrap_argv(
    launcher: Launcher,
    mode: SandboxMode,
    command: &str,
    args: &[String],
) -> (String, Vec<String>) {
    // The legacy `mode` model has no exec dimension → exec unrestricted.
    wrap_argv_dims(launcher, dims_for_mode(mode), None, command, args)
}

/// Build the final `(command, args)` for a [`Plan::Wrap`] from explicit
/// [`Dims`] plus an optional macOS exec allowlist. `exec_allow`:
/// - `None` → exec unrestricted (no `process-exec` clause),
/// - `Some(paths)` → deny all child exec except these absolute paths (macOS
///   only; ignored by `bwrap`, which cannot path-filter `execve`).
///
/// Pure.
#[must_use]
fn wrap_argv_dims(
    launcher: Launcher,
    dims: Dims,
    exec_allow: Option<&[String]>,
    command: &str,
    args: &[String],
) -> (String, Vec<String>) {
    match launcher {
        Launcher::SandboxExec => {
            let mut profile = sbpl_profile_dims(dims);
            if let Some(allow) = exec_allow {
                profile.push_str(&sbpl_exec_clause(allow));
            }
            let mut v = vec!["-p".to_string(), profile, command.to_string()];
            v.extend(args.iter().cloned());
            ("sandbox-exec".to_string(), v)
        }
        Launcher::Bwrap => {
            // bwrap/seccomp cannot allowlist execve by path; `exec_allow` is
            // handled (disclose / fail-closed) in `apply_caps`, not here.
            let mut v = bwrap_flags_dims(dims);
            v.push(command.to_string());
            v.extend(args.iter().cloned());
            ("bwrap".to_string(), v)
        }
    }
}

/// SBPL clause that blocks all child `process-exec` except the given absolute
/// paths. The addon's own binary must be among `allow_paths`, otherwise
/// `sandbox-exec` cannot even start it (its initial `execvp` is itself a
/// `process-exec`). Last-match-wins, so the allows follow the deny.
fn sbpl_exec_clause(allow_paths: &[String]) -> String {
    let mut s = String::from("(deny process-exec*)\n");
    for p in allow_paths {
        let esc = p.replace('\\', "\\\\").replace('"', "\\\"");
        s.push_str(&format!("(allow process-exec (literal \"{esc}\"))\n"));
    }
    s
}

/// macOS SBPL profile for `mode` (test-only wrapper over [`sbpl_profile_dims`];
/// the runtime path goes through [`wrap_argv`] → [`wrap_argv_dims`]).
#[cfg(test)]
fn sbpl_profile(mode: SandboxMode) -> String {
    sbpl_profile_dims(dims_for_mode(mode))
}

/// macOS SBPL profile for explicit [`Dims`]. `allow default` keeps the tool
/// working; the denies are the security wins. Last-match-wins, so the tmp
/// re-allow follows the deny.
fn sbpl_profile_dims(dims: Dims) -> String {
    let mut p = String::from("(version 1)\n(allow default)\n");
    if !dims.network_allowed {
        p.push_str("(deny network*)\n");
    }
    if !dims.fs_writable {
        p.push_str("(deny file-write*)\n");
        p.push_str("(allow file-write* (subpath \"/tmp\") (subpath \"/private/tmp\") (subpath \"/var/folders\"))\n");
    }
    p
}

/// bubblewrap flags for `mode` (test-only wrapper over [`bwrap_flags_dims`];
/// the runtime path goes through [`wrap_argv`] → [`wrap_argv_dims`]).
#[cfg(test)]
fn bwrap_flags(mode: SandboxMode) -> Vec<String> {
    bwrap_flags_dims(dims_for_mode(mode))
}

/// bubblewrap flags for explicit [`Dims`]: unshare the network unless allowed;
/// bind the root read-only (with a writable tmpfs at `/tmp`) unless writable.
fn bwrap_flags_dims(dims: Dims) -> Vec<String> {
    let mut f: Vec<String> = vec!["--die-with-parent".into()];
    if !dims.network_allowed {
        f.push("--unshare-net".into());
    }
    if dims.fs_writable {
        f.extend(
            ["--bind", "/", "/", "--dev", "/dev", "--proc", "/proc"]
                .iter()
                .map(|s| (*s).to_string()),
        );
    } else {
        f.extend(
            [
                "--ro-bind",
                "/",
                "/",
                "--dev",
                "/dev",
                "--proc",
                "/proc",
                "--tmpfs",
                "/tmp",
            ]
            .iter()
            .map(|s| (*s).to_string()),
        );
    }
    f
}

/// Resolve the configured sandbox mode and rewrite `(command, args)` for the
/// gateway spawn point. Returns the original invocation when sandboxing is off
/// or unavailable in `auto`; an `Err` when `strict` cannot be honoured (the
/// caller must then refuse to spawn). Reads the global-only `[addons]` config.
pub fn apply(command: &str, args: &[String]) -> Result<(String, Vec<String>), String> {
    let mode = crate::core::config::Config::load().addons.sandbox_mode();
    if mode == SandboxMode::Off {
        return Ok((command.to_string(), args.to_vec()));
    }
    match plan(mode, detect_launcher()) {
        Plan::Direct => {
            if mode != SandboxMode::Off {
                tracing::warn!(
                    "addons.sandbox = {} but no OS sandbox launcher is available — \
                     spawning `{command}` UNSANDBOXED",
                    mode.as_str()
                );
            }
            Ok((command.to_string(), args.to_vec()))
        }
        Plan::Wrap(launcher) => {
            tracing::debug!(
                "sandboxing `{command}` via {:?} ({} mode)",
                launcher,
                mode.as_str()
            );
            Ok(wrap_argv(launcher, mode, command, args))
        }
        Plan::Refuse(reason) => Err(reason),
    }
}

/// Resolve the sandbox for a spawn, preferring per-addon declared
/// [`AddonCapabilities`] over the legacy global `addons.sandbox` mode.
///
/// - `Some(caps)` → enforce exactly the declared capabilities (secure-by-default
///   for the platform/marketplace path). If the profile restricts anything but
///   no OS launcher is available, fail closed when `addons.enforce_capabilities`
///   is set, otherwise warn and run unsandboxed.
/// - `None` → fall back to [`apply`] (the legacy `addons.sandbox` behaviour), so
///   addons that predate the capability model keep working unchanged.
pub fn apply_for(
    command: &str,
    args: &[String],
    capabilities: Option<&AddonCapabilities>,
) -> Result<(String, Vec<String>), String> {
    match capabilities {
        Some(caps) => apply_caps(command, args, caps),
        None => apply(command, args),
    }
}

/// Enforce a per-addon capability profile at the spawn point. Pure decision
/// (`plan_caps`) + OS-launcher detection; the wrapping argv is unit-tested.
fn apply_caps(
    command: &str,
    args: &[String],
    caps: &AddonCapabilities,
) -> Result<(String, Vec<String>), String> {
    let dims = dims_for_caps(caps);
    let exec_restricted = caps.exec_restricted();

    // Network + filesystem unrestricted AND exec unrestricted → nothing for the
    // OS sandbox to add (env scrubbing still applies at the spawn point).
    if dims.is_noop() && !exec_restricted {
        return Ok((command.to_string(), args.to_vec()));
    }

    let enforce = crate::core::config::Config::load()
        .addons
        .enforce_capabilities;

    let Some(launcher) = detect_launcher() else {
        // No OS launcher: fail closed only when the org opted in, else warn.
        if enforce {
            return Err(format!(
                "addons.enforce_capabilities = true but no OS sandbox launcher \
                 (sandbox-exec / bwrap) is available to honour `{command}`'s declared \
                 restricted capabilities"
            ));
        }
        tracing::warn!(
            "addon `{command}` declares restricted capabilities but no OS sandbox \
             launcher is available — running UNSANDBOXED (set \
             addons.enforce_capabilities = true to fail closed)"
        );
        return Ok((command.to_string(), args.to_vec()));
    };

    match launcher {
        // macOS: SBPL enforces the exec allowlist precisely (process-exec
        // literals), alongside the network/filesystem profile.
        Launcher::SandboxExec => {
            let exec_allow = if exec_restricted {
                let resolved = resolve_exec_allow(command, caps.exec.allowlist());
                if resolved.is_empty() {
                    // Cannot pin the addon's own binary → emitting the deny would
                    // block it from starting. Fail closed or disclose; never break
                    // the spawn silently.
                    if enforce {
                        return Err(format!(
                            "addons.enforce_capabilities = true: cannot resolve `{command}` \
                             on PATH to pin its exec profile"
                        ));
                    }
                    tracing::warn!(
                        "addon `{command}` declares a restricted exec capability but its \
                         binary could not be resolved on PATH — exec NOT enforced"
                    );
                    None
                } else {
                    Some(resolved)
                }
            } else {
                None
            };
            tracing::debug!(
                "sandboxing `{command}` via sandbox-exec (net={}, fs_write={}, exec_restricted={})",
                dims.network_allowed,
                dims.fs_writable,
                exec_allow.is_some()
            );
            Ok(wrap_argv_dims(
                launcher,
                dims,
                exec_allow.as_deref(),
                command,
                args,
            ))
        }
        // Linux: bwrap/seccomp cannot allowlist `execve` by path and cannot
        // "allow once" for the addon's own start, so a restricted exec
        // declaration is not OS-enforceable. Be honest (no fake enforcement):
        // fail closed under enforce_capabilities, else disclose and still apply
        // the enforceable network/filesystem profile.
        Launcher::Bwrap => {
            if exec_restricted {
                if enforce {
                    return Err(format!(
                        "addons.enforce_capabilities = true: the declared exec restriction for \
                         `{command}` cannot be enforced on Linux (bwrap/seccomp cannot allowlist \
                         execve by path). Run on macOS for full exec enforcement, or set \
                         exec = \"full\"."
                    ));
                }
                tracing::warn!(
                    "addon `{command}` declares a restricted exec capability, but Linux cannot \
                     OS-enforce child-exec gating; running with network/filesystem sandbox only \
                     (exec disclosed, not blocked)"
                );
            }
            if dims.is_noop() {
                // Only exec was restricted, and we cannot enforce it here.
                return Ok((command.to_string(), args.to_vec()));
            }
            tracing::debug!(
                "sandboxing `{command}` via bwrap (net={}, fs_write={})",
                dims.network_allowed,
                dims.fs_writable
            );
            Ok(wrap_argv_dims(launcher, dims, None, command, args))
        }
    }
}

fn which(bin: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let p = dir.join(bin);
        p.is_file() && is_executable(&p)
    })
}

/// Resolve `bin` to the absolute path `execvp` would run, or `None` if it cannot
/// be found / is not executable. Absolute paths are taken as-is; names
/// containing `/` resolve against the CWD; bare names search `PATH`.
#[must_use]
fn which_path(bin: &str) -> Option<String> {
    let p = Path::new(bin);
    if p.is_absolute() {
        return (p.is_file() && is_executable(p)).then(|| bin.to_string());
    }
    if bin.contains('/') {
        let abs = std::env::current_dir().ok()?.join(bin);
        return (abs.is_file() && is_executable(&abs)).then(|| abs.to_string_lossy().into_owned());
    }
    let path = std::env::var("PATH").ok()?;
    std::env::split_paths(&path).find_map(|dir| {
        let cand = dir.join(bin);
        (cand.is_file() && is_executable(&cand)).then(|| cand.to_string_lossy().into_owned())
    })
}

/// Resolve the absolute paths a macOS SBPL profile must allow `process-exec`
/// for: always the addon's own binary (so it can start), plus each declared
/// allowlist entry. Unresolvable entries are dropped (they cannot be exec'd
/// anyway). De-duplicated and sorted for a deterministic profile.
#[must_use]
fn resolve_exec_allow(command: &str, allowlist: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(p) = which_path(command) {
        out.push(p);
    }
    for name in allowlist {
        if let Some(p) = which_path(name) {
            out.push(p);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_p: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_parse_roundtrip() {
        assert_eq!(SandboxMode::parse("auto"), SandboxMode::Auto);
        assert_eq!(SandboxMode::parse("STRICT"), SandboxMode::Strict);
        assert_eq!(SandboxMode::parse(""), SandboxMode::Off);
        assert_eq!(SandboxMode::parse("nonsense"), SandboxMode::Off);
        assert_eq!(SandboxMode::Strict.as_str(), "strict");
    }

    #[test]
    fn plan_off_is_always_direct() {
        assert_eq!(plan(SandboxMode::Off, Some(Launcher::Bwrap)), Plan::Direct);
        assert_eq!(plan(SandboxMode::Off, None), Plan::Direct);
    }

    #[test]
    fn plan_auto_without_launcher_runs_direct() {
        assert_eq!(plan(SandboxMode::Auto, None), Plan::Direct);
    }

    #[test]
    fn plan_strict_without_launcher_refuses() {
        assert!(matches!(plan(SandboxMode::Strict, None), Plan::Refuse(_)));
    }

    #[test]
    fn plan_wraps_when_launcher_present() {
        assert_eq!(
            plan(SandboxMode::Auto, Some(Launcher::SandboxExec)),
            Plan::Wrap(Launcher::SandboxExec)
        );
    }

    #[test]
    fn sandbox_exec_argv_prepends_profile_and_command() {
        let (cmd, args) = wrap_argv(
            Launcher::SandboxExec,
            SandboxMode::Auto,
            "my-mcp",
            &["serve".into()],
        );
        assert_eq!(cmd, "sandbox-exec");
        assert_eq!(args[0], "-p");
        assert!(args[1].contains("(deny network*)"));
        assert_eq!(args[2], "my-mcp");
        assert_eq!(args[3], "serve");
    }

    #[test]
    fn strict_sbpl_restricts_writes() {
        let p = sbpl_profile(SandboxMode::Strict);
        assert!(p.contains("(deny file-write*)"));
        assert!(p.contains("/tmp"));
        let auto = sbpl_profile(SandboxMode::Auto);
        assert!(!auto.contains("(deny file-write*)"));
    }

    #[test]
    fn bwrap_argv_unshares_network() {
        let (cmd, args) = wrap_argv(Launcher::Bwrap, SandboxMode::Auto, "my-mcp", &["x".into()]);
        assert_eq!(cmd, "bwrap");
        assert!(args.iter().any(|a| a == "--unshare-net"));
        assert!(args.iter().any(|a| a == "my-mcp"));
        assert!(args.iter().any(|a| a == "x"));
    }

    #[test]
    fn bwrap_strict_is_readonly_root() {
        let (_c, args) = wrap_argv(Launcher::Bwrap, SandboxMode::Strict, "m", &[]);
        assert!(args.iter().any(|a| a == "--ro-bind"));
        assert!(args.iter().any(|a| a == "--tmpfs"));
    }

    // --- capability-derived profiles (P1) ---

    use super::super::capabilities::{
        AddonCapabilities, ExecAccess, ExecMode, FilesystemAccess, NetworkAccess,
    };

    #[test]
    fn minimal_caps_block_network_and_writes() {
        let dims = dims_for_caps(&AddonCapabilities::default());
        assert!(!dims.network_allowed);
        assert!(!dims.fs_writable);
        let sbpl = sbpl_profile_dims(dims);
        assert!(sbpl.contains("(deny network*)"));
        assert!(sbpl.contains("(deny file-write*)"));
    }

    #[test]
    fn full_network_caps_omit_network_deny() {
        let caps = AddonCapabilities {
            network: NetworkAccess::Full,
            filesystem: FilesystemAccess::ReadOnly,
            env: vec![],
            exec: ExecAccess::default(),
        };
        let dims = dims_for_caps(&caps);
        assert!(dims.network_allowed);
        let sbpl = sbpl_profile_dims(dims);
        assert!(!sbpl.contains("(deny network*)"));
        assert!(sbpl.contains("(deny file-write*)"));
        // bwrap must NOT unshare the network when egress is allowed.
        let flags = bwrap_flags_dims(dims);
        assert!(!flags.iter().any(|f| f == "--unshare-net"));
        assert!(flags.iter().any(|f| f == "--ro-bind"));
    }

    #[test]
    fn fully_permissive_caps_are_a_noop() {
        let caps = AddonCapabilities {
            network: NetworkAccess::Full,
            filesystem: FilesystemAccess::ReadWrite,
            env: vec![],
            // exec must also be unrestricted for the spawn to be a true no-op.
            exec: ExecAccess::Mode(ExecMode::Full),
        };
        assert!(dims_for_caps(&caps).is_noop());
        // apply_for with permissive caps returns the command unchanged.
        let (cmd, args) = apply_for("my-mcp", &["serve".into()], Some(&caps)).expect("noop");
        assert_eq!(cmd, "my-mcp");
        assert_eq!(args, vec!["serve".to_string()]);
    }

    #[test]
    fn caps_wrap_argv_prepends_launcher() {
        let dims = dims_for_caps(&AddonCapabilities::default());
        let (cmd, args) =
            wrap_argv_dims(Launcher::SandboxExec, dims, None, "my-mcp", &["x".into()]);
        assert_eq!(cmd, "sandbox-exec");
        assert_eq!(args[0], "-p");
        assert!(args[1].contains("(deny network*)"));
        assert_eq!(args[2], "my-mcp");
        assert_eq!(args[3], "x");
    }

    // --- exec capability (P1, premium hardening) ---

    #[test]
    fn sbpl_exec_clause_denies_then_allowlists() {
        let clause = sbpl_exec_clause(&["/usr/local/bin/lean-ctx".into(), "/usr/bin/git".into()]);
        assert!(clause.contains("(deny process-exec*)"));
        assert!(clause.contains("(allow process-exec (literal \"/usr/local/bin/lean-ctx\"))"));
        assert!(clause.contains("(allow process-exec (literal \"/usr/bin/git\"))"));
        // deny must precede the allows (last-match-wins).
        let deny_at = clause.find("(deny process-exec*)").unwrap();
        let allow_at = clause.find("(allow process-exec").unwrap();
        assert!(deny_at < allow_at);
    }

    #[test]
    fn exec_allow_is_appended_to_sandbox_profile() {
        let dims = dims_for_caps(&AddonCapabilities::default());
        let allow = vec!["/bin/echo".to_string()];
        let (_cmd, args) = wrap_argv_dims(
            Launcher::SandboxExec,
            dims,
            Some(&allow),
            "/bin/sh",
            &["-c".into()],
        );
        let profile = &args[1];
        assert!(profile.contains("(deny process-exec*)"));
        assert!(profile.contains("(allow process-exec (literal \"/bin/echo\"))"));
    }

    #[test]
    fn exec_full_emits_no_exec_clause() {
        let dims = dims_for_caps(&AddonCapabilities::default());
        let (_cmd, args) = wrap_argv_dims(Launcher::SandboxExec, dims, None, "my-mcp", &[]);
        assert!(!args[1].contains("process-exec"));
    }

    #[test]
    fn resolve_exec_allow_pins_a_real_binary() {
        // /bin/sh exists on every supported unix host; the addon's own binary is
        // always pinned so it can start under the deny-all exec profile.
        let resolved = resolve_exec_allow("/bin/sh", &[]);
        assert_eq!(resolved, vec!["/bin/sh".to_string()]);
        // An unresolvable name is dropped, not fabricated.
        let none = resolve_exec_allow("definitely-not-a-real-binary-xyz", &[]);
        assert!(none.is_empty());
    }

    #[test]
    fn mode_path_unchanged_via_dims() {
        // Back-compat: the mode wrappers still produce the historical profiles.
        assert!(sbpl_profile(SandboxMode::Auto).contains("(deny network*)"));
        assert!(!sbpl_profile(SandboxMode::Auto).contains("(deny file-write*)"));
        assert!(sbpl_profile(SandboxMode::Strict).contains("(deny file-write*)"));
        assert!(
            bwrap_flags(SandboxMode::Auto)
                .iter()
                .any(|f| f == "--unshare-net")
        );
    }
}
