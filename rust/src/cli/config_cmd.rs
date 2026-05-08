use crate::core::config;
use crate::core::theme;

pub fn cmd_config(args: &[String]) {
    let cfg = config::Config::load();

    if args.is_empty() {
        println!("{}", cfg.show());
        return;
    }

    match args[0].as_str() {
        "init" | "create" => {
            let default = config::Config::default();
            match default.save() {
                Ok(()) => {
                    let path = config::Config::path().map_or_else(
                        || "~/.lean-ctx/config.toml".to_string(),
                        |p| p.to_string_lossy().to_string(),
                    );
                    println!("Created default config at {path}");
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        "set" => {
            if args.len() < 3 {
                eprintln!("Usage: lean-ctx config set <key> <value>");
                std::process::exit(1);
            }
            let mut cfg = cfg;
            let key = &args[1];
            let val = &args[2];
            match key.as_str() {
                "ultra_compact" => cfg.ultra_compact = val == "true",
                "tee_on_error" | "tee_mode" => {
                    cfg.tee_mode = match val.as_str() {
                        "true" | "failures" => config::TeeMode::Failures,
                        "always" => config::TeeMode::Always,
                        "false" | "never" => config::TeeMode::Never,
                        _ => {
                            eprintln!("Valid tee_mode values: always, failures, never");
                            std::process::exit(1);
                        }
                    };
                }
                "checkpoint_interval" => {
                    cfg.checkpoint_interval = val.parse().unwrap_or(15);
                }
                "theme" => {
                    if theme::from_preset(val).is_some() || val == "custom" {
                        cfg.theme.clone_from(val);
                    } else {
                        eprintln!(
                            "Unknown theme '{val}'. Available: {}",
                            theme::PRESET_NAMES.join(", ")
                        );
                        std::process::exit(1);
                    }
                }
                "slow_command_threshold_ms" => {
                    cfg.slow_command_threshold_ms = val.parse().unwrap_or(5000);
                }
                "passthrough_urls" => {
                    cfg.passthrough_urls = val.split(',').map(|s| s.trim().to_string()).collect();
                }
                "excluded_commands" => {
                    cfg.excluded_commands = val
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "rules_scope" => match val.as_str() {
                    "global" | "project" | "both" => {
                        cfg.rules_scope = Some(val.clone());
                    }
                    _ => {
                        eprintln!("Valid rules_scope values: global, project, both");
                        std::process::exit(1);
                    }
                },
                "proxy.anthropic_upstream" => {
                    cfg.proxy.anthropic_upstream = normalize_optional_upstream(val);
                }
                "proxy.openai_upstream" => {
                    cfg.proxy.openai_upstream = normalize_optional_upstream(val);
                }
                "proxy.gemini_upstream" => {
                    cfg.proxy.gemini_upstream = normalize_optional_upstream(val);
                }
                _ => {
                    eprintln!("Unknown config key: {key}");
                    std::process::exit(1);
                }
            }
            match cfg.save() {
                Ok(()) => println!("Updated {key} = {val}"),
                Err(e) => eprintln!("Error saving config: {e}"),
            }
        }
        _ => {
            eprintln!("Usage: lean-ctx config [init|set <key> <value>]");
            std::process::exit(1);
        }
    }
}

fn normalize_optional_upstream(value: &str) -> Option<String> {
    use crate::core::config::normalize_url_opt;
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        None
    } else {
        normalize_url_opt(trimmed)
    }
}

pub fn cmd_benchmark(args: &[String]) {
    use crate::core::benchmark;

    let action = args.first().map_or("run", std::string::String::as_str);

    match action {
        "--help" | "-h" => {
            println!("Usage: lean-ctx benchmark run [path] [--json]");
            println!("       lean-ctx benchmark report [path]");
        }
        "run" => {
            let path = args.get(1).map_or(".", std::string::String::as_str);
            let is_json = args.iter().any(|a| a == "--json");

            let result = benchmark::run_project_benchmark(path);
            if is_json {
                println!("{}", benchmark::format_json(&result));
            } else {
                println!("{}", benchmark::format_terminal(&result));
            }
        }
        "report" => {
            let path = args.get(1).map_or(".", std::string::String::as_str);
            let result = benchmark::run_project_benchmark(path);
            println!("{}", benchmark::format_markdown(&result));
        }
        _ => {
            if std::path::Path::new(action).exists() {
                let result = benchmark::run_project_benchmark(action);
                println!("{}", benchmark::format_terminal(&result));
            } else {
                eprintln!("Usage: lean-ctx benchmark run [path] [--json]");
                eprintln!("       lean-ctx benchmark report [path]");
                std::process::exit(1);
            }
        }
    }
}

pub fn cmd_stats(args: &[String]) {
    match args.first().map(std::string::String::as_str) {
        Some("reset-cep") => {
            crate::core::stats::reset_cep();
            println!("CEP stats reset. Shell hook data preserved.");
        }
        Some("json") => {
            let store = crate::core::stats::load();
            println!(
                "{}",
                serde_json::to_string_pretty(&store).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            let store = crate::core::stats::load();
            let input_saved = store
                .total_input_tokens
                .saturating_sub(store.total_output_tokens);
            let pct = if store.total_input_tokens > 0 {
                input_saved as f64 / store.total_input_tokens as f64 * 100.0
            } else {
                0.0
            };
            println!("Commands:    {}", store.total_commands);
            println!("Input:       {} tokens", store.total_input_tokens);
            println!("Output:      {} tokens", store.total_output_tokens);
            println!("Saved:       {input_saved} tokens ({pct:.1}%)");
            println!();
            println!("CEP sessions:  {}", store.cep.sessions);
            println!(
                "CEP tokens:    {} → {}",
                store.cep.total_tokens_original, store.cep.total_tokens_compressed
            );
            println!();
            println!("Subcommands: stats reset-cep | stats json");
        }
    }
}

pub fn cmd_cache(args: &[String]) {
    use crate::core::cli_cache;
    match args.first().map(std::string::String::as_str) {
        Some("clear") => {
            let count = cli_cache::clear();
            println!("Cleared {count} cached entries.");
        }
        Some("reset") => {
            let project_flag = args.get(1).map(std::string::String::as_str) == Some("--project");
            if project_flag {
                let root =
                    crate::core::session::SessionState::load_latest().and_then(|s| s.project_root);
                if let Some(root) = root {
                    let count = cli_cache::clear_project(&root);
                    println!("Reset {count} cache entries for project: {root}");
                } else {
                    eprintln!("No active project root found. Start a session first.");
                    std::process::exit(1);
                }
            } else {
                let count = cli_cache::clear();
                println!("Reset all {count} cache entries.");
            }
        }
        Some("stats") => {
            let (hits, reads, entries) = cli_cache::stats();
            let rate = if reads > 0 {
                (hits as f64 / reads as f64 * 100.0).round() as u32
            } else {
                0
            };
            println!("CLI Cache Stats:");
            println!("  Entries:   {entries}");
            println!("  Reads:     {reads}");
            println!("  Hits:      {hits}");
            println!("  Hit Rate:  {rate}%");
        }
        Some("invalidate") => {
            if args.len() < 2 {
                eprintln!("Usage: lean-ctx cache invalidate <path>");
                std::process::exit(1);
            }
            cli_cache::invalidate(&args[1]);
            println!("Invalidated cache for {}", args[1]);
        }
        Some("prune") => {
            let result = prune_bm25_caches();
            println!(
                "Pruned {} entries, freed {:.1} MB",
                result.removed,
                result.bytes_freed as f64 / 1_048_576.0
            );
        }
        _ => {
            let (hits, reads, entries) = cli_cache::stats();
            let rate = if reads > 0 {
                (hits as f64 / reads as f64 * 100.0).round() as u32
            } else {
                0
            };
            println!("CLI File Cache: {entries} entries, {hits}/{reads} hits ({rate}%)");
            println!();
            println!("Subcommands:");
            println!("  cache stats       Show detailed stats");
            println!("  cache clear       Clear all cached entries");
            println!("  cache reset       Reset all cache (or --project for current project only)");
            println!("  cache invalidate  Remove specific file from cache");
            println!(
                "  cache prune       Remove oversized, quarantined, and orphaned BM25 indexes"
            );
        }
    }
}

pub struct PruneResult {
    pub scanned: u32,
    pub removed: u32,
    pub bytes_freed: u64,
}

pub fn prune_bm25_caches() -> PruneResult {
    let mut result = PruneResult {
        scanned: 0,
        removed: 0,
        bytes_freed: 0,
    };

    let Ok(data_dir) = crate::core::data_dir::lean_ctx_data_dir() else {
        return result;
    };
    let vectors_dir = data_dir.join("vectors");
    let Ok(entries) = std::fs::read_dir(&vectors_dir) else {
        return result;
    };

    let max_bytes = crate::core::config::Config::load().bm25_max_cache_mb * 1024 * 1024;

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        result.scanned += 1;

        let quarantined = dir.join("bm25_index.json.quarantined");
        if quarantined.exists() {
            if let Ok(meta) = std::fs::metadata(&quarantined) {
                result.bytes_freed += meta.len();
            }
            let _ = std::fs::remove_file(&quarantined);
            result.removed += 1;
            println!("  Removed quarantined: {}", quarantined.display());
        }

        let index_path = dir.join("bm25_index.json");
        if let Ok(meta) = std::fs::metadata(&index_path) {
            if meta.len() > max_bytes {
                result.bytes_freed += meta.len();
                let _ = std::fs::remove_file(&index_path);
                result.removed += 1;
                println!(
                    "  Removed oversized ({:.1} MB): {}",
                    meta.len() as f64 / 1_048_576.0,
                    index_path.display()
                );
            }
        }

        let marker = dir.join("project_root.txt");
        if let Ok(root_str) = std::fs::read_to_string(&marker) {
            let root_path = std::path::Path::new(root_str.trim());
            if !root_path.exists() {
                let freed = dir_size(&dir);
                result.bytes_freed += freed;
                let _ = std::fs::remove_dir_all(&dir);
                result.removed += 1;
                println!(
                    "  Removed orphaned ({:.1} MB, project gone: {}): {}",
                    freed as f64 / 1_048_576.0,
                    root_str.trim(),
                    dir.display()
                );
            }
        }
    }

    result
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += dir_size(&p);
            }
        }
    }
    total
}
