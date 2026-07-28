use serde_json::{Map, Value};
use std::collections::HashMap;
use std::time::Instant;

const DEFAULT_MAX_CONVERSATIONS: usize = 100;
const DEFAULT_MAX_TURNS_PER_CONVERSATION: usize = 200;

pub(crate) struct ConversationTracker {
    conversations: HashMap<String, ConversationState>,
    max_conversations: usize,
    max_turns_per_conversation: usize,
    insertion_order: Vec<String>,
}

/// Tracked state for a single ChatGPT conversation.
#[derive(Debug, Clone)]
pub(super) struct ConversationState {
    pub messages: Vec<Value>,
    pub current_node: Option<String>,
    pub last_update: Instant,
}

impl Default for ConversationTracker {
    fn default() -> Self {
        Self::new(
            DEFAULT_MAX_CONVERSATIONS,
            DEFAULT_MAX_TURNS_PER_CONVERSATION,
        )
    }
}

impl ConversationTracker {
    pub(crate) fn new(max_conversations: usize, max_turns_per_conversation: usize) -> Self {
        Self {
            conversations: HashMap::new(),
            max_conversations,
            max_turns_per_conversation,
            insertion_order: Vec::new(),
        }
    }

    pub(crate) fn track_conversation_load(&mut self, conv_id: &str, response_json: &[u8]) {
        let Ok(response) = serde_json::from_slice::<Value>(response_json) else {
            return;
        };
        let Some(mapping) = response.get("mapping").and_then(Value::as_object) else {
            return;
        };

        let current_node = response
            .get("current_node")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let messages = current_node
            .as_deref()
            .map_or_else(Vec::new, |node| build_message_chain(mapping, node));

        if !self.conversations.contains_key(conv_id) {
            self.insertion_order.push(conv_id.to_owned());
        }
        self.conversations.insert(
            conv_id.to_owned(),
            ConversationState {
                messages,
                current_node,
                last_update: Instant::now(),
            },
        );
        self.evict_excess_conversations();
    }

    pub(crate) fn track_sse_response(
        &mut self,
        conv_id: &str,
        message_id: &str,
        role: &str,
        content: &str,
    ) {
        let Some(state) = self.conversations.get_mut(conv_id) else {
            return;
        };

        if state.current_node.as_deref() == Some(message_id)
            && let Some(message) = state.messages.last_mut()
        {
            if let Some(object) = message.as_object_mut() {
                object.insert("role".to_owned(), Value::String(role.to_owned()));
                object.insert("content".to_owned(), Value::String(content.to_owned()));
            }
        } else {
            state.messages.push(serde_json::json!({
                "role": role,
                "content": content,
            }));
        }

        state.current_node = Some(message_id.to_owned());
        state.last_update = Instant::now();
        enforce_turn_limit(state, self.max_turns_per_conversation);
    }

    pub(crate) fn reconstruct_with_new_turn(
        &self,
        conv_id: &str,
        new_message: &Value,
    ) -> Option<Vec<Value>> {
        let state = self.conversations.get(conv_id)?;
        let mut messages = state.messages.clone();
        messages.push(new_message.clone());
        Some(messages)
    }

    pub(crate) fn conversation_count(&self) -> usize {
        self.conversations.len()
    }

    pub(crate) fn has_conversation(&self, conv_id: &str) -> bool {
        self.conversations.contains_key(conv_id)
    }

    fn evict_excess_conversations(&mut self) {
        while self.conversations.len() > self.max_conversations {
            let Some(oldest) = self.insertion_order.first().cloned() else {
                break;
            };
            self.insertion_order.remove(0);
            self.conversations.remove(&oldest);
        }
    }
}

fn enforce_turn_limit(state: &mut ConversationState, max_turns: usize) {
    while state.messages.len() > max_turns {
        let removable = state
            .messages
            .iter()
            .position(|message| message.get("role").and_then(Value::as_str) != Some("system"));
        let Some(index) = removable else {
            break;
        };
        state.messages.remove(index);
    }
}

fn build_message_chain(mapping: &Map<String, Value>, current_node: &str) -> Vec<Value> {
    let mut chain = Vec::new();
    let mut node_id = Some(current_node.to_owned());
    let mut visited = std::collections::HashSet::new();

    while let Some(id) = node_id {
        if !visited.insert(id.clone()) {
            break;
        }
        let Some(node) = mapping.get(&id) else {
            break;
        };
        if let Some(message) = node.get("message").and_then(chatgpt_node_to_canonical) {
            chain.push(message);
        }
        node_id = node
            .get("parent")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }

    chain.reverse();
    chain
}

fn chatgpt_node_to_canonical(message: &Value) -> Option<Value> {
    let role = message.pointer("/author/role")?.as_str()?;
    let content = message.get("content")?;
    if content.get("content_type")?.as_str()? != "text" {
        return None;
    }
    let text = content.pointer("/parts/0")?.as_str()?;
    Some(serde_json::json!({
        "role": role,
        "content": text,
    }))
}

