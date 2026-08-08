//! ctx_cognitive — science-driven context intelligence MCP tool.

use rmcp::ErrorData;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::core::config::CognitiveMode;
use crate::server::tool_trait::{McpTool, ToolContext, ToolOutput};
use crate::tool_defs::tool_def;

#[derive(Debug, Deserialize)]
pub struct CognitiveParams {
    pub action: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FeatureStatus {
    name: &'static str,
    enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CognitiveStatus {
    action: &'static str,
    cognitive_mode: String,
    features: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CognitiveFeatures {
    action: &'static str,
    cognitive_mode: String,
    features: Vec<FeatureStatus>,
}

pub struct CtxCognitiveTool;

impl McpTool for CtxCognitiveTool {
    fn name(&self) -> &'static str {
        "ctx_cognitive"
    }

    fn tool_def(&self) -> Tool {
        tool_def(
            "ctx_cognitive",
            "Read science-driven context intelligence status and cognitive impact.\n\
             action=status reports the active mode; impact reports interruption savings; \
             features lists every science feature and whether it is enabled.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "impact", "features"],
                        "description": "Cognitive information to return"
                    }
                },
                "required": ["action"]
            }),
        )
    }

    fn handle(
        &self,
        args: &Map<String, Value>,
        _ctx: &ToolContext,
    ) -> Result<ToolOutput, ErrorData> {
        let params: CognitiveParams = serde_json::from_value(Value::Object(args.clone()))
            .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
        let mode = crate::core::config::Config::load().cognitive_mode;

        let value = match params.action.as_str() {
            "status" => serde_json::to_value(CognitiveStatus {
                action: "status",
                cognitive_mode: mode.to_string(),
                features: feature_statuses(mode)
                    .into_iter()
                    .filter_map(|feature| feature.enabled.then_some(feature.name))
                    .collect(),
            }),
            "impact" => {
                let report = crate::core::anti_interrupt::compute_impact();
                Ok(json!({
                    "action": "impact",
                    "interruptionsPrevented": report.interruptions_prevented,
                    "contextSwitchesSaved": report.context_switches_saved,
                    "echoTokensSaved": report.echo_tokens_saved,
                    "cognitiveLoadReduction": report.cognitive_load_reduction,
                    "focusTimeSavedMinutes": report.focus_time_saved_minutes,
                    "score": report.score
                }))
            }
            "features" => serde_json::to_value(CognitiveFeatures {
                action: "features",
                cognitive_mode: mode.to_string(),
                features: feature_statuses(mode),
            }),
            _ => {
                return Err(ErrorData::invalid_params(
                    "action must be one of: status, impact, features",
                    None,
                ));
            }
        }
        .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;

        let text = serde_json::to_string_pretty(&value)
            .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
        Ok(ToolOutput::simple(text))
    }

    fn produces_machine_readable(&self, _args: Option<&Map<String, Value>>) -> bool {
        true
    }
}

fn feature_statuses(mode: CognitiveMode) -> Vec<FeatureStatus> {
    let basic = !matches!(mode, CognitiveMode::Off);
    let full = matches!(mode, CognitiveMode::Full);

    vec![
        FeatureStatus {
            name: "intent_classification",
            enabled: basic,
        },
        FeatureStatus {
            name: "semantic_chunking",
            enabled: basic,
        },
        FeatureStatus {
            name: "memory_scheduling",
            enabled: full,
        },
        FeatureStatus {
            name: "anti_interruption",
            enabled: full,
        },
        FeatureStatus {
            name: "optimal_transport_allocation",
            enabled: full,
        },
        FeatureStatus {
            name: "graph_expansion",
            enabled: full,
        },
        FeatureStatus {
            name: "structural_descriptions",
            enabled: full,
        },
        FeatureStatus {
            name: "verbosity_learning",
            enabled: full,
        },
        FeatureStatus {
            name: "context_prefetch",
            enabled: full,
        },
        FeatureStatus {
            name: "stigmergic_coordination",
            enabled: full,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_mode_enables_only_basic_features() {
        let features = feature_statuses(CognitiveMode::Basic);
        assert_eq!(features.iter().filter(|feature| feature.enabled).count(), 2);
        assert!(features[0].enabled);
        assert!(features[1].enabled);
    }

    #[test]
    fn full_mode_enables_every_feature() {
        let features = feature_statuses(CognitiveMode::Full);
        assert_eq!(features.len(), 10);
        assert!(features.iter().all(|feature| feature.enabled));
    }
}
