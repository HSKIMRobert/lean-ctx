# LeanCTX Website Deployment

## Architecture

- **Source**: `website/` directory (Astro + Tailwind, NOT tracked in git)
- **Server**: `pounce-server` (185.142.213.170, user: administrator)
- **Runtime**: Docker container `lean-ctx-web` (nginx:alpine)
- **Reverse Proxy**: Traefik on `coolify` network with auto-TLS (Let's Encrypt)
- **Domains**: leanctx.com, www.leanctx.com, lean-ctx.pounce.ch, leanctx.tech, www.leanctx.tech

## Prerequisites

- SSH access: `ssh pounce-server` (configured in `~/.ssh/config`)
- SSH key: `~/.ssh/pounce_server`
- Docker on server (requires sudo)
- Node.js locally for building

## Deployment Steps

### 1. Build Locally (verify changes)

```bash
cd website
npm run build
```

### 2. Sync Files to Server

```bash
rsync -avz --delete \
  --exclude='node_modules' \
  --exclude='dist' \
  --exclude='.astro' \
  --exclude='.vscode' \
  website/ pounce-server:/home/administrator/lean-ctx/website/
```

Also sync the Dockerfile if changed:

```bash
rsync -avz Dockerfile.web pounce-server:/home/administrator/lean-ctx/Dockerfile.web
```

### 3. Build & Deploy via GitLab CI

The fastest way to deploy (handles Docker build + restart with sudo):

```bash
# Temporarily add CI config to git
cd /path/to/lean-ctx
git add -f .gitlab-ci.yml

# Make deploy-website auto-trigger (remove "when: manual")
# Edit .gitlab-ci.yml: remove "when: manual" and "allow_failure: true" from deploy-website rules

git commit -m "ci: trigger website deployment"
git push origin main

# Wait for pipeline to succeed (GitLab project ID: 5)
# Check: GitLab MCP → list_pipelines project_id=5

# Clean up: remove CI config from git again
git rm --cached .gitlab-ci.yml
git commit -m "chore: remove CI config from tracking"
git push origin main
git push github main
```

### Alternative: Manual Docker Deploy (requires sudo password)

```bash
ssh pounce-server
cd /home/administrator/lean-ctx
sudo docker build --no-cache -f Dockerfile.web -t lean-ctx-web .
sudo docker stop lean-ctx-web && sudo docker rm lean-ctx-web
sudo docker run -d \
  --name lean-ctx-web \
  --network coolify \
  --restart unless-stopped \
  -l "traefik.enable=true" \
  -l "traefik.docker.network=coolify" \
  -l 'traefik.http.routers.lean-ctx.rule=Host(`leanctx.com`) || Host(`www.leanctx.com`) || Host(`lean-ctx.pounce.ch`) || Host(`leanctx.tech`) || Host(`www.leanctx.tech`)' \
  -l "traefik.http.routers.lean-ctx.entrypoints=https" \
  -l "traefik.http.routers.lean-ctx.tls=true" \
  -l "traefik.http.routers.lean-ctx.tls.certresolver=letsencrypt" \
  -l "traefik.http.services.lean-ctx.loadbalancer.server.port=80" \
  lean-ctx-web
```

## Verification

```bash
curl -sL https://leanctx.com/ -o /dev/null -w "%{http_code}"       # Should be 200
curl -sL https://leanctx.com/docs/installation/ | grep "yvgude"     # Check content
curl -sL https://leanctx.com/docs/tools-benchmark/ | grep "tdd"     # Check TDD content
```

## Important Notes

- **Website is NOT in git** — `website/` is in `.gitignore`. Changes are deployed manually via rsync + CI.
- **`.gitlab-ci.yml` is NOT in git** — it lives on disk only. Add it temporarily to trigger CI, then remove.
- **GitLab project ID**: 5 (for MCP tools: `list_pipelines project_id=5`)
- **GitLab CI Variables** (stored in GitLab Settings > CI/CD > Variables):
  - `DEPLOY_SSH_KEY` — SSH private key for server access
  - `DEPLOY_HOST` — Server IP (185.142.213.170)
  - `DEPLOY_USER` — SSH user (administrator)
  - `DEPLOY_SUDO_PASSWORD` — sudo password for Docker commands

## File Structure

```
lean-ctx/
├── website/                    # NOT in git
│   ├── src/pages/docs/         # 16 documentation pages
│   ├── src/pages/features.astro
│   ├── src/pages/index.astro
│   ├── src/components/         # Shared UI components
│   ├── src/layouts/            # Base + Docs layouts
│   ├── src/styles/global.css   # Tailwind styles
│   ├── nginx.conf              # Server config (inside Docker)
│   ├── package.json
│   └── DEPLOYMENT.md           # This file
├── Dockerfile.web              # NOT in git, multi-stage Docker build
└── .gitlab-ci.yml              # NOT in git, CI/CD pipeline
```
