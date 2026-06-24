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
    match launcher {
        Launcher::SandboxExec => {
            let mut v = vec!["-p".to_string(), sbpl_profile(mode), command.to_string()];
            v.extend(args.iter().cloned());
            ("sandbox-exec".to_string(), v)
        }
        Launcher::Bwrap => {
            let mut v = bwrap_flags(mode);
            v.push(command.to_string());
            v.extend(args.iter().cloned());
            ("bwrap".to_string(), v)
        }
    }
}

/// macOS SBPL profile. `allow default` keeps the tool working; the denies are
/// the security wins. Last-match-wins, so the tmp re-allow follows the deny.
fn sbpl_profile(mode: SandboxMode) -> String {
    let mut p = String::from("(version 1)\n(allow default)\n(deny network*)\n");
    if mode == SandboxMode::Strict {
        p.push_str("(deny file-write*)\n");
        p.push_str("(allow file-write* (subpath \"/tmp\") (subpath \"/private/tmp\") (subpath \"/var/folders\"))\n");
    }
    p
}

/// bubblewrap flags: unshare the network always; in strict, bind the root
/// read-only with a writable tmpfs at `/tmp`.
fn bwrap_flags(mode: SandboxMode) -> Vec<String> {
    let mut f: Vec<String> = vec!["--unshare-net".into(), "--die-with-parent".into()];
    match mode {
        SandboxMode::Strict => {
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
        _ => {
            f.extend(
                ["--bind", "/", "/", "--dev", "/dev", "--proc", "/proc"]
                    .iter()
                    .map(|s| (*s).to_string()),
            );
        }
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

fn which(bin: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let p = dir.join(bin);
        p.is_file() && is_executable(&p)
    })
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
}
