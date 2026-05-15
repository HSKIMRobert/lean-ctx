.PHONY: setup-hooks install dev test help

# ── Setup ─────────────────────────────────────────────────

setup-hooks: ## Configure git to use .githooks/ for hooks
	git config core.hooksPath .githooks
	@echo "Git hooks configured: .githooks/"

# ── Build & Install ──────────────────────────────────────

install: ## Build release + install to ~/.cargo/bin
	cd rust && cargo install --path .
	@echo "Installed: $$(lean-ctx --version)"

dev: ## Quick debug build + copy to ~/.cargo/bin
	cd rust && cargo build && cp target/debug/lean-ctx ~/.cargo/bin/lean-ctx
	@echo "Dev installed: $$(lean-ctx --version)"

test: ## Run all Rust tests + clippy
	cd rust && cargo test && cargo clippy

# ── Help ──────────────────────────────────────────────────

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
