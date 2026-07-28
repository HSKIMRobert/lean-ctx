# Agent 02: Enforce Wiring on MCP + Proxy Hot Paths

## Preamble
Read `../.worktrees/e13-preamble.md` first for project context and build commands.

## Objective
Wire `enforce_plan()` into the MCP and proxy hot paths so that when
`KernelMode::Enforce` is active, policy-violating plan entries are actually
removed (not just logged as in Shadow mode).

Currently, `enforce_plan()` exists in `enforce.rs` but is ONLY called from tests.
The hot paths in `bridge.rs::kernel_enrich()` generate a plan but never enforce it.

## Files to Modify
- `rust/src/core/context_kernel/bridge.rs` (MODIFY: call enforce_plan after plan generation)
- `rust/src/core/context_kernel/hotpath_wiring.rs` (MODIFY: pass enforce result through)

## Files to Create
- NONE

## Files NOT to Touch
- Do NOT modify `enforce.rs` (the function is already correct)
- Do NOT modify `types.rs`, `activation.rs`, `ocla_bus.rs`
- Do NOT modify `server/post_dispatch.rs` or `proxy/forward/mod.rs`
- Do NOT modify any tool files (ctx_read, ctx_compose, etc.)

## Exact Requirements

### 1. In bridge.rs::kernel_enrich(), call enforce_plan after plan generation

Current flow (simplified):
```rust
pub fn kernel_enrich(task, project_root, budget_tokens) -> Option<KernelEnrichment> {
    let kernel = ContextKernel::for_project(project_root);
    let plan = kernel.plan(&ctx);  // generates plan
    // ... builds blocks from plan.selected
    // PROBLEM: plan.selected is never filtered by policy
}
```

New flow — after `kernel.plan(&ctx)`:
```rust
let mode = super::enforce::resolve_mode(project_root);
let enforced = if mode == super::enforce::KernelMode::Enforce {
    let policy = super::policy::ContextPolicy::default();
    let result = super::enforce::enforce_plan(plan.clone(), &policy, mode);
    if !result.blocked.is_empty() {
        tracing::debug!(
            blocked = result.blocked.len(),
            "kernel enforce: plan entries blocked by policy"
        );
    }
    ContextPlanV1 {
        selected: result.allowed,
        ..plan
    }
} else {
    plan
};
```
Then use `enforced` instead of `plan` for building blocks.

### 2. In hotpath_wiring.rs, propagate enforcement

If `hotpath_wiring.rs` has functions that call `kernel_enrich` or generate
plans independently, ensure they also call `enforce_plan` when in Enforce mode.

Look for any function that:
- Calls `ContextKernel::plan()`
- Builds enrichment from `plan.selected`
And add the same enforce pattern.

### 3. Add KernelEnrichment.enforced_mode field
Add a field to `KernelEnrichment` so callers know which mode was used:
```rust
pub struct KernelEnrichment {
    pub plan: ContextPlanV1,
    pub blocks: String,
    pub verdict: KernelVerdict,
    pub enforced_mode: super::enforce::KernelMode,  // NEW
}
```
Set this field in `kernel_enrich()`.

### 4. Tests
- `test_enforce_mode_blocks_policy_violations` — plan entries with low phi score or excluded providers are removed in Enforce mode
- `test_shadow_mode_allows_all_entries` — same entries pass through in Shadow mode

## NOT in Scope
- Do NOT change the enforce.rs logic itself
- Do NOT add new CLI commands or API endpoints
- Do NOT modify the tool dispatch layer
- Do NOT add new dependencies

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib bridge
```