#[cfg(test)]
mod tests {
    use super::ConversationTracker;
    use serde_json::{Value, json};

    fn loaded_conversation() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "mapping": {
                "system-node": {
                    "id": "system-node",
                    "parent": null,
                    "message": {
                        "id": "system-message",
                        "author": { "role": "system" },
                        "content": {
                            "content_type": "text",
                            "parts": ["You are a concise travel assistant."]
                        }
                    }
                },
                "user-node": {
                    "id": "user-node",
                    "parent": "system-node",
                    "message": {
                        "id": "user-message",
                        "author": { "role": "user" },
                        "content": {
                            "content_type": "text",
                            "parts": ["Suggest a weekend in Lausanne."]
                        }
                    }
                },
                "assistant-node": {
                    "id": "assistant-node",
                    "parent": "user-node",
                    "message": {
                        "id": "assistant-message",
                        "author": { "role": "assistant" },
                        "content": {
                            "content_type": "text",
                            "parts": ["Visit Ouchy and the old town."]
                        }
                    }
                }
            },
            "current_node": "assistant-node",
            "title": "Lausanne weekend"
        }))
        .expect("fixture must serialize")
    }

    fn reconstructed(tracker: &ConversationTracker, conv_id: &str) -> Vec<Value> {
        tracker
            .reconstruct_with_new_turn(
                conv_id,
                &json!({
                    "role": "user",
                    "content": "Where should I eat?"
                }),
            )
            .expect("conversation must be tracked")
    }

    #[test]
    fn test_track_load_builds_message_chain() {
        let mut tracker = ConversationTracker::new(100, 200);
        tracker.track_conversation_load("conv-lausanne", &loaded_conversation());

        let messages = reconstructed(&tracker, "conv-lausanne");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["content"], "Suggest a weekend in Lausanne.");
        assert_eq!(messages[2]["role"], "assistant");
    }

    #[test]
    fn test_track_sse_appends_assistant_message() {
        let mut tracker = ConversationTracker::new(100, 200);
        tracker.track_conversation_load("conv-lausanne", &loaded_conversation());
        tracker.track_sse_response(
            "conv-lausanne",
            "next-assistant",
            "assistant",
            "Try Café de Grancy.",
        );

        let messages = reconstructed(&tracker, "conv-lausanne");
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[3]["content"], "Try Café de Grancy.");
    }

    #[test]
    fn test_track_sse_updates_existing_message() {
        let mut tracker = ConversationTracker::new(100, 200);
        tracker.track_conversation_load("conv-lausanne", &loaded_conversation());
        tracker.track_sse_response(
            "conv-lausanne",
            "streamed-assistant",
            "assistant",
            "Try Café",
        );
        tracker.track_sse_response(
            "conv-lausanne",
            "streamed-assistant",
            "assistant",
            "Try Café de Grancy.",
        );

        let messages = reconstructed(&tracker, "conv-lausanne");
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[3]["content"], "Try Café de Grancy.");
    }

    #[test]
    fn test_reconstruct_with_new_turn() {
        let mut tracker = ConversationTracker::new(100, 200);
        tracker.track_conversation_load("conv-lausanne", &loaded_conversation());

        let messages = reconstructed(&tracker, "conv-lausanne");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[3]["role"], "user");
        assert_eq!(messages[3]["content"], "Where should I eat?");
    }

    #[test]
    fn test_reconstruct_unknown_conversation_returns_none() {
        let tracker = ConversationTracker::new(100, 200);
        let result = tracker.reconstruct_with_new_turn(
            "unknown-conversation",
            &json!({ "role": "user", "content": "Hello" }),
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_lru_eviction_when_full() {
        let mut tracker = ConversationTracker::new(2, 200);
        tracker.track_conversation_load("first", &loaded_conversation());
        tracker.track_conversation_load("second", &loaded_conversation());
        tracker.track_conversation_load("third", &loaded_conversation());

        assert_eq!(tracker.conversation_count(), 2);
        assert!(!tracker.has_conversation("first"));
        assert!(tracker.has_conversation("second"));
        assert!(tracker.has_conversation("third"));
    }

    #[test]
    fn test_max_turns_enforcement() {
        let mut tracker = ConversationTracker::new(100, 3);
        tracker.track_conversation_load("conv-lausanne", &loaded_conversation());
        tracker.track_sse_response(
            "conv-lausanne",
            "next-assistant",
            "assistant",
            "Try Café de Grancy.",
        );

        let messages = reconstructed(&tracker, "conv-lausanne");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[2]["content"], "Try Café de Grancy.");
    }

    #[test]
    fn test_empty_mapping_returns_empty_messages() {
        let response = serde_json::to_vec(&json!({
            "mapping": {},
            "current_node": null,
            "title": "New chat"
        }))
        .expect("fixture must serialize");
        let mut tracker = ConversationTracker::new(100, 200);
        tracker.track_conversation_load("empty", &response);

        let messages = reconstructed(&tracker, "empty");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["content"], "Where should I eat?");
    }
}
