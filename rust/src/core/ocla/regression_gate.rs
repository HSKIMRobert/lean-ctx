//! Compression hot-path regression gate (E11, master-plan P4).
//!
//! Compares concrete `BuiltinCompressionProvider` calls with
//! `dyn CompressionProvider` dispatch through `OclaRegistry`.
//! Fails if dynamic dispatch adds more than 2% wall-clock latency.

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::time::Instant;

    use crate::core::ocla::OclaRegistry;
    use crate::core::ocla::builtin::compression_provider::BuiltinCompressionProvider;
    use crate::core::ocla::traits::CompressionProvider;
    use crate::core::ocla::types::{CompressionRequest, OclaRequestContext};
    use crate::core::tokens;

    const ITERATIONS: usize = 1000;
    const MAX_REGRESSION_PCT: f64 = 2.0;
    const SOURCE_REF: &str = "file:rust/src/core/ocla/regression_gate.rs";

    fn make_context() -> OclaRequestContext {
        OclaRequestContext::new(
            "bench-compression".to_string(),
            "bench".to_string(),
            "regression-gate".to_string(),
            SOURCE_REF.to_string(),
            None,
            Some("compression-regression-gate".to_string()),
        )
    }

    fn make_request(source_tokens: u64) -> CompressionRequest {
        CompressionRequest {
            context: make_context(),
            source_ref: SOURCE_REF.to_string(),
            source_tokens,
            target_tokens: source_tokens,
            quality_policy_ref: None,
        }
    }

    fn measure(
        iterations: usize,
        source_tokens: u64,
        mut compress: impl FnMut(CompressionRequest),
    ) -> f64 {
        let requests: Vec<_> = (0..iterations)
            .map(|_| make_request(source_tokens))
            .collect();
        let start = Instant::now();
        for request in requests {
            compress(request);
        }
        start.elapsed().as_nanos() as f64
    }

    #[test]
    fn compression_dyn_dispatch_regression_gate() {
        let source = std::fs::read_to_string(
            SOURCE_REF
                .strip_prefix("file:rust/")
                .expect("source ref must point inside the rust crate"),
        )
        .expect("regression-gate source must be readable");
        let source_tokens = tokens::count_tokens(&source) as u64;
        let direct_provider = BuiltinCompressionProvider::new();
        let registry = OclaRegistry::global();

        for _ in 0..10 {
            black_box(
                direct_provider
                    .compress(make_request(source_tokens))
                    .expect("direct warm-up compression must succeed"),
            );
            black_box(
                registry
                    .compression_provider
                    .compress(make_request(source_tokens))
                    .expect("dynamic warm-up compression must succeed"),
            );
        }

        let direct_ns = measure(ITERATIONS, source_tokens, |request| {
            black_box(
                direct_provider
                    .compress(request)
                    .expect("direct compression must succeed"),
            );
        });

        let dyn_ns = measure(ITERATIONS, source_tokens, |request| {
            black_box(
                registry
                    .compression_provider
                    .compress(request)
                    .expect("dynamic compression must succeed"),
            );
        });

        let regression_pct = ((dyn_ns - direct_ns) / direct_ns) * 100.0;
        eprintln!(
            "Compression regression gate: direct={:.0}ns, dyn={:.0}ns, delta={:.2}%",
            direct_ns / ITERATIONS as f64,
            dyn_ns / ITERATIONS as f64,
            regression_pct
        );

        assert!(
            regression_pct < MAX_REGRESSION_PCT,
            "dyn CompressionProvider adds {regression_pct:.2}% latency \
             (max {MAX_REGRESSION_PCT}%)"
        );
    }
}
