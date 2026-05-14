use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct ClientMcpCapabilities {
    pub client_id: String,
    pub resources: bool,
    pub prompts: bool,
    pub elicitation: bool,
    pub sampling: bool,
    pub dynamic_tools: bool,
    pub max_tools: Option<usize>,
}

impl Default for ClientMcpCapabilities {
    fn default() -> Self {
        Self {
            client_id: "unknown".to_string(),
            resources: false,
            prompts: false,
            elicitation: false,
            sampling: false,
            dynamic_tools: false,
            max_tools: None,
        }
    }
}

impl ClientMcpCapabilities {
    pub fn detect(client_name: &str) -> Self {
        let lower = client_name.to_lowercase();
        let id = identify_client(&lower);

        match id.as_str() {
            "cursor" | "kiro" => Self {
                client_id: id,
                resources: true,
                prompts: true,
                elicitation: true,
                sampling: false,
                dynamic_tools: true,
                max_tools: None,
            },
            "claude-code" => Self {
                client_id: id,
                resources: true,
                prompts: true,
                elicitation: true,
                sampling: true,
                dynamic_tools: true,
                max_tools: None,
            },
            "windsurf" => Self {
                client_id: id,
                resources: false,
                prompts: false,
                elicitation: false,
                sampling: false,
                dynamic_tools: true,
                max_tools: Some(100),
            },
            "zed" => Self {
                client_id: id,
                resources: false,
                prompts: true,
                elicitation: false,
                sampling: false,
                dynamic_tools: true,
                max_tools: None,
            },
            "vscode-copilot" => Self {
                client_id: id,
                resources: true,
                prompts: true,
                elicitation: false,
                sampling: false,
                dynamic_tools: true,
                max_tools: None,
            },
            "codex" => Self {
                client_id: id,
                resources: true,
                prompts: false,
                elicitation: false,
                sampling: false,
                dynamic_tools: true,
                max_tools: None,
            },
            "antigravity" | "gemini-cli" => Self {
                client_id: id,
                resources: false,
                prompts: false,
                elicitation: false,
                sampling: false,
                dynamic_tools: false,
                max_tools: None,
            },
            _ => Self {
                client_id: id,
                ..Default::default()
            },
        }
    }

    pub fn tier(&self) -> u8 {
        let score = [
            self.resources,
            self.prompts,
            self.elicitation,
            self.sampling,
            self.dynamic_tools,
        ]
        .iter()
        .filter(|&&v| v)
        .count();

        match score {
            4..=5 => 1,
            2..=3 => 2,
            1 => 3,
            _ => 4,
        }
    }

    pub fn format_summary(&self) -> String {
        let features: Vec<&str> = [
            ("resources", self.resources),
            ("prompts", self.prompts),
            ("elicitation", self.elicitation),
            ("sampling", self.sampling),
            ("dynamic_tools", self.dynamic_tools),
        ]
        .iter()
        .filter(|(_, v)| *v)
        .map(|(k, _)| *k)
        .collect();

        let tools_note = self
            .max_tools
            .map(|n| format!(" (max {n} tools)"))
            .unwrap_or_default();

        format!(
            "{} (tier {}): [{}]{}",
            self.client_id,
            self.tier(),
            features.join(", "),
            tools_note,
        )
    }
}

fn identify_client(lower: &str) -> String {
    if lower.contains("cursor") {
        "cursor".to_string()
    } else if lower.contains("claude") {
        "claude-code".to_string()
    } else if lower.contains("windsurf") || lower.contains("codeium") {
        "windsurf".to_string()
    } else if lower.contains("zed") {
        "zed".to_string()
    } else if lower.contains("copilot") || lower.contains("github") {
        "vscode-copilot".to_string()
    } else if lower.contains("kiro") {
        "kiro".to_string()
    } else if lower.contains("codex") || lower.contains("openai") {
        "codex".to_string()
    } else if lower.contains("antigravity") {
        "antigravity".to_string()
    } else if lower.contains("gemini") {
        "gemini-cli".to_string()
    } else {
        "unknown".to_string()
    }
}

static GLOBAL: OnceLock<Mutex<ClientMcpCapabilities>> = OnceLock::new();

pub fn global() -> &'static Mutex<ClientMcpCapabilities> {
    GLOBAL.get_or_init(|| Mutex::new(ClientMcpCapabilities::default()))
}

pub fn set_detected(caps: ClientMcpCapabilities) {
    if let Ok(mut g) = global().lock() {
        *g = caps;
    }
}

pub fn current() -> ClientMcpCapabilities {
    global().lock().map(|g| g.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_detection() {
        let caps = ClientMcpCapabilities::detect("Cursor");
        assert_eq!(caps.client_id, "cursor");
        assert!(caps.resources);
        assert!(caps.prompts);
        assert!(caps.elicitation);
        assert!(caps.dynamic_tools);
        assert_eq!(caps.tier(), 1);
    }

    #[test]
    fn claude_code_detection() {
        let caps = ClientMcpCapabilities::detect("claude-code");
        assert_eq!(caps.client_id, "claude-code");
        assert!(caps.sampling);
        assert_eq!(caps.tier(), 1);
    }

    #[test]
    fn windsurf_detection() {
        let caps = ClientMcpCapabilities::detect("Windsurf");
        assert_eq!(caps.client_id, "windsurf");
        assert!(!caps.resources);
        assert!(!caps.prompts);
        assert_eq!(caps.max_tools, Some(100));
        assert_eq!(caps.tier(), 3);
    }

    #[test]
    fn unknown_client_tier4() {
        let caps = ClientMcpCapabilities::detect("random-editor");
        assert_eq!(caps.client_id, "unknown");
        assert_eq!(caps.tier(), 4);
    }

    #[test]
    fn format_summary() {
        let caps = ClientMcpCapabilities::detect("Cursor");
        let s = caps.format_summary();
        assert!(s.contains("cursor"));
        assert!(s.contains("tier 1"));
    }
}
