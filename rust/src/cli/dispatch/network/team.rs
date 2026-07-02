//! `lean-ctx team …` — team savings, digest, SLO report.

pub(crate) fn cmd_team(rest: &[String]) {
    let sub = rest.first().map_or("help", std::string::String::as_str);
    match sub {
        "serve" => {
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

            let cfg =
                crate::http_server::team::TeamServerConfig::load(std::path::Path::new(&cfg_path))
                    .unwrap_or_else(|e| {
                        eprintln!("Invalid team config: {e}");
                        std::process::exit(1);
                    });

            if let Err(e) =
                crate::cli::dispatch::run_async(crate::http_server::team::serve_team(cfg))
            {
                tracing::error!("Team server error: {e}");
                std::process::exit(1);
            }
        }
        "token" => {
            let action = rest.get(1).map_or("help", std::string::String::as_str);
            if action == "create" {
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
                let role_arg = args.iter().enumerate().find_map(|(i, a)| {
                    if let Some(v) = a.strip_prefix("--role=") {
                        return Some(v.to_string());
                    }
                    if a == "--role" {
                        return args.get(i + 1).cloned();
                    }
                    None
                });

                // EPIC 13.2: a token may be granted via explicit scopes and/or a
                // coarse role (viewer/member/admin/owner).
                if cfg_path.trim().is_empty()
                    || token_id.trim().is_empty()
                    || (scopes_csv.trim().is_empty() && role_arg.is_none())
                {
                    eprintln!(
                        "Usage: lean-ctx team token create --config <path> --id <id> (--scopes <csv> | --role <viewer|member|admin|owner>)"
                    );
                    std::process::exit(1);
                }

                let role = match role_arg.as_deref() {
                    Some(r) => {
                        let Some(role) = crate::http_server::team::TeamRole::parse(r) else {
                            eprintln!("Unknown role: {r}. Valid: viewer, member, admin, owner");
                            std::process::exit(1);
                        };
                        Some(role)
                    }
                    None => None,
                };

                let cfg_p = std::path::PathBuf::from(&cfg_path);
                let mut cfg = crate::http_server::team::TeamServerConfig::load(cfg_p.as_path())
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
                        "artifacts" => crate::http_server::team::TeamScope::Artifacts,
                        "index" => crate::http_server::team::TeamScope::Index,
                        "events" => crate::http_server::team::TeamScope::Events,
                        "sessionmutations" | "session_mutations" => {
                            crate::http_server::team::TeamScope::SessionMutations
                        }
                        "knowledge" => crate::http_server::team::TeamScope::Knowledge,
                        "audit" => crate::http_server::team::TeamScope::Audit,
                        _ => {
                            eprintln!(
                                "Unknown scope: {p}. Valid: search, graph, artifacts, index, events, sessionmutations, knowledge, audit"
                            );
                            std::process::exit(1);
                        }
                    };
                    if !scopes.contains(&scope) {
                        scopes.push(scope);
                    }
                }
                if scopes.is_empty() && role.is_none() {
                    eprintln!("At least 1 scope or a role is required");
                    std::process::exit(1);
                }

                let (token, hash) = crate::http_server::team::create_token().unwrap_or_else(|e| {
                    eprintln!("Token generation failed: {e}");
                    std::process::exit(1);
                });

                cfg.tokens.push(crate::http_server::team::TeamTokenConfig {
                    id: token_id,
                    sha256_hex: hash,
                    scopes,
                    role,
                });

                cfg.save(cfg_p.as_path()).unwrap_or_else(|e| {
                    eprintln!("Failed to write config: {e}");
                    std::process::exit(1);
                });

                println!("{token}");
                return;
            }
            eprintln!("Usage: lean-ctx team token create --config <path> --id <id> --scopes <csv>");
            std::process::exit(1);
        }
        "slo-report" => {
            cmd_team_slo_report(&rest[1..]);
        }
        "sync" => {
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
                eprintln!("Usage: lean-ctx team sync --config <path> [--workspace <id>]");
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

            let cfg =
                crate::http_server::team::TeamServerConfig::load(std::path::Path::new(&cfg_path))
                    .unwrap_or_else(|e| {
                        eprintln!("Invalid team config: {e}");
                        std::process::exit(1);
                    });

            for ws in &cfg.workspaces {
                if let Some(ref only) = only_ws
                    && ws.id != *only
                {
                    continue;
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
                        eprintln!("git fetch failed for workspace '{}': {e}", ws.id);
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
        }
        _ => {
            eprintln!(
                "Usage:\n  lean-ctx team serve --config <path>\n  lean-ctx team token create --config <path> --id <id> --scopes <csv>\n  lean-ctx team sync --config <path> [--workspace <id>]\n  lean-ctx team slo-report --server <url> --token <token> [--json]"
            );
            std::process::exit(1);
        }
    }
}

