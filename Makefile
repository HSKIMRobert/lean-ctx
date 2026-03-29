.PHONY: push-github push-gitlab push-all setup-hooks test cloud-build cloud-release help

# ── Push targets ──────────────────────────────────────────

push-github: ## Push open-source code to GitHub (cloud/ excluded via .gitignore)
	git push github main

push-gitlab: ## Push main + deploy branch (with cloud/) to GitLab
	@echo "Pushing main to GitLab..."
	git push origin main
	@echo "Creating deploy branch with proprietary code..."
	git checkout -B deploy main
	git checkout origin/deploy -- cloud/ docker-compose.yml 2>/dev/null || true
	git add -f cloud/.gitignore cloud/.env.example cloud/Cargo.toml cloud/Cargo.lock \
		cloud/Dockerfile cloud/migrations/ cloud/src/ docker-compose.yml
	git rm -r --cached cloud/target 2>/dev/null || true
	git commit -m "deploy: include proprietary cloud backend" --allow-empty
	git push origin deploy --force
	git checkout main
	@# Restore proprietary files locally after branch switch
	git checkout origin/deploy -- cloud/ docker-compose.yml 2>/dev/null || true
	git reset HEAD cloud/ docker-compose.yml 2>/dev/null || true
	@echo "Done. GitLab has main (clean) + deploy (full)."

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
