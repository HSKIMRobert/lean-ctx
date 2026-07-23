# Deployment Guide — lean-ctx

## Enterprise Token-Control-Platform Deployment Target

Diese Datei beschreibt unten bestehende Deployments. Das OCLA/Enterprise-SOLL
ist customer-owned und unterstützt drei Profile:

| Profil | Einsatz |
|---|---|
| In-process | minimale Latenz, Rust/embedded Consumer |
| Sidecar | Partner-Gateway, air-gapped/VPC, klare Prozessgrenze |
| HA Gateway | zentraler Enterprise Token Data Plane |

Pflicht-Gates vor Production:

- Shadow Mode und belastbare Baseline;
- Streaming-/Tool-Call-Parität;
- horizontale Skalierung und Failure Tests;
- Circuit Breaker, Backpressure und Rate Limits;
- konfigurierbares Fail-open/Fail-closed;
- lokale Ledger-, Policy- und Identity-Persistenz;
- kein Betriebszwang zu Thinkery Cloud oder AI Value Gate;
- Backup/Restore, Key Rotation, Audit und Rollback Runbooks.

Ein Enterprise-Rollout folgt `OBSERVE → MEASURE → CONTROL → OPTIMIZE → AUTOMATE`.

## Repository- und Delivery-Topologie (Target)

| Repository | Verantwortung |
|---|---|
| GitHub `yvgude/lean-ctx` | kanonischer Apache-2.0 Source + OSS Releases |
| GitLab `root/lean-ctx` | read-only OSS Mirror |
| GitLab `root/lean-ctx-enterprise` | AI Value Gate / Single-Tenant Control Plane |
| GitLab `root/lean-ctx-cloud` | SaaS, Billing, Sync, License Issuance |
| GitLab `root/lean-ctx-deploy` | Helm/Terraform Deployment Factory |
| GitLab `<customer>-deploy` | Values, Digests, Secret-Refs, Runbooks; kein Code |

Production Delivery:

```text
commit → required CI → contracts/tests → SBOM → signature/provenance
       → immutable OCI digest → customer overlay → promotion → rollback digest
```

- keine Production-Images mit `latest`;
- keine manuellen Source-Builds auf dem Server nach Cutover;
- keine Klartext-Secrets in Doku, Git, Image, Logs oder Customer Overlay;
- vollständiger SSOT:
  `docs/business/gateway-integration/repository-delivery-boundary.md`.

## Website Deployment (leanctx.com)

> **LEGACY CURRENT-STATE RUNBOOK — zur Ablösung in W0.** Ziel ist ein separates
> privates `leanctx-web`-Repository mit required CI, immutable Image Digest und
> Promotion. Die folgenden rsync-/Server-Build-Schritte dürfen nach dem Cutover
> nicht mehr als Production-Pfad verwendet werden.

### Prerequisites
- SSH access to `administrator@185.142.213.170` via `~/.ssh/pounce_server`
- Server sudo password: stored in `server_access.md` (not in repo)
- Node.js >= 22.12.0 (use `/opt/homebrew/bin/node` on macOS)

### Steps

#### 1. Build Website
```bash
cd /Users/yvesgugger/Documents/Privat/Projects/lean-ctx/website
PATH="/opt/homebrew/bin:$PATH" npm run build
```

#### 2. Sync to Server
```bash
cd /Users/yvesgugger/Documents/Privat/Projects/lean-ctx
rsync -az --delete \
  -e "ssh -i ~/.ssh/pounce_server" \
  --exclude ".git" --exclude "node_modules" --exclude "website/node_modules" \
  --exclude "website/dist" --exclude "dist" --exclude ".env" --exclude "deploy.sh" \
  --exclude "rust/target" \
  ./ administrator@185.142.213.170:/home/administrator/lean-ctx/
```

#### 3. Build Docker Image
```bash
ssh -i ~/.ssh/pounce_server administrator@185.142.213.170 \
  "cd /home/administrator/lean-ctx && \
   sudo docker build -t lean-ctx-web -f Dockerfile.web ."
```

#### 4. Restart Container
```bash
ssh -i ~/.ssh/pounce_server administrator@185.142.213.170 \
  "sudo docker stop lean-ctx-web 2>/dev/null; \
   sudo docker rm lean-ctx-web 2>/dev/null; \
   sudo docker run -d \
     --name lean-ctx-web \
     --network coolify \
     --restart unless-stopped \
     --label traefik.enable=true \
     --label 'traefik.http.routers.lean-ctx.rule=Host(\`leanctx.com\`) || Host(\`www.leanctx.com\`)' \
     --label traefik.http.routers.lean-ctx.entrypoints=websecure \
     --label traefik.http.routers.lean-ctx.tls=true \
     --label traefik.http.routers.lean-ctx.tls.certresolver=letsencrypt \
     --label traefik.http.services.lean-ctx.loadbalancer.server.port=80 \
     lean-ctx-web"
```

