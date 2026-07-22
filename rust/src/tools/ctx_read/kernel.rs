//! Context Kernel integration for ctx_read hot-path.

use std::cell::RefCell;
use std::collections::HashSet;

use crate::core::context_kernel::activation::{load_config, supplement_budget};
use crate::core::context_kernel::context_dedup::dedup_kernel_blocks;

thread_local! {
    static SEEN_KERNEL_BLOCKS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn append_kernel_blocks(result: &mut String, blocks: &str, seen_hashes: &mut HashSet<String>) {
    let framed = format!("--- kernel context ---\n{blocks}");
    result.push('\n');
    result.push_str(&dedup_kernel_blocks(&framed, seen_hashes));
}

/// Enrich a read result with cross-store context from the Context Kernel.
///
/// Returns `true` if enrichment was appended to `result`.
pub(super) fn enrich_with_kernel(result: &mut String, task: Option<&str>) -> bool {
    let (Some(task_str), Some(project_root)) =
        (task, crate::core::config::Config::find_project_root())
    else {
        return false;
    };

    let config = load_config(&project_root);
    let budget = supplement_budget(&config);
    if let Some(enrichment) =
        crate::core::context_kernel::bridge::kernel_enrich(task_str, &project_root, budget)
        && !enrichment.blocks.is_empty()
    {
        SEEN_KERNEL_BLOCKS.with(|seen_hashes| {
            append_kernel_blocks(result, &enrichment.blocks, &mut seen_hashes.borrow_mut());
        });
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::append_kernel_blocks;

    #[test]
    fn duplicate_kernel_blocks_become_stubs() {
        let mut seen_hashes = HashSet::new();
        let mut first = String::new();
        let mut second = String::new();

        append_kernel_blocks(
            &mut first,
            "\n## Relevant Knowledge\n- shared\n",
            &mut seen_hashes,
        );
        append_kernel_blocks(
            &mut second,
            "\n## Relevant Knowledge\n- shared\n",
            &mut seen_hashes,
        );

        assert!(first.contains("- shared"));
        assert!(!second.contains("- shared"));
        assert!(second.contains("kernel context unchanged"));
    }
}
