use super::super::{install_mcp_json_agent, mcp_server_quiet_mode, write_file};
use super::shared::prepare_project_rules_path;

pub(crate) fn install_windsurf_rules(global: bool) {
    if global {
        let home = crate::core::home::resolve_home_dir().unwrap_or_default();
        let config_path = home
            .join(".codeium")
            .join("windsurf")
            .join("mcp_config.json");
        install_mcp_json_agent(
            "Windsurf",
            "~/.codeium/windsurf/mcp_config.json",
            &config_path,
        );
    }

    let Some(rules_path) = prepare_project_rules_path(global, ".windsurfrules") else {
        return;
    };

    let rules = include_str!("../../templates/windsurfrules.txt");
    write_file(&rules_path, rules);
    if !mcp_server_quiet_mode() {
        eprintln!("Installed .windsurfrules in current project.");
    }
}