#### 5. Verify
```bash
command curl -s -o /dev/null -w '%{http_code}' https://leanctx.com/
# Should return 200
```

### Troubleshooting

- **Traefik Host rules empty**: Backtick escaping in SSH commands. Use `\`` not `\\\\\\\``.
- **Node.js too old**: Use `PATH="/opt/homebrew/bin:$PATH"` before build commands.
- **Astro "Unexpected &"**: Wrap PowerShell code in template literals `{` `` ` `` `}` inside `<pre><code>`.

---

## Server Details

| Property | Value |
|----------|-------|
| IP | `185.142.213.170` |
| User | `administrator` |
| SSH Key | `~/.ssh/pounce_server` |
| Docker Network | `coolify` |
| Reverse Proxy | Traefik |
| TLS | Let's Encrypt |
| Container Name | `lean-ctx-web` |
| Image | `lean-ctx-web` (nginx:alpine) |
| Port | 80 (internal), 443 (external via Traefik) |

---

## Public Demo (demo.leanctx.com) — LeanCTX Enterprise / AI Value Gate

Live seit 2026-07-06 (GitLab enterprise#81–#85). Echte Gateway-Instanz, kein Mock.

| Surface | URL | Auth |
|---------|-----|------|
| Landing + Value-Report | `https://demo.leanctx.com` (`/report/value-report.html`) | öffentlich |
| Admin-Konsole | `https://console.demo.leanctx.com` | publiziertes Demo-Admin-Token (API read-only) |
| Persönliche Sicht | `https://me.demo.leanctx.com/me` | publizierter Guest-Key |

- **Quelle**: `lean-ctx-deploy` Repo, Ordner `demo/` (compose, config, seeder, landing, traefik, Runbook `demo/README.md`).
- **Server**: `/data/leanctx/demo/stack` auf pounce-server; Container `leanctx-demo-{gateway,postgres,ollama,seeder,reporter,landing}` im `coolify`-Netz + internem `demo-internal`-Netz.
- **Traefik**: File-Provider-Config `/data/coolify/proxy/dynamic/leanctx-demo.yml`. Inference-Pfade sind öffentlich NICHT geroutet — publizierte Keys können das LLM nicht fremdnutzen.
- **Secrets**: `.env` + `demo-keys.json` nur auf dem Server (`/data/leanctx/demo/stack/demo/`), nicht im Git.
- **Deploy-Update**: lokal ändern → `rsync` nach `/data/leanctx/demo/stack/demo/` → `docker compose up -d --build <service>`. Gateway-Image: `lean-ctx-gateway:3.9.1-patch1` (= Tag v3.9.1 + cherry-pick `03cbd212ac` acme-Tooltip; Build-Clone `/data/leanctx/demo/build/lean-ctx`).
- **Landing = Sales-Seite** (seit 2026-07-06): Hero-Wertversprechen, Live-KPI-Streifen (`/live/usage` = nginx-Proxy auf GET-only `/api/admin/usage`, Token server-seitig in `rendered/default.conf`), «30-day proof»-Pilotangebot, Security/Finance-Facts. Beide Templates in `demo/landing/`, gerendert via `render-landing.sh <admin-token> <guest-key>`.
- **Traffic**: Seeder hält Dashboards gefüllt (Initial-Burst 150 Requests abgeschlossen, Trickle ~12 req/10 min). Ollama auf 12 CPUs gecappt.
- **Lokale Modellkosten**: `local_shadow_rate_per_mtok = 0.25` explizit in `config.toml` — lokale Inferenz nie $0, Methodik im Value-Report.

---

## Git Push

> **TARGET:** Kein direkter Push nach `main`. OSS-Änderungen laufen als GitHub
> PR mit Required Checks; GitLab `root/lean-ctx` wird automatisch gespiegelt.
> Die folgenden Befehle dokumentieren nur den bisherigen manuellen Zustand.

### GitHub
```bash
# Standard push (may fail for workflow files due to OAuth scope)
git push github main --tags

# SSH-based push (bypasses OAuth scope restriction)
GIT_SSH_COMMAND="ssh -i ~/.ssh/id_ed25519 -o IdentitiesOnly=yes" git push github main --tags
```

### GitLab
```bash
git push origin main --tags
```

**Note**: GitLab push may show exit code 1 despite success — this is lean-ctx's shell hook compressing the output. Check the actual message for "main -> main".
