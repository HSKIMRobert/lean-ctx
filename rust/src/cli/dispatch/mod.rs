use crate::{
    core, dashboard, doctor, heatmap, hook_handlers, report, setup, shell, status, token_report,
    tools, tui, uninstall,
};

mod help;
mod lifecycle;
mod server;

#[allow(clippy::wildcard_imports)]
use help::*;
#[allow(clippy::wildcard_imports)]
use lifecycle::*;
#[allow(clippy::wildcard_imports)]
use server::*;

pub fn run() {
    let mut args: Vec<String> = std::env::args().collect();

    // On Linux, if the binary was replaced while running, systemd may write
    // the path with " (deleted)" suffix into ExecStart, causing "(deleted)"
    // to appear as an argument. Strip it defensively.
    if args.get(1).is_some_and(|a| a == "(deleted)") {
        args.remove(1);
    }

    let enters_mcp = args.len() == 1 || args.get(1).is_some_and(|a| a == "mcp");
    if !enters_mcp {
        crate::core::logging::init_logging();
    }

    if args.len() > 1 {
        let rest = args[2..].to_vec();

        match args[1].as_str() {
            "-c" | "exec" => {
                let raw = rest.first().is_some_and(|a| a == "--raw");
                let cmd_args = if raw { &args[3..] } else { &args[2..] };
                let command = if cmd_args.len() == 1 {
                    cmd_args[0].clone()
                } else {
                    shell::join_command(cmd_args)
                };
                if std::env::var("LEAN_CTX_ACTIVE").is_ok()
                    || std::env::var("LEAN_CTX_DISABLED").is_ok()
                {
                    passthrough(&command);
                }
                if raw {
                    std::env::set_var("LEAN_CTX_RAW", "1");
                } else {
                    std::env::set_var("LEAN_CTX_COMPRESS", "1");
                }
                let code = shell::exec(&command);
                core::stats::flush();
                core::heatmap::flush();
                std::process::exit(code);
            }
            "-t" | "--track" => {
                let cmd_args = &args[2..];
                let code = if cmd_args.len() > 1 {
                    shell::exec_argv(cmd_args)
                } else {
                    let command = cmd_args[0].clone();
                    if std::env::var("LEAN_CTX_ACTIVE").is_ok()
                        || std::env::var("LEAN_CTX_DISABLED").is_ok()
                    {
                        passthrough(&command);
                    }
                    shell::exec(&command)
                };
                core::stats::flush();
                core::heatmap::flush();
                std::process::exit(code);
            }
            "shell" | "--shell" => {
                shell::interactive();
                return;
            }
            "gain" => {
                if rest.iter().any(|a| a == "--reset") {
                    core::stats::reset_all();
                    println!("Stats reset. All token savings data cleared.");
                    return;
                }
                if rest.iter().any(|a| a == "--live" || a == "--watch") {
                    core::stats::gain_live();
                    return;
                }
                let model = rest.iter().enumerate().find_map(|(i, a)| {
                    if let Some(v) = a.strip_prefix("--model=") {
                        return Some(v.to_string());
                    }
                    if a == "--model" {
                        return rest.get(i + 1).cloned();
                    }
                    None
                });
                let period = rest
                    .iter()
                    .enumerate()
                    .find_map(|(i, a)| {
                        if let Some(v) = a.strip_prefix("--period=") {
                            return Some(v.to_string());
                        }
                        if a == "--period" {
                            return rest.get(i + 1).cloned();
                        }
                        None
                    })
                    .unwrap_or_else(|| "all".to_string());
                let limit = rest
                    .iter()
                    .enumerate()
                    .find_map(|(i, a)| {
                        if let Some(v) = a.strip_prefix("--limit=") {
                            return v.parse::<usize>().ok();
                        }
                        if a == "--limit" {
                            return rest.get(i + 1).and_then(|v| v.parse::<usize>().ok());
                        }
                        None
                    })
                    .unwrap_or(10);

                if rest.iter().any(|a| a == "--graph") {
                    println!("{}", core::stats::format_gain_graph());
                } else if rest.iter().any(|a| a == "--daily") {
                    println!("{}", core::stats::format_gain_daily());
                } else if rest.iter().any(|a| a == "--json") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle(
                            "json",
                            Some(&period),
                            model.as_deref(),
                            Some(limit)
                        )
                    );
                } else if rest.iter().any(|a| a == "--score") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle("score", None, model.as_deref(), Some(limit))
                    );
                } else if rest.iter().any(|a| a == "--cost") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle("cost", None, model.as_deref(), Some(limit))
                    );
                } else if rest.iter().any(|a| a == "--tasks") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle("tasks", None, None, Some(limit))
                    );
                } else if rest.iter().any(|a| a == "--agents") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle("agents", None, None, Some(limit))
                    );
                } else if rest.iter().any(|a| a == "--heatmap") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle("heatmap", None, None, Some(limit))
                    );
                } else if rest.iter().any(|a| a == "--wrapped") {
                    println!(
                        "{}",
                        tools::ctx_gain::handle(
                            "wrapped",
                            Some(&period),
                            model.as_deref(),
                            Some(limit)
                        )
                    );
                } else if rest.iter().any(|a| a == "--pipeline") {
                    let stats_path = dirs::home_dir()
                        .unwrap_or_default()
                        .join(".lean-ctx")
                        .join("pipeline_stats.json");
                    if let Ok(data) = std::fs::read_to_string(&stats_path) {
                        if let Ok(stats) =
                            serde_json::from_str::<core::pipeline::PipelineStats>(&data)
                        {
                            println!("{}", stats.format_summary());
                        } else {
                            println!("No pipeline stats available yet (corrupt data).");
                        }
                    } else {
                        println!(
                            "No pipeline stats available yet. Use MCP tools to generate data."
                        );
                    }
                } else if rest.iter().any(|a| a == "--deep") {
                    println!(
                        "{}\n{}\n{}\n{}\n{}",
                        tools::ctx_gain::handle("report", None, model.as_deref(), Some(limit)),
                        tools::ctx_gain::handle("tasks", None, None, Some(limit)),
                        tools::ctx_gain::handle("cost", None, model.as_deref(), Some(limit)),
                        tools::ctx_gain::handle("agents", None, None, Some(limit)),
                        tools::ctx_gain::handle("heatmap", None, None, Some(limit))
                    );
                } else {
                    println!("{}", core::stats::format_gain());
                }
                return;
            }
            "token-report" | "report-tokens" => {
                let code = token_report::run_cli(&rest);
                if code != 0 {
                    std::process::exit(code);
                }
                return;
            }
            "pack" => {
                crate::cli::cmd_pack(&rest);
                return;
            }
            "proof" => {
                crate::cli::cmd_proof(&rest);
                return;
            }
            "verify" => {
                crate::cli::cmd_verify(&rest);
                return;
            }
            "audit" => {
                println!("{}", crate::cli::audit_report::generate_report());
                return;
            }
            "instructions" => {
                crate::cli::cmd_instructions(&rest);
                return;
            }
            "index" => {
                crate::cli::cmd_index(&rest);
                return;
            }
            "cep" => {
                println!("{}", tools::ctx_gain::handle("score", None, None, Some(10)));
                return;
            }
            "dashboard" => {
                if rest.iter().any(|a| a == "--help" || a == "-h") {
                    println!("Usage: lean-ctx dashboard [--port=N] [--host=H] [--project=PATH]");
                    println!("Examples:");
                    println!("  lean-ctx dashboard");
                    println!("  lean-ctx dashboard --port=3333");
                    println!("  lean-ctx dashboard --host=0.0.0.0");
                    return;
                }
                let port = rest
                    .iter()
                    .find_map(|p| p.strip_prefix("--port=").or_else(|| p.strip_prefix("-p=")))
                    .and_then(|p| p.parse().ok());
                let host = rest
                    .iter()
                    .find_map(|p| p.strip_prefix("--host=").or_else(|| p.strip_prefix("-H=")))
                    .map(String::from);
                let project = rest
                    .iter()
                    .find_map(|p| p.strip_prefix("--project="))
                    .map(String::from);
                if let Some(ref p) = project {
                    std::env::set_var("LEAN_CTX_DASHBOARD_PROJECT", p);
                }
                spawn_proxy_if_needed();
                run_async(dashboard::start(port, host));
                return;
            }
            "team" => {
                let sub = rest.first().map_or("help", std::string::String::as_str);
                match sub {
                    "serve" => {
                        #[cfg(feature = "team-server")]
                        {
                            let cfg_path = rest
                                .iter()
                                .enumerate()
                                .find_map(|(i, a)| {
                                    if let Some(v) = a.strip_prefix("--config=") {
                                        return Some(v.to_string());
                                    }
                                    if a == "--config" {
                                        return rest.get(i + 1).cloned();
                                    }
                                    None
                                })
                                .unwrap_or_default();

                            if cfg_path.trim().is_empty() {
                                eprintln!("Usage: lean-ctx team serve --config <path>");
                                std::process::exit(1);
                            }

                            let cfg = crate::http_server::team::TeamServerConfig::load(
                                std::path::Path::new(&cfg_path),
                            )
                            .unwrap_or_else(|e| {
                                eprintln!("Invalid team config: {e}");
                                std::process::exit(1);
                            });

                            if let Err(e) = run_async(crate::http_server::team::serve_team(cfg)) {
                                tracing::error!("Team server error: {e}");
                                std::process::exit(1);
                            }
                            return;
                        }
                        #[cfg(not(feature = "team-server"))]
                        {
                            eprintln!("lean-ctx team serve is not available in this build");
                            std::process::exit(1);
                        }
                    }
                    "token" => {
                        let action = rest.get(1).map_or("help", std::string::String::as_str);
                        if action == "create" {
                            #[cfg(feature = "team-server")]
                            {
                                let args = &rest[2..];
                                let cfg_path = args
                                    .iter()
                                    .enumerate()
                                    .find_map(|(i, a)| {
                                        if let Some(v) = a.strip_prefix("--config=") {
                                            return Some(v.to_string());
                                        }
                                        if a == "--config" {
                                            return args.get(i + 1).cloned();
                                        }
                                        None
                                    })
                                    .unwrap_or_default();
                                let token_id = args
                                    .iter()
                                    .enumerate()
                                    .find_map(|(i, a)| {
                                        if let Some(v) = a.strip_prefix("--id=") {
                                            return Some(v.to_string());
                                        }
                                        if a == "--id" {
                                            return args.get(i + 1).cloned();
                                        }
                                        None
                                    })
                                    .unwrap_or_default();
                                let scopes_csv = args
                                    .iter()
                                    .enumerate()
                                    .find_map(|(i, a)| {
                                        if let Some(v) = a.strip_prefix("--scopes=") {
                                            return Some(v.to_string());
                                        }
                                        if let Some(v) = a.strip_prefix("--scope=") {
                                            return Some(v.to_string());
                                        }
                                        if a == "--scopes" || a == "--scope" {
                                            return args.get(i + 1).cloned();
                                        }
                                        None
                                    })
                                    .unwrap_or_default();

                                if cfg_path.trim().is_empty()
                                    || token_id.trim().is_empty()
                                    || scopes_csv.trim().is_empty()
                                {
                                    eprintln!(
                                            "Usage: lean-ctx team token create --config <path> --id <id> --scopes <csv>"
                                        );
                                    std::process::exit(1);
                                }

                                let cfg_p = std::path::PathBuf::from(&cfg_path);
                                let mut cfg = crate::http_server::team::TeamServerConfig::load(
                                    cfg_p.as_path(),
                                )
                                .unwrap_or_else(|e| {
                                    eprintln!("Invalid team config: {e}");
                                    std::process::exit(1);
                                });

                                let mut scopes = Vec::new();
                                for part in scopes_csv.split(',') {
                                    let p = part.trim().to_ascii_lowercase();
                                    if p.is_empty() {
                                        continue;
                                    }
                                    let scope = match p.as_str() {
                                        "search" => crate::http_server::team::TeamScope::Search,
                                        "graph" => crate::http_server::team::TeamScope::Graph,
                                        "artifacts" => {
                                            crate::http_server::team::TeamScope::Artifacts
                                        }
                                        "index" => crate::http_server::team::TeamScope::Index,
                                        "events" => crate::http_server::team::TeamScope::Events,
                                        "sessionmutations" | "session_mutations" => {
                                            crate::http_server::team::TeamScope::SessionMutations
                                        }
                                        "knowledge" => {
                                            crate::http_server::team::TeamScope::Knowledge
                                        }
                                        "audit" => crate::http_server::team::TeamScope::Audit,
                                        _ => {
                                            eprintln!("Unknown scope: {p}. Valid: search, graph, artifacts, index, events, sessionmutations, knowledge, audit");
                                            std::process::exit(1);
                                        }
                                    };
                                    if !scopes.contains(&scope) {
                                        scopes.push(scope);
                                    }
                                }
                                if scopes.is_empty() {
                                    eprintln!("At least 1 scope is required");
                                    std::process::exit(1);
                                }

                                let (token, hash) = crate::http_server::team::create_token()
                                    .unwrap_or_else(|e| {
                                        eprintln!("Token generation failed: {e}");
                                        std::process::exit(1);
                                    });

                                cfg.tokens.push(crate::http_server::team::TeamTokenConfig {
                                    id: token_id,
                                    sha256_hex: hash,
                                    scopes,
                                });

                                cfg.save(cfg_p.as_path()).unwrap_or_else(|e| {
                                    eprintln!("Failed to write config: {e}");
                                    std::process::exit(1);
                                });

                                println!("{token}");
                                return;
                            }

                            #[cfg(not(feature = "team-server"))]
                            {
                                eprintln!("lean-ctx team token is not available in this build");
                                std::process::exit(1);
                            }
                        }
                        eprintln!(
                            "Usage: lean-ctx team token create --config <path> --id <id> --scopes <csv>"
                        );
                        std::process::exit(1);
                    }
                    "sync" => {
                        #[cfg(feature = "team-server")]
                        {
                            let args = &rest[1..];
                            let cfg_path = args
                                .iter()
                                .enumerate()
                                .find_map(|(i, a)| {
                                    if let Some(v) = a.strip_prefix("--config=") {
                                        return Some(v.to_string());
                                    }
                                    if a == "--config" {
                                        return args.get(i + 1).cloned();
                                    }
                                    None
                                })
                                .unwrap_or_default();
                            if cfg_path.trim().is_empty() {
                                eprintln!(
                                    "Usage: lean-ctx team sync --config <path> [--workspace <id>]"
                                );
                                std::process::exit(1);
                            }
                            let only_ws = args.iter().enumerate().find_map(|(i, a)| {
                                if let Some(v) = a.strip_prefix("--workspace=") {
                                    return Some(v.to_string());
                                }
                                if let Some(v) = a.strip_prefix("--workspace-id=") {
                                    return Some(v.to_string());
                                }
                                if a == "--workspace" || a == "--workspace-id" {
                                    return args.get(i + 1).cloned();
                                }
                                None
                            });

                            let cfg = crate::http_server::team::TeamServerConfig::load(
                                std::path::Path::new(&cfg_path),
                            )
                            .unwrap_or_else(|e| {
                                eprintln!("Invalid team config: {e}");
                                std::process::exit(1);
                            });

                            for ws in &cfg.workspaces {
                                if let Some(ref only) = only_ws {
                                    if ws.id != *only {
                                        continue;
                                    }
                                }
                                let git_dir = ws.root.join(".git");
                                if !git_dir.exists() {
                                    eprintln!(
                                        "workspace '{}' root is not a git repo: {}",
                                        ws.id,
                                        ws.root.display()
                                    );
                                    std::process::exit(1);
                                }
                                let status = std::process::Command::new("git")
                                    .arg("-C")
                                    .arg(&ws.root)
                                    .args(["fetch", "--all", "--prune"])
                                    .status()
                                    .unwrap_or_else(|e| {
                                        eprintln!(
                                            "git fetch failed for workspace '{}': {e}",
                                            ws.id
                                        );
                                        std::process::exit(1);
                                    });
                                if !status.success() {
                                    eprintln!(
                                        "git fetch failed for workspace '{}' (exit={})",
                                        ws.id,
                                        status.code().unwrap_or(1)
                                    );
                                    std::process::exit(1);
                                }
                            }
                            return;
                        }
                        #[cfg(not(feature = "team-server"))]
                        {
                            eprintln!("lean-ctx team sync is not available in this build");
                            std::process::exit(1);
                        }
                    }
                    _ => {
                        eprintln!(
                            "Usage:\n  lean-ctx team serve --config <path>\n  lean-ctx team token create --config <path> --id <id> --scopes <csv>\n  lean-ctx team sync --config <path> [--workspace <id>]"
                        );
                        std::process::exit(1);
                    }
                }
            }
            "serve" => {
                #[cfg(feature = "http-server")]
                {
                    let mut cfg = crate::http_server::HttpServerConfig::default();
                    let mut daemon_mode = false;
                    let mut stop_mode = false;
                    let mut status_mode = false;
                    let mut foreground_daemon = false;
                    let mut i = 0;
                    while i < rest.len() {
                        match rest[i].as_str() {
                            "--daemon" | "-d" => daemon_mode = true,
                            "--stop" => stop_mode = true,
                            "--status" => status_mode = true,
                            "--_foreground-daemon" => foreground_daemon = true,
                            "--host" | "-H" => {
                                i += 1;
                                if i < rest.len() {
                                    cfg.host.clone_from(&rest[i]);
                                }
                            }
                            arg if arg.starts_with("--host=") => {
                                cfg.host = arg["--host=".len()..].to_string();
                            }
                            "--port" | "-p" => {
                                i += 1;
                                if i < rest.len() {
                                    if let Ok(p) = rest[i].parse::<u16>() {
                                        cfg.port = p;
                                    }
                                }
                            }
                            arg if arg.starts_with("--port=") => {
                                if let Ok(p) = arg["--port=".len()..].parse::<u16>() {
                                    cfg.port = p;
                                }
                            }
                            "--project-root" => {
                                i += 1;
                                if i < rest.len() {
                                    cfg.project_root = std::path::PathBuf::from(&rest[i]);
                                }
                            }
                            arg if arg.starts_with("--project-root=") => {
                                cfg.project_root =
                                    std::path::PathBuf::from(&arg["--project-root=".len()..]);
                            }
                            "--auth-token" => {
                                i += 1;
                                if i < rest.len() {
                                    cfg.auth_token = Some(rest[i].clone());
                                }
                            }
                            arg if arg.starts_with("--auth-token=") => {
                                cfg.auth_token = Some(arg["--auth-token=".len()..].to_string());
                            }
                            "--stateful" => cfg.stateful_mode = true,
                            "--stateless" => cfg.stateful_mode = false,
                            "--json" => cfg.json_response = true,
                            "--sse" => cfg.json_response = false,
                            "--disable-host-check" => cfg.disable_host_check = true,
                            "--allowed-host" => {
                                i += 1;
                                if i < rest.len() {
                                    cfg.allowed_hosts.push(rest[i].clone());
                                }
                            }
                            arg if arg.starts_with("--allowed-host=") => {
                                cfg.allowed_hosts
                                    .push(arg["--allowed-host=".len()..].to_string());
                            }
                            "--max-body-bytes" => {
                                i += 1;
                                if i < rest.len() {
                                    if let Ok(n) = rest[i].parse::<usize>() {
                                        cfg.max_body_bytes = n;
                                    }
                                }
                            }
                            arg if arg.starts_with("--max-body-bytes=") => {
                                if let Ok(n) = arg["--max-body-bytes=".len()..].parse::<usize>() {
                                    cfg.max_body_bytes = n;
                                }
                            }
                            "--max-concurrency" => {
                                i += 1;
                                if i < rest.len() {
                                    if let Ok(n) = rest[i].parse::<usize>() {
                                        cfg.max_concurrency = n;
                                    }
                                }
                            }
                            arg if arg.starts_with("--max-concurrency=") => {
                                if let Ok(n) = arg["--max-concurrency=".len()..].parse::<usize>() {
                                    cfg.max_concurrency = n;
                                }
                            }
                            "--max-rps" => {
                                i += 1;
                                if i < rest.len() {
                                    if let Ok(n) = rest[i].parse::<u32>() {
                                        cfg.max_rps = n;
                                    }
                                }
                            }
                            arg if arg.starts_with("--max-rps=") => {
                                if let Ok(n) = arg["--max-rps=".len()..].parse::<u32>() {
                                    cfg.max_rps = n;
                                }
                            }
                            "--rate-burst" => {
                                i += 1;
                                if i < rest.len() {
                                    if let Ok(n) = rest[i].parse::<u32>() {
                                        cfg.rate_burst = n;
                                    }
                                }
                            }
                            arg if arg.starts_with("--rate-burst=") => {
                                if let Ok(n) = arg["--rate-burst=".len()..].parse::<u32>() {
                                    cfg.rate_burst = n;
                                }
                            }
                            "--request-timeout-ms" => {
                                i += 1;
                                if i < rest.len() {
                                    if let Ok(n) = rest[i].parse::<u64>() {
                                        cfg.request_timeout_ms = n;
                                    }
                                }
                            }
                            arg if arg.starts_with("--request-timeout-ms=") => {
                                if let Ok(n) = arg["--request-timeout-ms=".len()..].parse::<u64>() {
                                    cfg.request_timeout_ms = n;
                                }
                            }
                            "--help" | "-h" => {
                                eprintln!(
                                    "Usage: lean-ctx serve [--host H] [--port N] [--project-root DIR] [--daemon] [--stop] [--status]\\n\\
                                     \\n\\
                                     Options:\\n\\
                                       --daemon, -d          Start as background daemon (UDS)\\n\\
                                       --stop                Stop running daemon\\n\\
                                       --status              Show daemon status\\n\\
                                       --host, -H            Bind host (default: 127.0.0.1)\\n\\
                                       --port, -p            Bind port (default: 8080)\\n\\
                                       --project-root        Resolve relative paths against this root (default: cwd)\\n\\
                                       --auth-token          Require Authorization: Bearer <token> (required for non-loopback binds)\\n\\
                                       --stateful/--stateless  Streamable HTTP session mode (default: stateless)\\n\\
                                       --json/--sse          Response framing in stateless mode (default: json)\\n\\
                                       --max-body-bytes      Max request body size in bytes (default: 2097152)\\n\\
                                       --max-concurrency     Max concurrent requests (default: 32)\\n\\
                                       --max-rps             Max requests/sec (global, default: 50)\\n\\
                                       --rate-burst          Rate limiter burst (global, default: 100)\\n\\
                                       --request-timeout-ms  REST tool-call timeout (default: 30000)\\n\\
                                       --allowed-host        Add allowed Host header (repeatable)\\n\\
                                       --disable-host-check  Disable Host header validation (unsafe)"
                                );
                                return;
                            }
                            _ => {}
                        }
                        i += 1;
                    }

                    if stop_mode {
                        crate::daemon_autostart::stop();
                        if let Err(e) = crate::daemon::stop_daemon() {
                            eprintln!("Error: {e}");
                            std::process::exit(1);
                        }
                        return;
                    }

                    if status_mode {
                        println!("{}", crate::daemon::daemon_status());
                        return;
                    }

                    if daemon_mode {
                        if let Err(e) = crate::daemon::start_daemon(&rest) {
                            eprintln!("Error: {e}");
                            std::process::exit(1);
                        }
                        return;
                    }

                    if foreground_daemon {
                        if let Err(e) = crate::daemon::init_foreground_daemon() {
                            eprintln!("Error writing PID file: {e}");
                            std::process::exit(1);
                        }
                        let addr = crate::daemon::daemon_addr();
                        if let Err(e) = run_async(crate::http_server::serve_ipc(cfg.clone(), addr))
                        {
                            tracing::error!("Daemon server error: {e}");
                            crate::daemon::cleanup_daemon_files();
                            std::process::exit(1);
                        }
                        crate::daemon::cleanup_daemon_files();
                        return;
                    }

                    if cfg.auth_token.is_none() {
                        if let Ok(v) = std::env::var("LEAN_CTX_HTTP_TOKEN") {
                            if !v.trim().is_empty() {
                                cfg.auth_token = Some(v);
                            }
                        }
                    }

                    if let Err(e) = run_async(crate::http_server::serve(cfg)) {
                        tracing::error!("HTTP server error: {e}");
                        std::process::exit(1);
                    }
                    return;
                }
                #[cfg(not(feature = "http-server"))]
                {
                    eprintln!("lean-ctx serve is not available in this build");
                    std::process::exit(1);
                }
            }
            "watch" => {
                if rest.iter().any(|a| a == "--help" || a == "-h") {
                    println!("Usage: lean-ctx watch");
                    println!("  Live TUI dashboard (real-time event stream).");
                    return;
                }
                if let Err(e) = tui::run() {
                    tracing::error!("TUI error: {e}");
                    std::process::exit(1);
                }
                return;
            }
            "proxy" => {
                #[cfg(feature = "http-server")]
                {
                    let sub = rest.first().map_or("help", std::string::String::as_str);
                    match sub {
                        "start" => {
                            let port: u16 = rest
                                .iter()
                                .find_map(|p| {
                                    p.strip_prefix("--port=").or_else(|| p.strip_prefix("-p="))
                                })
                                .and_then(|p| p.parse().ok())
                                .unwrap_or_else(crate::proxy_setup::default_port);
                            let autostart = rest.iter().any(|a| a == "--autostart");
                            if autostart {
                                crate::proxy_autostart::install(port, false);
                                return;
                            }
                            if let Err(e) = run_async(crate::proxy::start_proxy(port)) {
                                tracing::error!("Proxy error: {e}");
                                std::process::exit(1);
                            }
                        }
                        "stop" => {
                            let port: u16 = rest
                                .iter()
                                .find_map(|p| p.strip_prefix("--port="))
                                .and_then(|p| p.parse().ok())
                                .unwrap_or_else(crate::proxy_setup::default_port);
                            let health_url = format!("http://127.0.0.1:{port}/health");
                            match ureq::get(&health_url).call() {
                                Ok(resp) => {
                                    if let Ok(body) = resp.into_body().read_to_string() {
                                        if let Some(pid_str) = body
                                            .split("pid\":")
                                            .nth(1)
                                            .and_then(|s| s.split([',', '}']).next())
                                        {
                                            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                                                let _ =
                                                    crate::ipc::process::terminate_gracefully(pid);
                                                std::thread::sleep(
                                                    std::time::Duration::from_millis(500),
                                                );
                                                if crate::ipc::process::is_alive(pid) {
                                                    let _ = crate::ipc::process::force_kill(pid);
                                                }
                                                println!(
                                                    "Proxy on port {port} stopped (PID {pid})."
                                                );
                                                return;
                                            }
                                        }
                                    }
                                    println!("Proxy on port {port} running but could not parse PID. Use `lean-ctx stop` to kill all.");
                                }
                                Err(_) => {
                                    println!("No proxy running on port {port}.");
                                }
                            }
                        }
                        "status" => {
                            let port: u16 = rest
                                .iter()
                                .find_map(|p| p.strip_prefix("--port="))
                                .and_then(|p| p.parse().ok())
                                .unwrap_or_else(crate::proxy_setup::default_port);
                            let cfg = crate::core::config::Config::load();
                            println!("lean-ctx proxy:");
                            match cfg.proxy_enabled {
                                Some(true) => println!("  Config:  enabled"),
                                Some(false) => println!("  Config:  disabled"),
                                None => println!("  Config:  undecided (not yet configured)"),
                            }
                            println!("  Port:    {port}");
                            if let Ok(resp) =
                                ureq::get(&format!("http://127.0.0.1:{port}/status")).call()
                            {
                                let body = resp.into_body().read_to_string().unwrap_or_default();
                                println!("  Process: running");
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                                    println!("  Requests:    {}", v["requests_total"]);
                                    println!("  Compressed:  {}", v["requests_compressed"]);
                                    println!("  Tokens saved: {}", v["tokens_saved"]);
                                    println!(
                                        "  Compression: {}%",
                                        v["compression_ratio_pct"].as_str().unwrap_or("0.0")
                                    );
                                }
                            } else {
                                println!("  Process: not running");
                            }
                            if cfg.proxy_enabled == Some(false) || cfg.proxy_enabled.is_none() {
                                println!();
                                println!("  Enable: lean-ctx proxy enable");

                                let home = dirs::home_dir().unwrap_or_default();
                                if crate::proxy_setup::has_stale_proxy_url(&home) {
                                    println!();
                                    println!("  \x1b[33m⚠ WARNING: Claude Code ANTHROPIC_BASE_URL points to the local proxy,\x1b[0m");
                                    println!("  \x1b[33m  but proxy is not enabled. This causes 401 auth failures.\x1b[0m");
                                    println!("  Fix:  lean-ctx proxy cleanup   (remove stale URL)");
                                    println!("        lean-ctx proxy enable    (enable the proxy)");
                                }
                            }
                        }
                        "enable" => {
                            let force = rest.iter().any(|a| a == "--force");
                            let mut cfg = crate::core::config::Config::load();
                            cfg.proxy_enabled = Some(true);
                            let _ = cfg.save();

                            let port = crate::proxy_setup::default_port();
                            crate::proxy_autostart::install(port, false);
                            std::thread::sleep(std::time::Duration::from_millis(500));

                            let home = dirs::home_dir().unwrap_or_default();
                            crate::proxy_setup::install_proxy_env_unchecked(
                                &home, port, false, force,
                            );
                            println!("\x1b[32m✓\x1b[0m Proxy enabled on port {port}. LLM requests will be compressed.");
                        }
                        "disable" => {
                            let mut cfg = crate::core::config::Config::load();
                            cfg.proxy_enabled = Some(false);
                            let _ = cfg.save();

                            crate::proxy_autostart::uninstall(false);
                            let home = dirs::home_dir().unwrap_or_default();
                            crate::proxy_setup::uninstall_proxy_env(&home, false);

                            println!(
                                "\x1b[32m✓\x1b[0m Proxy disabled. Original endpoint restored."
                            );
                            println!("  Re-enable anytime: lean-ctx proxy enable");
                        }
                        "cleanup" => {
                            let home = dirs::home_dir().unwrap_or_default();
                            let removed = crate::proxy_setup::cleanup_stale_proxy_env(&home);
                            if removed > 0 {
                                println!(
                                    "\x1b[32m✓\x1b[0m Cleaned up {removed} stale proxy URL(s)."
                                );
                                println!("  Restart your AI tool for changes to take effect.");
                            } else {
                                println!("  No stale proxy URLs found. Nothing to clean up.");
                            }
                        }
                        _ => {
                            println!("Usage: lean-ctx proxy <start|stop|status|enable|disable|cleanup> [--port=4444]");
                        }
                    }
                    return;
                }
                #[cfg(not(feature = "http-server"))]
                {
                    eprintln!("lean-ctx proxy is not available in this build");
                    std::process::exit(1);
                }
            }
            "daemon" => {
                let sub = rest.first().map_or("status", std::string::String::as_str);
                match sub {
                    "enable" => {
                        crate::daemon_autostart::install(false);
                        println!(
                            "\x1b[32m✓\x1b[0m Daemon autostart enabled. Will start on login and restart if stopped."
                        );
                    }
                    "disable" => {
                        crate::daemon_autostart::uninstall(false);
                        println!("\x1b[32m✓\x1b[0m Daemon autostart disabled.");
                    }
                    "start" => {
                        if let Err(e) = crate::daemon::start_daemon(&rest[1..]) {
                            eprintln!("Error: {e}");
                            std::process::exit(1);
                        }
                    }
                    "stop" => {
                        crate::daemon_autostart::stop();
                        match crate::daemon::stop_daemon() {
                            Ok(()) => println!("Daemon stopped."),
                            Err(e) => eprintln!("Error: {e}"),
                        }
                    }
                    "status" => {
                        if crate::daemon::is_daemon_running() {
                            let pid = crate::daemon::read_daemon_pid().unwrap_or(0);
                            println!("lean-ctx daemon:");
                            println!("  Status:    running (PID {pid})");
                            println!(
                                "  Autostart: {}",
                                if crate::daemon_autostart::is_installed() {
                                    "enabled"
                                } else {
                                    "not installed (run: lean-ctx daemon enable)"
                                }
                            );
                        } else {
                            println!("lean-ctx daemon:");
                            println!("  Status:    not running");
                            println!(
                                "  Autostart: {}",
                                if crate::daemon_autostart::is_installed() {
                                    "enabled"
                                } else {
                                    "not installed"
                                }
                            );
                            println!();
                            println!("  Start:     lean-ctx daemon start");
                            println!("  Autostart: lean-ctx daemon enable");
                        }
                    }
                    _ => {
                        println!("Usage: lean-ctx daemon <start|stop|status|enable|disable>");
                    }
                }
                return;
            }
            "init" => {
                super::cmd_init(&rest);
                return;
            }
            "setup" => {
                let non_interactive = rest.iter().any(|a| a == "--non-interactive");
                let yes = rest.iter().any(|a| a == "--yes" || a == "-y");
                let fix = rest.iter().any(|a| a == "--fix");
                let json = rest.iter().any(|a| a == "--json");
                let no_auto_approve = rest.iter().any(|a| a == "--no-auto-approve");

                if non_interactive || fix || json || yes {
                    let opts = setup::SetupOptions {
                        non_interactive,
                        yes,
                        fix,
                        json,
                        no_auto_approve,
                        ..Default::default()
                    };
                    match setup::run_setup_with_options(opts) {
                        Ok(report) => {
                            if json {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&report)
                                        .unwrap_or_else(|_| "{}".to_string())
                                );
                            }
                            if !report.success {
                                std::process::exit(1);
                            }
                        }
                        Err(e) => {
                            eprintln!("{e}");
                            std::process::exit(1);
                        }
                    }
                } else {
                    setup::run_setup();
                }
                return;
            }
            "install" => {
                let repair = rest.iter().any(|a| a == "--repair" || a == "--fix");
                let json = rest.iter().any(|a| a == "--json");
                if !repair {
                    eprintln!("Usage: lean-ctx install --repair [--json]");
                    std::process::exit(1);
                }
                let opts = setup::SetupOptions {
                    non_interactive: true,
                    yes: true,
                    fix: true,
                    json,
                    ..Default::default()
                };
                match setup::run_setup_with_options(opts) {
                    Ok(report) => {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&report)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
                        }
                        if !report.success {
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            "bootstrap" => {
                let json = rest.iter().any(|a| a == "--json");
                let opts = setup::SetupOptions {
                    non_interactive: true,
                    yes: true,
                    fix: true,
                    json,
                    ..Default::default()
                };
                match setup::run_setup_with_options(opts) {
                    Ok(report) => {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&report)
                                    .unwrap_or_else(|_| "{}".to_string())
                            );
                        }
                        if !report.success {
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
                return;
            }
            "status" => {
                let code = status::run_cli(&rest);
                if code != 0 {
                    std::process::exit(code);
                }
                return;
            }
            "read" => {
                super::cmd_read(&rest);
                core::stats::flush();
                return;
            }
            "diff" => {
                super::cmd_diff(&rest);
                core::stats::flush();
                return;
            }
            "grep" => {
                super::cmd_grep(&rest);
                core::stats::flush();
                return;
            }
            "find" => {
                super::cmd_find(&rest);
                core::stats::flush();
                return;
            }
            "ls" => {
                super::cmd_ls(&rest);
                core::stats::flush();
                return;
            }
            "deps" => {
                super::cmd_deps(&rest);
                core::stats::flush();
                return;
            }
            "discover" => {
                super::cmd_discover(&rest);
                return;
            }
            "ghost" => {
                super::cmd_ghost(&rest);
                return;
            }
            "filter" => {
                super::cmd_filter(&rest);
                return;
            }
            "heatmap" => {
                heatmap::cmd_heatmap(&rest);
                return;
            }
            "graph" => {
                let sub = rest.first().map_or("build", std::string::String::as_str);
                match sub {
                    "build" => {
                        let root = rest.get(1).cloned().or_else(|| {
                            std::env::current_dir()
                                .ok()
                                .map(|p| p.to_string_lossy().to_string())
                        });
                        let root = root.unwrap_or_else(|| ".".to_string());
                        let index = core::graph_index::load_or_build(&root);
                        println!(
                            "Graph built: {} files, {} edges",
                            index.files.len(),
                            index.edges.len()
                        );
                    }
                    "export-html" => {
                        let mut root: Option<String> = None;
                        let mut out: Option<String> = None;
                        let mut max_nodes: usize = 2500;

                        let args = &rest[1..];
                        let mut i = 0usize;
                        while i < args.len() {
                            let a = args[i].as_str();
                            if let Some(v) = a.strip_prefix("--root=") {
                                root = Some(v.to_string());
                            } else if a == "--root" {
                                root = args.get(i + 1).cloned();
                                i += 1;
                            } else if let Some(v) = a.strip_prefix("--out=") {
                                out = Some(v.to_string());
                            } else if a == "--out" {
                                out = args.get(i + 1).cloned();
                                i += 1;
                            } else if let Some(v) = a.strip_prefix("--max-nodes=") {
                                max_nodes = v.parse::<usize>().unwrap_or(0);
                            } else if a == "--max-nodes" {
                                let v = args.get(i + 1).map_or("", String::as_str);
                                max_nodes = v.parse::<usize>().unwrap_or(0);
                                i += 1;
                            }
                            i += 1;
                        }

                        let root = root
                            .or_else(|| {
                                std::env::current_dir()
                                    .ok()
                                    .map(|p| p.to_string_lossy().to_string())
                            })
                            .unwrap_or_else(|| ".".to_string());
                        let Some(out) = out else {
                            eprintln!("Usage: lean-ctx graph export-html --out <path> [--root <path>] [--max-nodes <n>]");
                            std::process::exit(1);
                        };
                        if max_nodes == 0 {
                            eprintln!("--max-nodes must be >= 1");
                            std::process::exit(1);
                        }

                        core::graph_export::export_graph_html(
                            &root,
                            std::path::Path::new(&out),
                            max_nodes,
                        )
                        .unwrap_or_else(|e| {
                            eprintln!("graph export failed: {e}");
                            std::process::exit(1);
                        });
                        println!("{out}");
                    }
                    "related" | "impact" | "symbol" | "context" | "status" => {
                        let path_arg = if sub == "status" {
                            None
                        } else {
                            rest.get(1).map(String::as_str)
                        };
                        let root_idx = if sub == "status" { 1 } else { 2 };
                        let root = resolve_graph_root(rest.get(root_idx));
                        println!(
                            "{}",
                            tools::ctx_graph::handle(
                                sub,
                                path_arg,
                                &root,
                                &mut core::cache::SessionCache::new(),
                                tools::CrpMode::Off,
                                None,
                                None,
                            )
                        );
                    }
                    _ => {
                        eprintln!(
                            "Usage:\n  \
                             lean-ctx graph build [path]\n  \
                             lean-ctx graph related <file>\n  \
                             lean-ctx graph impact <file|symbol>\n  \
                             lean-ctx graph symbol <name>\n  \
                             lean-ctx graph context <query>\n  \
                             lean-ctx graph status\n  \
                             lean-ctx graph export-html --out <path> [--root <path>] [--max-nodes <n>]"
                        );
                        std::process::exit(1);
                    }
                }
                return;
            }
            "smells" => {
                let action = rest.first().map_or("summary", String::as_str);
                let rule = rest.iter().enumerate().find_map(|(i, a)| {
                    if let Some(v) = a.strip_prefix("--rule=") {
                        return Some(v.to_string());
                    }
                    if a == "--rule" {
                        return rest.get(i + 1).cloned();
                    }
                    None
                });
                let path = rest.iter().enumerate().find_map(|(i, a)| {
                    if let Some(v) = a.strip_prefix("--path=") {
                        return Some(v.to_string());
                    }
                    if a == "--path" {
                        return rest.get(i + 1).cloned();
                    }
                    None
                });
                let root = rest
                    .iter()
                    .enumerate()
                    .find_map(|(i, a)| {
                        if let Some(v) = a.strip_prefix("--root=") {
                            return Some(v.to_string());
                        }
                        if a == "--root" {
                            return rest.get(i + 1).cloned();
                        }
                        None
                    })
                    .or_else(|| {
                        std::env::current_dir()
                            .ok()
                            .map(|p| p.to_string_lossy().to_string())
                    })
                    .unwrap_or_else(|| ".".to_string());
                let fmt = if rest.iter().any(|a| a == "--json") {
                    Some("json")
                } else {
                    None
                };
                println!(
                    "{}",
                    tools::ctx_smells::handle(action, rule.as_deref(), path.as_deref(), &root, fmt)
                );
                return;
            }
            "session" => {
                super::cmd_session_action(&rest);
                return;
            }
            "ledger" => {
                super::cmd_ledger(&rest);
                return;
            }
            "control" | "context-control" => {
                super::cmd_control(&rest);
                return;
            }
            "plan" | "context-plan" => {
                super::cmd_plan(&rest);
                return;
            }
            "compile" | "context-compile" => {
                super::cmd_compile(&rest);
                return;
            }
            "knowledge" => {
                super::cmd_knowledge(&rest);
                return;
            }
            "overview" => {
                super::cmd_overview(&rest);
                return;
            }
            "compress" => {
                super::cmd_compress(&rest);
                return;
            }
            "wrapped" => {
                super::cmd_wrapped(&rest);
                return;
            }
            "sessions" => {
                super::cmd_sessions(&rest);
                return;
            }
            "benchmark" => {
                super::cmd_benchmark(&rest);
                return;
            }
            "profile" => {
                super::cmd_profile(&rest);
                return;
            }
            "config" => {
                super::cmd_config(&rest);
                return;
            }
            "stats" => {
                super::cmd_stats(&rest);
                return;
            }
            "cache" => {
                super::cmd_cache(&rest);
                return;
            }
            "theme" => {
                super::cmd_theme(&rest);
                return;
            }
            "tee" => {
                super::cmd_tee(&rest);
                return;
            }
            "terse" | "compression" => {
                super::cmd_compression(&rest);
                return;
            }
            "slow-log" => {
                super::cmd_slow_log(&rest);
                return;
            }
            "update" | "--self-update" => {
                core::updater::run(&rest);
                return;
            }
            "restart" => {
                cmd_restart();
                return;
            }
            "stop" => {
                cmd_stop();
                return;
            }
            "dev-install" => {
                cmd_dev_install();
                return;
            }
            "doctor" => {
                let code = doctor::run_cli(&rest);
                if code != 0 {
                    std::process::exit(code);
                }
                return;
            }
            "harden" => {
                super::harden::run(&rest);
                return;
            }
            "export-rules" => {
                super::export_rules::run(&rest);
                return;
            }
            "gotchas" | "bugs" => {
                super::cloud::cmd_gotchas(&rest);
                return;
            }
            "learn" => {
                super::cmd_learn(&rest);
                return;
            }
            "buddy" | "pet" => {
                super::cloud::cmd_buddy(&rest);
                return;
            }
            "hook" => {
                hook_handlers::mark_hook_environment();
                hook_handlers::arm_watchdog(std::time::Duration::from_secs(5));
                let action = rest.first().map_or("help", std::string::String::as_str);
                match action {
                    "rewrite" => hook_handlers::handle_rewrite(),
                    "redirect" => hook_handlers::handle_redirect(),
                    "observe" => hook_handlers::handle_observe(),
                    "copilot" => hook_handlers::handle_copilot(),
                    "codex-pretooluse" => hook_handlers::handle_codex_pretooluse(),
                    "codex-session-start" => hook_handlers::handle_codex_session_start(),
                    "rewrite-inline" => hook_handlers::handle_rewrite_inline(),
                    _ => {
                        eprintln!("Usage: lean-ctx hook <rewrite|redirect|observe|copilot|codex-pretooluse|codex-session-start|rewrite-inline>");
                        eprintln!("  Internal commands used by agent hooks (Claude, Cursor, Copilot, etc.)");
                        std::process::exit(1);
                    }
                }
                return;
            }
            "report-issue" | "report" => {
                report::run(&rest);
                return;
            }
            "uninstall" => {
                let dry_run = rest.iter().any(|a| a == "--dry-run");
                let keep_config = rest.iter().any(|a| a == "--keep-config");
                uninstall::run(dry_run, keep_config);
                return;
            }
            "bypass" => {
                if rest.is_empty() {
                    eprintln!("Usage: lean-ctx bypass \"command\"");
                    eprintln!("Runs the command with zero compression (raw passthrough).");
                    std::process::exit(1);
                }
                let command = if rest.len() == 1 {
                    rest[0].clone()
                } else {
                    shell::join_command(&args[2..])
                };
                std::env::set_var("LEAN_CTX_RAW", "1");
                let code = shell::exec(&command);
                std::process::exit(code);
            }
            "safety-levels" | "safety" => {
                println!("{}", core::compression_safety::format_safety_table());
                return;
            }
            "cheat" | "cheatsheet" | "cheat-sheet" => {
                super::cmd_cheatsheet();
                return;
            }
            "login" => {
                super::cloud::cmd_login(&rest);
                return;
            }
            "register" => {
                super::cloud::cmd_register(&rest);
                return;
            }
            "forgot-password" => {
                super::cloud::cmd_forgot_password(&rest);
                return;
            }
            "sync" => {
                super::cloud::cmd_sync();
                return;
            }
            "contribute" => {
                super::cloud::cmd_contribute();
                return;
            }
            "cloud" => {
                super::cloud::cmd_cloud(&rest);
                return;
            }
            "upgrade" => {
                super::cloud::cmd_upgrade();
                return;
            }
            "--version" | "-V" => {
                println!("{}", core::integrity::origin_line());
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            "mcp" => {}
            _ => {
                tracing::error!("lean-ctx: unknown command '{}'", args[1]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Bare `lean-ctx` in an interactive terminal: a human almost certainly did
    // not mean to start a silent stdio MCP server (which just hangs waiting for
    // JSON-RPC). Show a short quickstart instead. MCP clients pipe stdin (not a
    // TTY) so they still get the server, and explicit `lean-ctx mcp` always
    // serves regardless of TTY.
    if args.len() == 1 && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        print_quickstart();
        return;
    }

    if let Err(e) = run_mcp_server() {
        tracing::error!("lean-ctx: {e}");
        std::process::exit(1);
    }
}

fn resolve_graph_root(arg: Option<&String>) -> String {
    arg.cloned()
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| ".".to_string())
}

fn passthrough(command: &str) -> ! {
    let (shell, flag) = shell::shell_and_flag();
    let mut cmd = std::process::Command::new(&shell);
    cmd.arg(&flag).arg(command).env("LEAN_CTX_ACTIVE", "1");
    shell::platform::apply_utf8_locale(&mut cmd);
    let status = cmd.status().map_or(127, |s| s.code().unwrap_or(1));
    std::process::exit(status);
}

fn run_async<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Runtime::new()
        .expect("failed to create async runtime")
        .block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn quickstart_is_short_and_points_to_setup() {
        let q = quickstart_text();
        assert!(
            q.contains("lean-ctx setup"),
            "quickstart must point to setup"
        );
        assert!(q.contains("--help"), "quickstart must point to full help");
        // Must stay a *quickstart*, not the full reference — keep it tight.
        assert!(
            q.lines().count() <= 16,
            "quickstart should be short; got {} lines",
            q.lines().count()
        );
        assert!(
            !q.contains("COMMANDS:"),
            "quickstart must not inline the full command reference"
        );
    }

    #[test]
    fn capability_banner_tool_count_matches_registry() {
        let n = crate::server::registry::tool_count();
        let banner = capability_banner();
        assert!(
            banner.contains(&format!("{n} MCP tools")),
            "banner must show the live registry count ({n}); got: {banner}"
        );
    }

    #[test]
    #[serial]
    fn worker_threads_default_clamps_low() {
        std::env::remove_var("LEAN_CTX_WORKER_THREADS");
        assert_eq!(resolve_worker_threads(1), 1);
    }

    #[test]
    #[serial]
    fn worker_threads_default_clamps_high() {
        std::env::remove_var("LEAN_CTX_WORKER_THREADS");
        assert_eq!(resolve_worker_threads(32), 4);
    }

    #[test]
    #[serial]
    fn worker_threads_default_passthrough() {
        std::env::remove_var("LEAN_CTX_WORKER_THREADS");
        assert_eq!(resolve_worker_threads(3), 3);
    }

    #[test]
    #[serial]
    fn worker_threads_env_override() {
        std::env::set_var("LEAN_CTX_WORKER_THREADS", "12");
        assert_eq!(resolve_worker_threads(2), 12);
        std::env::remove_var("LEAN_CTX_WORKER_THREADS");
    }

    #[test]
    #[serial]
    fn worker_threads_env_invalid_falls_back() {
        std::env::set_var("LEAN_CTX_WORKER_THREADS", "not_a_number");
        assert_eq!(resolve_worker_threads(3), 3);
        std::env::remove_var("LEAN_CTX_WORKER_THREADS");
    }
}
