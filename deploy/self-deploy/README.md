# lean-ctx Self-Deployment — 2nd Fork-Free Deployment (G10)

This directory deploys lean-ctx as its own Docker Compose customer. It proves
the deployment design works without a source-code fork (Completion Rule §10,
CR2).

## Prerequisites

- Docker Engine and the Docker Compose plugin.
- Access to the selected `LEANCTX_IMAGE` registry image.
- Three generated, non-empty secret values.

## Quick start

```bash
cp .env.example .env
openssl rand -hex 32
# Put a different generated value in each required secret variable in .env.
docker compose up -d
./verify.sh
```

The local `.env` file is intentionally ignored by Git. Do not commit it or
paste its values into evidence files.

## Architecture

The configuration follows `lean-ctx-deploy-template/compose/docker-compose.yml`
but remains standalone: it defines PostgreSQL, gateway dependencies, persistent
storage, health checks, and runtime values through environment variables.

| Service | Purpose |
| --- | --- |
| `postgres` | Persistent gateway database with a health check |
| `gateway` | lean-ctx gateway exposed on port 19187 |

The named `pgdata` volume keeps database state across container recreation.

## Verification

```bash
docker compose ps
curl --fail http://localhost:19187/health
lean-ctx status
./verify.sh
```

`verify.sh` writes `security/evidence/g10-self-deploy-evidence.json` only after
checking gateway health. A non-200 result makes the verification fail.

## Lifecycle

```bash
docker compose logs --follow gateway
docker compose down
docker compose up -d
```

Use `docker compose down` without `--volumes` for normal restarts. Removing
`pgdata` destroys persistent state and requires an approved recovery procedure.
