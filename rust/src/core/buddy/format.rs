use super::rpg::format_compact;
use super::sprite::sprite_lines_for_tick;
use super::types::BuddyState;

pub fn format_buddy_block(state: &BuddyState, theme: &super::super::theme::Theme) -> String {
    format_buddy_block_at(state, theme, None)
}

pub fn format_buddy_block_at(
    state: &BuddyState,
    theme: &super::super::theme::Theme,
    tick: Option<u64>,
) -> String {
    let r = super::super::theme::rst();
    let a = theme.accent.fg();
    let m = theme.muted.fg();
    let p = theme.primary.fg();
    let rarity_color = state.rarity.color_code();

    let info_lines = [
        format!(
            "{a}{}{r} | {p}{}{r} | {rarity_color}{}{r} | Lv.{}{r}",
            state.name,
            state.species.label(),
            state.rarity.label(),
            state.level,
        ),
        format!(
            "{m}Mood: {} | XP: {}{r}",
            state.mood.label(),
            format_compact(state.xp),
        ),
        format!("{m}\"{}\"{r}", state.speech),
    ];

    let mut lines = Vec::with_capacity(9);
    lines.push(String::new());
    let sprite = sprite_lines_for_tick(state, tick);
    for (i, sprite_line) in sprite.iter().enumerate() {
        let info = if i < info_lines.len() {
            &info_lines[i]
        } else {
            ""
        };
        lines.push(format!("  {p}{sprite_line}{r}  {info}"));
    }
    lines.push(String::new());
    lines.join("\n")
}

pub fn format_buddy_full(state: &BuddyState, theme: &super::super::theme::Theme) -> String {
    let rst = super::super::theme::rst();
    let accent = theme.accent.fg();
    let muted = theme.muted.fg();
    let primary = theme.primary.fg();
    let success = theme.success.fg();
    let warn = theme.warning.fg();
    let bold = super::super::theme::bold();
    let rarity_color = state.rarity.color_code();

    let mut out = Vec::new();

    out.push(String::new());
    out.push(format!("  {bold}{accent}Token Guardian{rst}"));
    out.push(String::new());

    for line in &state.ascii_art {
        out.push(format!("    {primary}{line}{rst}"));
    }
    out.push(String::new());

    out.push(format!(
        "  {bold}{accent}{}{rst}  {muted}the {}{rst}  {rarity_color}{}{rst}  {muted}Lv.{}{rst}",
        state.name,
        state.species.label(),
        state.rarity.label(),
        state.level,
    ));
    out.push(format!(
        "  {muted}Mood: {}  |  XP: {} / {}  |  Streak: {}d{rst}",
        state.mood.label(),
        format_compact(state.xp),
        format_compact(state.xp_next_level),
        state.streak_days,
    ));
    out.push(format!(
        "  {muted}Tokens saved: {}  |  Bugs prevented: {}{rst}",
        format_compact(state.tokens_saved),
        state.bugs_prevented,
    ));
    out.push(String::new());

    out.push(format!("  {bold}Stats{rst}"));
    out.push(format!(
        "  {success}Compression{rst}  {}",
        stat_bar(state.stats.compression, theme)
    ));
    out.push(format!(
        "  {warn}Vigilance  {rst}  {}",
        stat_bar(state.stats.vigilance, theme)
    ));
    out.push(format!(
        "  {primary}Endurance  {rst}  {}",
        stat_bar(state.stats.endurance, theme)
    ));
    out.push(format!(
        "  {accent}Wisdom     {rst}  {}",
        stat_bar(state.stats.wisdom, theme)
    ));
    out.push(format!(
        "  {muted}Experience {rst}  {}",
        stat_bar(state.stats.experience, theme)
    ));
    out.push(String::new());

    out.push(format!("  {muted}\"{}\"{rst}", state.speech));
    out.push(String::new());

    out.join("\n")
}

fn stat_bar(value: u8, theme: &super::super::theme::Theme) -> String {
    let filled = (value as usize) / 5;
    let empty = 20 - filled;
    let r = super::super::theme::rst();
    let g = theme.success.fg();
    let m = theme.muted.fg();
    format!(
        "{g}{}{m}{}{r} {value}/100",
        "█".repeat(filled),
        "░".repeat(empty),
    )
}

pub(super) fn detect_project_root_for_buddy() -> String {
    if let Some(session) = super::super::session::SessionState::load_latest() {
        if let Some(root) = session.project_root.as_deref() {
            if !root.trim().is_empty() {
                return root.to_string();
            }
        }
        if let Some(cwd) = session.shell_cwd.as_deref() {
            if !cwd.trim().is_empty() {
                return super::super::protocol::detect_project_root_or_cwd(cwd);
            }
        }
        if let Some(last) = session.files_touched.last() {
            if !last.path.trim().is_empty() {
                if let Some(parent) = std::path::Path::new(&last.path).parent() {
                    let p = parent.to_string_lossy().to_string();
                    return super::super::protocol::detect_project_root_or_cwd(&p);
                }
            }
        }
    }
    std::env::current_dir()
        .map(|p| super::super::protocol::detect_project_root_or_cwd(&p.to_string_lossy()))
        .unwrap_or_default()
}
