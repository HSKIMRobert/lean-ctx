# lean-ctx-bin

Pre-built binary distribution of [lean-ctx](https://github.com/yvgude/lean-ctx) — the Cognitive Filter for AI Engineering.

No Rust toolchain required. The correct binary for your platform is downloaded automatically during `npm install`.

## Install

```bash
npm install -g lean-ctx-bin
```

## Supported Platforms

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64, aarch64 |
| macOS | x86_64, Apple Silicon |
| Windows | x86_64 |

## Alternative Install Methods

```bash
# From source (requires Rust)
cargo install lean-ctx

# Homebrew (macOS/Linux)
brew tap yvgude/lean-ctx && brew install lean-ctx

# One-liner (no Rust needed)
curl -fsSL https://raw.githubusercontent.com/yvgude/lean-ctx/main/install.sh | bash -s -- --download
```

## Links

- [Documentation](https://leanctx.com)
- [GitHub](https://github.com/yvgude/lean-ctx)
- [crates.io](https://crates.io/crates/lean-ctx)
