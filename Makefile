.PHONY: setup-hooks install dev test help

# ── Setup ─────────────────────────────────────────────────

setup-hooks: ## Configure git to use .githooks/ for hooks
	git config core.hooksPath .githooks
	@echo "Git hooks configured: .githooks/"

# ── Build & Install ──────────────────────────────────────

install: ## Build release + install to ~/.local/bin
	cd rust && cargo install --path . --force --locked --root "$$HOME/.local"
	@echo "Installed: $$(lean-ctx --version)"

dev: ## Quick debug build + copy to ~/.local/bin
	cd rust && cargo build
	@mkdir -p "$$HOME/.local/bin"
	cp rust/target/debug/lean-ctx "$$HOME/.local/bin/lean-ctx"
	@echo "Dev installed: $$(lean-ctx --version)"

test: ## Run all Rust tests + clippy
	cd rust && cargo test && cargo clippy

# ── Help ──────────────────────────────────────────────────

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
