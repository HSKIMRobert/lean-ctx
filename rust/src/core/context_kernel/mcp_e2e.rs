//! End-to-end conformance tests for the MCP-to-kernel bridge.

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::super::client_wiring::OptimizationLevel;
    use super::super::coverage_class::CoverageClass;
    use super::super::mcp_bridge::{self, McpCallData, McpClientInfo};
    use super::super::mcp_coverage;
    use super::super::mcp_receipt::{self, McpReceipt};
    use super::super::mcp_schema_opt::{self, SchemaBudget, SchemaEntry};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn isolated_test() -> MutexGuard<'static, ()> {
        let guard = match TEST_LOCK.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        mcp_bridge::reset_mcp_state();
        mcp_receipt::reset_mcp_receipts();
        guard
    }

    fn cursor() -> McpClientInfo {
        McpClientInfo {
            client_name: "cursor".to_owned(),
            supports_roots: true,
            tool_count: 15,
        }
    }

    fn call(tool_name: &str, output_tokens: usize) -> McpCallData {
        McpCallData {
            tool_name: tool_name.to_owned(),
            input_tokens: 1_000,
            output_tokens,
            schema_tokens: 100,
            accepted: true,
        }
    }

    fn receipt(tool_name: &str) -> McpReceipt {
        McpReceipt {
            tool_name: tool_name.to_owned(),
            original_tokens: 1_000,
            compressed_tokens: 400,
            kernel_overhead_tokens: 50,
            accepted: true,
        }
    }

    #[test]
    fn full_mcp_lifecycle() {
        let _guard = isolated_test();
        let context = mcp_bridge::process_mcp_context(&cursor());
        assert_eq!(context.coverage, CoverageClass::FullInline);

        for output_tokens in [100, 125, 150] {
            mcp_bridge::record_mcp_call(&call("ctx_read", output_tokens));
            mcp_receipt::record_mcp_receipt(&receipt("ctx_read"));
        }

        assert!(mcp_bridge::mcp_etpao() > 0.0);
        let accounting = mcp_receipt::mcp_accounting();
        assert_eq!(
            accounting.delivered_tokens,
            accounting
                .compressed_tokens
                .saturating_add(accounting.kernel_overhead_tokens)
        );
        assert!(accounting.actual_compression_ratio <= accounting.reported_compression_ratio);
    }

    #[test]
    fn schema_compression_saves_tokens() {
        let _guard = isolated_test();
        let schemas = (0..10)
            .map(|index| SchemaEntry {
                name: format!("tool-{index}"),
                description: "x".repeat(200),
            })
            .collect::<Vec<_>>();
        let budget = SchemaBudget {
            max_total: 2_000,
            max_per_tool: 150,
        };

        let result = mcp_schema_opt::optimize_schemas(&schemas, budget);

        assert!(result.tokens_after < result.tokens_before);
        assert!(result.compressed_count > 0);
    }

    #[test]
    fn cursor_full_pipeline() {
        let _guard = isolated_test();
        assert_eq!(
            mcp_coverage::detect_mcp_coverage("cursor", true, true),
            CoverageClass::FullInline
        );
        assert_eq!(
            mcp_coverage::mcp_client_profile("cursor").context_window,
            200_000
        );
        assert_eq!(
            mcp_coverage::mcp_optimization_level("cursor"),
            OptimizationLevel::Full
        );
    }

    #[test]
    fn vscode_pipeline() {
        let _guard = isolated_test();
        assert_eq!(
            mcp_coverage::detect_mcp_coverage("vscode", true, true),
            CoverageClass::ContextControlled
        );
        assert_eq!(
            mcp_coverage::mcp_client_profile("vscode").context_window,
            128_000
        );
        assert_eq!(
            mcp_coverage::mcp_optimization_level("vscode"),
            OptimizationLevel::Standard
        );
    }

    #[test]
    fn receipt_per_tool_tracking() {
        let _guard = isolated_test();
        for _ in 0..5 {
            mcp_receipt::record_mcp_receipt(&receipt("ctx_read"));
        }
        for _ in 0..3 {
            mcp_receipt::record_mcp_receipt(&receipt("ctx_search"));
        }

        let savings = mcp_receipt::per_tool_savings();
        assert_eq!(savings.len(), 2);
        let read = savings
            .iter()
            .find(|entry| entry.tool_name == "ctx_read")
            .expect("ctx_read savings must be present");
        assert_eq!(read.calls, 5);
    }

    #[test]
    fn etpao_tracks_mcp_calls() {
        let _guard = isolated_test();
        mcp_bridge::reset_mcp_state();
        for index in 0..10 {
            mcp_bridge::record_mcp_call(&call("ctx_read", 50 + index));
        }

        assert!(mcp_bridge::mcp_etpao() > 0.0);
        assert_eq!(mcp_bridge::mcp_summary().total_calls, 10);
    }

    #[test]
    fn coverage_affects_schema_budget() {
        let _guard = isolated_test();
        let full = mcp_schema_opt::budget_for_coverage(CoverageClass::FullInline);
        let controlled = mcp_schema_opt::budget_for_coverage(CoverageClass::ContextControlled);
        let observe = mcp_schema_opt::budget_for_coverage(CoverageClass::ObserveOnly);

        assert!(full.max_total > observe.max_total);
        assert_eq!(full.max_total, 12_000);
        assert_eq!(controlled.max_total, 8_000);
        assert_eq!(observe.max_total, 4_000);
    }

    #[test]
    fn end_to_end_identity_to_receipt() {
        let _guard = isolated_test();
        mcp_bridge::reset_mcp_state();
        mcp_receipt::reset_mcp_receipts();
        mcp_bridge::process_mcp_context(&cursor());

        for index in 0..5 {
            mcp_bridge::record_mcp_call(&call("ctx_read", 100 + index));
            mcp_receipt::record_mcp_receipt(&receipt("ctx_read"));
        }

        assert_eq!(mcp_bridge::mcp_summary().total_calls, 5);
        assert!(!mcp_receipt::per_tool_savings().is_empty());
        assert!(mcp_receipt::mcp_accounting().kernel_overhead_tokens > 0);
    }
}
