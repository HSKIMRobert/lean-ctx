# Agent 04: Stream Controller Foundation

## Preamble
Read `../.worktrees/e13-preamble.md` first for project context and build commands.

## Objective
Create a new `stream_controller` module in `core/context_kernel/` that handles
append-stream workloads (terminal output, build logs, file watchers). The
controller tracks content generations, detects appends vs rotations, and
delivers minimal deltas instead of full content.

## Files to Modify
- `rust/src/core/context_kernel/mod.rs` (MODIFY: add `pub mod stream_controller;`)

## Files to Create
- `rust/src/core/context_kernel/stream_controller.rs` (NEW, ~250 LOC max)

## Files NOT to Touch
- Do NOT modify `types.rs`, `bridge.rs`, `enforce.rs`, `ocla_bus.rs`
- Do NOT modify `proxy/` or `server/` files
- Do NOT modify any tool files

## Exact Requirements

### 1. StreamRef — identifies a tracked stream
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamRef {
    pub source_id: String,
    pub stream_type: StreamType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamType {
    Terminal,
    BuildLog,
    FileWatch,
    Custom,
}
```

### 2. StreamState — tracks a single stream's state
```rust
#[derive(Debug, Clone)]
pub struct StreamState {
    pub generation: u64,
    pub line_cursor: usize,
    pub byte_cursor: usize,
    pub prefix_hash: u64,
    pub last_seen: std::time::Instant,
    pub total_lines: usize,
}
```

### 3. StreamDelta — the delivery result
```rust
#[derive(Debug, Clone)]
pub enum StreamDelta {
    /// Content unchanged since last check — deliver nothing
    Unchanged,
    /// New lines appended — deliver only the new portion
    Append { new_lines: Vec<String>, from_line: usize },
    /// Content rotated/replaced — deliver full snapshot
    Rotation { full_content: Vec<String>, reason: String },
    /// Stream expired — client should discard cached state
    Expired,
}
```

### 4. StreamController — the main controller
```rust
pub struct StreamController {
    streams: std::collections::HashMap<StreamRef, StreamState>,
    max_tracked: usize,
    expiry: std::time::Duration,
}

impl StreamController {
    pub fn new(max_tracked: usize, expiry_secs: u64) -> Self;

    /// Compare current content with tracked state. Returns the minimal delta.
    pub fn compute_delta(
        &mut self,
        stream_ref: &StreamRef,
        current_content: &[String],
    ) -> StreamDelta;

    /// Remove expired streams and return the number removed.
    pub fn gc(&mut self) -> usize;

    /// Number of actively tracked streams.
    pub fn tracked_count(&self) -> usize;
}
```

### 5. Delta computation logic
```
fn compute_delta:
  1. If stream_ref not in self.streams:
     - Store new state (generation=1, cursor at end, hash prefix)
     - Return Rotation { full_content, reason: "first_seen" }

  2. If current_content is empty:
     - Return Unchanged (don't track empty streams)

  3. Compute prefix_hash of first min(10, len) lines
     If prefix_hash != stored prefix_hash:
       - Increment generation, reset cursors
       - Return Rotation { full_content, reason: "prefix_changed" }

  4. If current_content.len() == stored.total_lines && hash matches:
     - Return Unchanged

  5. If current_content.len() > stored.total_lines:
     - Verify prefix still matches (first N lines same)
     - Return Append { new_lines: content[stored.line_cursor..], from_line: stored.line_cursor }
     - Update cursor

  6. If current_content.len() < stored.total_lines:
     - Return Rotation { full_content, reason: "truncated" }
```

### 6. Prefix hash function
Use a simple hash (not cryptographic) for speed:
```rust
fn compute_prefix_hash(lines: &[String], max_lines: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for line in lines.iter().take(max_lines) {
        line.hash(&mut hasher);
    }
    hasher.finish()
}
```

### 7. Tests (at least 6)
- `test_first_seen_returns_rotation` — new stream returns full content
- `test_unchanged_content_returns_unchanged` — same content on second call
- `test_append_detection` — added lines at end → Append delta
- `test_prefix_change_returns_rotation` — modified first lines → Rotation
- `test_truncation_returns_rotation` — shorter content → Rotation
- `test_gc_removes_expired_streams` — expired streams cleaned up
- `test_empty_content_unchanged` — empty content is not tracked

## NOT in Scope
- Do NOT integrate with ctx_shell or proxy (future wiring work)
- Do NOT add async/tokio — keep it sync
- Do NOT add new dependencies to Cargo.toml
- Do NOT modify the existing tools layer

## Build Verification
```bash
cd rust && cargo fmt && cargo clippy --lib -- -D warnings && cargo test --lib stream_controller
```
