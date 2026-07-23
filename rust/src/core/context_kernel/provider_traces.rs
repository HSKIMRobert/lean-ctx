#[cfg(test)]
mod tests {
    use super::super::provider_parity::{detect_provider, envelope_from_usage};
    use super::super::token_envelope::ProviderKind;
    use serde_json::json;

    #[test]
    fn openai_standard() {
        let provider = detect_provider("https://api.openai.com/v1/chat/completions");
        assert_eq!(provider, ProviderKind::OpenAi);

        let envelope = envelope_from_usage(
            ProviderKind::OpenAi,
            "gpt-4",
            &json!({"prompt_tokens": 150, "completion_tokens": 80}),
        );
        assert_eq!(envelope.input_tokens, 150);
        assert_eq!(envelope.output_tokens, 80);
    }

    #[test]
    fn openai_with_cache() {
        let envelope = envelope_from_usage(
            ProviderKind::OpenAi,
            "gpt-4",
            &json!({
                "prompt_tokens": 200,
                "completion_tokens": 50,
                "prompt_tokens_details": {"cached_tokens": 100}
            }),
        );

        assert_eq!(envelope.cache_read_tokens, 100);
    }

    #[test]
    fn openai_with_reasoning() {
        let envelope = envelope_from_usage(
            ProviderKind::OpenAi,
            "gpt-4",
            &json!({
                "prompt_tokens": 100,
                "completion_tokens": 500,
                "completion_tokens_details": {"reasoning_tokens": 400}
            }),
        );

        assert_eq!(envelope.reasoning_tokens, 400);
    }

    #[test]
    fn anthropic_standard() {
        let provider = detect_provider("https://api.anthropic.com/v1/messages");
        assert_eq!(provider, ProviderKind::Anthropic);

        let envelope = envelope_from_usage(
            ProviderKind::Anthropic,
            "claude",
            &json!({"input_tokens": 120, "output_tokens": 60}),
        );
        assert_eq!(envelope.input_tokens, 120);
        assert_eq!(envelope.output_tokens, 60);
    }

    #[test]
    fn anthropic_with_cache() {
        let envelope = envelope_from_usage(
            ProviderKind::Anthropic,
            "claude",
            &json!({
                "input_tokens": 200,
                "output_tokens": 50,
                "cache_read_input_tokens": 80,
                "cache_creation_input_tokens": 30
            }),
        );

        assert_eq!(envelope.cache_read_tokens, 80);
        assert_eq!(envelope.cache_write_tokens, 30);
    }

    #[test]
    fn gemini_usage() {
        let provider = detect_provider("https://generativelanguage.googleapis.com");
        assert_eq!(provider, ProviderKind::Gemini);

        let envelope = envelope_from_usage(
            ProviderKind::Gemini,
            "gemini",
            &json!({
                "promptTokenCount": 100,
                "candidatesTokenCount": 40,
                "cachedContentTokenCount": 20
            }),
        );
        assert_eq!(envelope.input_tokens, 100);
        assert_eq!(envelope.output_tokens, 40);
        assert_eq!(envelope.cache_read_tokens, 20);
    }

    #[test]
    fn bedrock_usage() {
        let provider = detect_provider("https://bedrock-runtime.us-east-1.amazonaws.com");
        assert_eq!(provider, ProviderKind::Bedrock);

        let envelope = envelope_from_usage(
            ProviderKind::Bedrock,
            "bedrock",
            &json!({"inputTokens": 90, "outputTokens": 30}),
        );
        assert_eq!(envelope.input_tokens, 90);
        assert_eq!(envelope.output_tokens, 30);
    }

    #[test]
    fn azure_openai() {
        let provider = detect_provider("https://my-resource.openai.azure.com");
        assert_eq!(provider, ProviderKind::Azure);

        let envelope = envelope_from_usage(
            ProviderKind::Azure,
            "gpt-4",
            &json!({"prompt_tokens": 100, "completion_tokens": 50}),
        );
        assert_eq!(envelope.input_tokens, 100);
        assert_eq!(envelope.output_tokens, 50);
    }

    #[test]
    fn empty_usage_no_panic() {
        let envelope = envelope_from_usage(ProviderKind::Unknown, "x", &json!({}));

        assert_eq!(envelope.provider, ProviderKind::Unknown);
        assert_eq!(envelope.input_tokens, 0);
        assert_eq!(envelope.output_tokens, 0);
        assert_eq!(envelope.cache_read_tokens, 0);
        assert_eq!(envelope.cache_write_tokens, 0);
        assert_eq!(envelope.reasoning_tokens, 0);
    }

    #[test]
    fn null_fields_safe() {
        let envelope = envelope_from_usage(
            ProviderKind::OpenAi,
            "gpt-4",
            &json!({
                "prompt_tokens": null,
                "completion_tokens": "not_a_number"
            }),
        );

        assert_eq!(envelope.input_tokens, 0);
        assert_eq!(envelope.output_tokens, 0);
    }
}
