# Enterprise CI

`lean-ctx-enterprise` is a separate private Rust workspace. Its CI must build
the real feature-gated integration path; compiling only the public compatibility
shims is not sufficient.

## Setup

Clone the private repository next to this checkout, or provide its path through
`ENTERPRISE_DIR`:

```sh
git clone <enterprise-repository-url> ../lean-ctx-enterprise
make enterprise-ci-check

# Alternative checkout location
make enterprise-ci-check ENTERPRISE_DIR=/path/to/lean-ctx-enterprise
```

The validation target confirms that the checkout is a Cargo workspace before
CI invokes Cargo from it.

## Required enterprise CI jobs

Run these jobs from the `lean-ctx-enterprise` workspace on every pull request
and protected-branch push:

```sh
cargo check --workspace --features engine-integration
cargo test --workspace --features engine-integration
cargo clippy --workspace --all-targets --features engine-integration -- -D warnings
cargo fmt --all -- --check
```

The `cargo check` job is the required compile gate. It must select every
enterprise crate in the workspace and enable `engine-integration`; a check of
only `compat.rs` does not exercise the production integration code. Keep the
test and clippy jobs feature-enabled as well, so feature-only regressions and
warnings cannot bypass the compile gate.

## `compat.rs` removal timeline

1. Until the `engine-integration` check is required and green for every
   enterprise crate, retain `compat.rs` as the compatibility boundary.
2. Migrate each enterprise crate to the feature-gated integration API and keep
   the feature-enabled check, test, and clippy jobs required.
3. Remove `compat.rs` only after all enterprise crates build and test through
   `engine-integration` in required CI; remove its callers in the same change.

After removal, keep the feature-enabled jobs permanently. They are the guard
against silently returning to shim-only builds.
