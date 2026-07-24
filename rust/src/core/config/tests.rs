//! Unit tests for [`Config`] parsing, defaults, and section behaviour.
//!
//! Split by configuration area to keep each test source focused.

use super::*;

#[cfg(test)]
#[path = "tests_disabled_tools.rs"]
mod disabled_tools_tests;

#[cfg(test)]
#[path = "tests_prefer_native_editor.rs"]
mod prefer_native_editor_tests;

#[cfg(test)]
#[path = "tests_default_tool_categories.rs"]
mod default_tool_categories_tests;

#[cfg(test)]
#[path = "tests_no_degrade.rs"]
mod no_degrade_tests;

#[cfg(test)]
#[path = "tests_delta_explicit.rs"]
mod delta_explicit_tests;

#[cfg(test)]
#[path = "tests_rules_scope.rs"]
mod rules_scope_tests;

#[cfg(test)]
#[path = "tests_rules_injection.rs"]
mod rules_injection_tests;

#[cfg(test)]
#[path = "tests_permission_inheritance.rs"]
mod permission_inheritance_tests;

#[cfg(test)]
#[path = "tests_loop_detection_config.rs"]
mod loop_detection_config_tests;

#[cfg(test)]
#[path = "tests_extra_roots.rs"]
mod extra_roots_tests;

#[cfg(test)]
#[path = "tests_config_load_cache.rs"]
mod config_load_cache_tests;

#[cfg(test)]
#[path = "tests_cost_config.rs"]
mod cost_config_tests;

#[cfg(test)]
#[path = "tests_persist_global.rs"]
mod persist_global_tests;

#[cfg(test)]
#[path = "tests_max_index_threads.rs"]
mod max_index_threads_tests;

#[cfg(test)]
#[path = "tests_shell_timeout_and_writes.rs"]
mod shell_timeout_and_writes_tests;

#[cfg(test)]
#[path = "tests_config_path_visibility.rs"]
mod config_path_visibility_tests;

#[cfg(test)]
#[path = "tests_context_budget.rs"]
mod context_budget_tests;