/// `lean-ctx team slo-report` — fetches `/v1/metrics` from a team server and
/// renders the hosted-index SLO gate (GL #391). Exit code 0 = all objectives
/// green, 1 = at least one violated (CI-friendly for the 30-day GA gate).
fn cmd_team_slo_report(args: &[String]) {
    let flag = |name: &str| -> Option<String> {
        args.iter().enumerate().find_map(|(i, a)| {
            if let Some(v) = a.strip_prefix(&format!("--{name}=")) {
                return Some(v.to_string());
            }
            if a == format!("--{name}").as_str() {
                return args.get(i + 1).cloned();
            }
            None
        })
    };
    let server = flag("server").unwrap_or_default();
    let token = flag("token")
        .or_else(|| std::env::var("LEAN_CTX_TEAM_TOKEN").ok())
        .unwrap_or_default();
    let json_out = args.iter().any(|a| a == "--json");

    if server.trim().is_empty() || token.trim().is_empty() {
        eprintln!(
            "Usage: lean-ctx team slo-report --server <url> --token <token> [--json]\n  (token also via LEAN_CTX_TEAM_TOKEN)"
        );
        std::process::exit(1);
    }

    let url = format!("{}/v1/metrics", server.trim_end_matches('/'));
    let body = match ureq::get(&url)
        .header("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(resp) => resp.into_body().read_to_string().unwrap_or_default(),
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m Could not reach team server at {url}: {e}");
            std::process::exit(1);
        }
    };
    let v: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("\x1b[31m✗\x1b[0m Invalid /v1/metrics response: {e}");
            std::process::exit(1);
        }
    };
    let Some(slo) = v.get("slo") else {
        eprintln!(
            "\x1b[31m✗\x1b[0m Server response has no `slo` block — server predates GL #391; upgrade the team server."
        );
        std::process::exit(1);
    };

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(slo).unwrap_or_else(|_| slo.to_string())
        );
    }

    let read_f64 = |key: &str| slo.get(key).and_then(serde_json::Value::as_f64);
    let availability = read_f64("availability_pct").unwrap_or(100.0);
    let p95 = read_f64("p95_ms").unwrap_or(0.0);
    let lag = read_f64("index_lag_seconds");
    let window = slo.get("window_len").and_then(serde_json::Value::as_u64);
    let uptime = slo
        .get("uptime_seconds")
        .and_then(serde_json::Value::as_u64);

    // The three GA-gate objectives (docs/examples/team-slos.toml).
    let avail_ok = availability >= 99.5;
    let p95_ok = p95 < 500.0;
    let lag_ok = lag.is_none_or(|secs| secs < 300.0);

    if !json_out {
        let mark = |ok: bool| {
            if ok {
                "\x1b[32mOK\x1b[0m"
            } else {
                "\x1b[31mVIOLATED\x1b[0m"
            }
        };
        println!("Hosted Index SLO Report — {server}");
        println!(
            "  Availability  {availability:7.2} %   (target ≥ 99.5)   {}",
            mark(avail_ok)
        );
        println!(
            "  Query p95     {p95:7.0} ms   (target < 500)    {}",
            mark(p95_ok)
        );
        match lag {
            Some(secs) => println!(
                "  Index lag     {secs:7.0} s    (target < 300)    {}",
                mark(lag_ok)
            ),
            None => println!("  Index lag         n/a    (no index write observed yet)"),
        }
        if let (Some(win), Some(up)) = (window, uptime) {
            let (days, hours, mins) = (up / 86_400, (up % 86_400) / 3_600, (up % 3_600) / 60);
            println!("  Window        {win} requests · uptime {days}d {hours}h {mins}m");
        }
        if avail_ok && p95_ok && lag_ok {
            println!("  \x1b[32m→ GA gate: PASS (all objectives green)\x1b[0m");
        } else {
            println!("  \x1b[31m→ GA gate: FAIL\x1b[0m   Runbook: docs/guides/hosted-index-slo.md");
        }
    }

    if !(avail_ok && p95_ok && lag_ok) {
        std::process::exit(1);
    }
}
