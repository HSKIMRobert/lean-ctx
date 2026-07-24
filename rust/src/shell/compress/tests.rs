use super::classification::{has_structural_output, is_git_data_command, is_verbatim_output};
use super::engine::compress_if_beneficial;
use super::is_excluded_command;

#[cfg(test)]
#[path = "tests_cli_api_data.rs"]
mod cli_api_data_tests;
#[cfg(test)]
#[path = "tests_passthrough.rs"]
mod passthrough_tests;
#[cfg(test)]
#[path = "tests_structural_output.rs"]
mod structural_output_tests;
#[cfg(test)]
#[path = "tests_verbatim_output.rs"]
mod verbatim_output_tests;

/// Regression guard: test-runner output must never have its pass/fail summaries
/// compressed or truncated away — even a large, fully-passing run. This is the
/// exact failure where a multi-crate `cargo test` lost its per-binary
/// `test result:` lines and the user had to fall back to LEAN_CTX_RAW=1.
#[cfg(test)]
#[path = "tests_test_runner_output.rs"]
mod test_runner_output_tests;
