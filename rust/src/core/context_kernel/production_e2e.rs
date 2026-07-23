//! End-to-end tests for production Context Kernel activation.

#[cfg(test)]
mod tests {
    use std::sync::MutexGuard;

    use crate::core::context_kernel::{
        config_bridge, ctx_read_dedup, dedup_wiring, envelope_wiring, kernel_config,
        list_tools_opt, mcp_bridge, proxy_bridge, schema_wiring,
    };

    fn isolated() -> MutexGuard<'static, ()> {
        let guard = kernel_config::KERNEL_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        kernel_config::reset_features();
        dedup_wiring::reset_dedup();
        schema_wiring::reset_schema_state();
        envelope_wiring::reset_evidence();
        proxy_bridge::reset_state();
        mcp_bridge::reset_mcp_state();
        guard
    }

    fn tools(count: usize) -> Vec<(String, String, usize)> {
        (0..count)
            .map(|index| (format!("tool_{index}"), "A".repeat(4000), 3))
            .collect()
    }

    fn proxy_data(index: usize) -> proxy_bridge::ProxyRequestData {
        proxy_bridge::ProxyRequestData {
            headers: vec![("x-user-id".to_owned(), format!("e2e-user-{index}"))],
            input_tokens: 100,
            output_tokens: 20,
            tokens_saved: 25,
            request_count: index + 1,
            ..proxy_bridge::ProxyRequestData::default()
        }
    }

    fn process_proxy(index: usize) {
        let data = proxy_data(index);
        let result = proxy_bridge::process_proxy_request(&data);
        envelope_wiring::process_proxy_evidence(&data, &result);
    }

    fn mcp_data(index: usize) -> mcp_bridge::McpCallData {
        mcp_bridge::McpCallData {
            tool_name: format!("ctx_e2e_{index}"),
            input_tokens: 80,
            output_tokens: 20,
            call_number: index + 1,
            ..mcp_bridge::McpCallData::default()
        }
    }

    fn process_mcp(index: usize) {
        let data = mcp_data(index);
        mcp_bridge::record_mcp_call(&data);
        envelope_wiring::process_mcp_evidence(&data);
    }

    fn assert_tools_reduced(
        original: &[(String, String, usize)],
        optimized: &[(String, String, usize)],
    ) {
        let original_total: usize = original.iter().map(|t| t.1.len()).sum();
        let optimized_total: usize = optimized.iter().map(|t| t.1.len()).sum();
        assert!(
            optimized_total < original_total,
            "optimized total ({optimized_total}) should be less than original ({original_total})"
        );
    }

    fn assert_default_features(features: &kernel_config::KernelFeatures) {
        let defaults = kernel_config::KernelFeatures::default();
        assert_eq!(features.enabled, defaults.enabled);
        assert_eq!(features.proxy_etpao, defaults.proxy_etpao);
        assert_eq!(features.mcp_etpao, defaults.mcp_etpao);
        assert_eq!(features.content_dedup, defaults.content_dedup);
        assert_eq!(features.schema_optimization, defaults.schema_optimization);
        assert_eq!(features.receipt_chain, defaults.receipt_chain);
        assert_eq!(features.usage_tracking, defaults.usage_tracking);
        assert_eq!(features.identity_tracking, defaults.identity_tracking);
        assert_eq!(features.max_kernel_budget, defaults.max_kernel_budget);
        assert_eq!(features.dedup_capacity, defaults.dedup_capacity);
    }

    /// Content long enough that the dedup stub is shorter than full delivery.
    const LONG_CONTENT: &str = "fn main() {\n    let very_long_variable = 42;\n    println!(\"{very_long_variable}\");\n    for i in 0..100 {\n        println!(\"iteration {i}\");\n    }\n}\n";

    #[test]
    fn dedup_bridge_reduces_repeated_reads() {
        let _guard = isolated();

        assert_eq!(ctx_read_dedup::try_dedup("file.rs", LONG_CONTENT), None);
        let stub = ctx_read_dedup::try_dedup("file.rs", LONG_CONTENT)
            .expect("repeated content should produce a dedup stub");
        assert!(
            stub.len() < LONG_CONTENT.len(),
            "stub ({}) should be shorter than content ({})",
            stub.len(),
            LONG_CONTENT.len()
        );
    }

    #[test]
    fn schema_opt_reduces_tool_tokens() {
        let _guard = isolated();
        kernel_config::update_features(kernel_config::KernelFeatures::default());
        let original = tools(15);

        let optimized = list_tools_opt::optimize_descriptions(original.clone(), "cursor");

        assert_tools_reduced(&original, &optimized);
    }

    #[test]
    fn config_bridge_applies() {
        let _guard = isolated();

        config_bridge::apply_config();

        assert!(kernel_config::is_enabled());
    }

    #[test]
    fn config_controls_dedup() {
        let _guard = isolated();
        let features = kernel_config::KernelFeatures {
            content_dedup: false,
            ..kernel_config::KernelFeatures::default()
        };
        kernel_config::update_features(features.clone());

        assert!(!ctx_read_dedup::should_dedup("default"));

        let features = kernel_config::KernelFeatures {
            content_dedup: true,
            ..kernel_config::KernelFeatures::default()
        };
        kernel_config::update_features(features);

        assert!(ctx_read_dedup::should_dedup("default"));
    }

    #[test]
    fn evidence_chain_after_activity() {
        let _guard = isolated();

        for index in 0..3 {
            process_proxy(index);
        }
        for index in 0..2 {
            process_mcp(index);
        }

        assert!(envelope_wiring::evidence_summary().total_envelopes >= 5);
    }

    #[test]
    fn full_production_pipeline() {
        let _guard = isolated();
        kernel_config::update_features(kernel_config::KernelFeatures::default());
        let files: [(&str, &str); 3] = [
            (
                "one.rs",
                "fn one() {\n    let value = 1;\n    println!(\"{value}\");\n    for i in 0..10 { println!(\"line {i}\"); }\n}\n",
            ),
            (
                "two.rs",
                "fn two() {\n    let value = 2;\n    println!(\"{value}\");\n    for i in 0..20 { println!(\"line {i}\"); }\n}\n",
            ),
            (
                "three.rs",
                "fn three() {\n    let value = 3;\n    println!(\"{value}\");\n    for i in 0..30 { println!(\"line {i}\"); }\n}\n",
            ),
        ];

        for (path, content) in files {
            assert_eq!(ctx_read_dedup::try_dedup(path, content), None);
        }
        for (path, content) in files {
            assert!(ctx_read_dedup::try_dedup(path, content).is_some());
        }

        let original = tools(10);
        let optimized = list_tools_opt::optimize_descriptions(original.clone(), "cursor");
        assert_tools_reduced(&original, &optimized);

        for index in 0..5 {
            process_proxy(index);
        }

        assert!(envelope_wiring::evidence_summary().total_envelopes >= 5);
        assert_eq!(dedup_wiring::dedup_stats().cache_hits, 3);
    }

    #[test]
    fn disabled_kernel_no_effects() {
        let _guard = isolated();
        let features = kernel_config::KernelFeatures {
            enabled: false,
            ..kernel_config::KernelFeatures::default()
        };
        kernel_config::update_features(features);

        assert_eq!(ctx_read_dedup::try_dedup("disabled.rs", LONG_CONTENT), None);
        assert_eq!(ctx_read_dedup::try_dedup("disabled.rs", LONG_CONTENT), None);
        assert!(!list_tools_opt::should_optimize_for_client("cursor"));

        process_proxy(0);
        process_mcp(0);

        assert_eq!(
            envelope_wiring::evidence_summary(),
            envelope_wiring::EvidenceSummary::default()
        );
    }

    #[test]
    fn config_report_accurate() {
        let _guard = isolated();

        let report = config_bridge::config_report();

        assert_eq!(format!("{:?}", report.source), "Default");
        assert_default_features(&report.features);
    }
}
