.PHONY: push-github push-gitlab push-all setup-hooks test cloud-build

# ── Push targets ──────────────────────────────────────────

push-github: ## Push open-source code to GitHub (cloud/ excluded via .gitignore)
	git push github main

push-gitlab: ## Push everything (incl. cloud/) to GitLab
	git add -f cloud/ docker-compose.yml 2>/dev/null || true
	git stash push -m "gitlab-push-staging" -- cloud/ docker-compose.yml 2>/dev/null || true
	git push origin main
	@echo "Pushed to GitLab (origin)."

push-all: push-github push-gitlab ## Push to both remotes

# ── Setup ─────────────────────────────────────────────────

setup-hooks: ## Configure git to use .githooks/ for hooks
	git config core.hooksPath .githooks
	@echo "Git hooks configured: .githooks/"

# ── Build ─────────────────────────────────────────────────

test: ## Run all Rust tests + clippy
	cd rust && cargo test && cargo clippy

cloud-build: ## Build cloud backend
	cd cloud && cargo build

cloud-release: ## Release build cloud backend
	cd cloud && cargo build --release

# ── Help ──────────────────────────────────────────────────

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
