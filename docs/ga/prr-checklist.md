# Production Readiness Review Checklist

## Purpose

Use this checklist to approve a lean-ctx production release or deployment.
Every item has an observable command or inspection. Record command output,
approver, date, release tag, and any approved exception with release evidence.

## Review metadata

| Field | Record |
| --- | --- |
| Release tag | |
| Source commit | |
| Environment | |
| Reviewer | |
| Review date | |
| Evidence location | |

Do not approve a deployment while any required item is unchecked. Run commands
from the repository root unless a command explicitly changes directory.

## 1. Binary and release integrity

- [ ] The macOS binary has a valid signature.

  ```bash
  codesign -v ~/.local/bin/lean-ctx
  ```

- [ ] The installed binary reports the approved release version.

  ```bash
  lean-ctx --version
  ```

  Compare the output exactly with the approved release tag after removing its
  leading `v`, if present.

- [ ] `SHA256SUMS` is attached to the published release and covers every binary.

  ```bash
  gh release view vX.Y.Z --repo yvgude/lean-ctx --json assets
  ```

- [ ] Downloaded release artifacts match the published checksums.

  ```bash
  sha256sum -c SHA256SUMS
  ```

- [ ] An SBOM is attached to the published release.

  ```bash
  gh release view vX.Y.Z --repo yvgude/lean-ctx --json assets
  ```

  Confirm the asset list includes `SBOM.txt` or the approved SBOM format.

- [ ] The binary or release manifest has a verifiable Cosign signature.

  ```bash
  cosign verify-blob --bundle release-manifest.sigstore.json \
    --certificate release-manifest.pem --signature release-manifest.sig \
    release-manifest.json
  ```

- [ ] The release manifest identifies the approved source commit.

  ```bash
  python scripts/verify-release-integrity.py verify \
    --tag vX.Y.Z --dir ./release-files
  ```

## 2. Process health

- [ ] A proxy process is running.

  ```bash
  pgrep -f 'lean-ctx.*proxy'
  ```

- [ ] The macOS LaunchAgent is loaded.

  ```bash
  launchctl list | grep leanctx
  ```

- [ ] The installed LaunchAgent enables automatic recovery.

  ```bash
  plutil -p ~/Library/LaunchAgents/com.leanctx.proxy.plist | grep KeepAlive
  ```

  Confirm the value is `true`.

- [ ] The proxy health endpoint responds successfully.

  ```bash
  curl --fail --silent "http://127.0.0.1:${PORT}/health"
  ```

  Set `PORT` to the configured proxy port before running this check.

- [ ] The runtime diagnostic reports no errors.

  ```bash
  lean-ctx doctor
  ```

- [ ] A restart preserves a healthy proxy.

  ```bash
  lean-ctx restart && lean-ctx status
  ```

- [ ] The KeepAlive crash-recovery evidence passes.

  ```bash
  scripts/chaos-restart-test.sh
  ```

## 3. Security

- [ ] PathJail has at least one configured allowed path.

  ```bash
  lean-ctx config show | grep -E '^[[:space:]]*allow_paths'
  ```

- [ ] The shell command allowlist is configured.

  ```bash
  lean-ctx config show | grep -E '^[[:space:]]*shell_allowlist'
  ```

- [ ] Repository history has no disallowed secrets.

  ```bash
  python3 scripts/history-policy-gate.py
  ```

- [ ] Required branch protection is active on the release branch.

  ```bash
  python3 scripts/verify-branch-protection.py
  ```

- [ ] Required secrets are within their rotation period.

  ```bash
  python3 scripts/check-secret-expiry.py
  ```

- [ ] Deployment configuration contains no committed runtime secret values.

  ```bash
  git grep -nE '(_TOKEN|_PASSWORD|DATABASE_URL)[[:space:]]*=[^$[:space:]]' \
    -- deploy/self-deploy
  ```

  The command must return no values other than variable references or comments.

- [ ] The local deployment environment file is not tracked.

  ```bash
  git check-ignore -q deploy/self-deploy/.env
  ```

## 4. Data integrity

- [ ] Savings-ledger token totals are internally consistent.

  ```bash
  lean-ctx stats
  ```

  Confirm reported input, output, and saved token totals reconcile with the
  ledger export for the review interval.

- [ ] Committed files contain no compression omission markers.

  ```bash
  git grep -n '\[lean-ctx: omitted' "$(git rev-parse HEAD)" -- .
  ```

  The command must return no matches.

- [ ] The active configuration parses successfully.

  ```bash
  lean-ctx config show
  ```

- [ ] The proxy can read its configured state without repair warnings.

  ```bash
  lean-ctx doctor
  ```

- [ ] Backup media passes checksum verification before production changes.

  ```bash
  sha256sum -c /path/to/lean-ctx-backup.tar.gz.sha256
  ```

## 5. Performance

- [ ] Aggregate compression savings are at least 30 percent.

  ```bash
  lean-ctx stats
  ```

  Record the reported savings percentage; it must be `>= 30%`.

- [ ] At least one CEP session is warm.

  ```bash
  lean-ctx sessions list
  ```

  Confirm the output reports one or more active or persisted sessions.

- [ ] Proxy overhead is below 50 ms per request under the intended workload.

  ```bash
  scripts/slo-check.sh
  ```

  Preserve the latency measurement and confirm it is `< 50ms`.

- [ ] The health endpoint remains responsive during the measurement.

  ```bash
  curl --fail --silent "http://127.0.0.1:${PORT}/health"
  ```

## 6. Deployment

- [ ] Installation succeeds on a fresh supported machine.

  ```bash
  curl -fsSL https://leanctx.com/install.sh | sh
  lean-ctx doctor
  ```

- [ ] An upgrade from the previous released version preserves configuration and
  starts a healthy proxy.

  ```bash
  lean-ctx --version && lean-ctx status && lean-ctx doctor
  ```

  Record the previous version, target version, and outputs in release evidence.

- [ ] A rollback is rehearsed with the prior verified binary.

  ```bash
  lean-ctx stop
  cp /path/to/previous/lean-ctx ~/.local/bin/lean-ctx
  launchctl load ~/Library/LaunchAgents/com.leanctx.proxy.plist
  lean-ctx status
  ```

- [ ] The self-deployment has healthy Compose services.

  ```bash
  (cd deploy/self-deploy && docker compose up -d && ./verify.sh)
  ```

- [ ] Self-deployment evidence records a successful gateway health check.

  ```bash
  test "$(jq -r .pass security/evidence/g10-self-deploy-evidence.json)" = true
  ```

## 7. Documentation

- [ ] The GA documentation directory contains all approved GA guides.

  ```bash
  ls docs/ga/
  ```

  Confirm it lists the ten baseline GA guides plus this PRR checklist:
  installation, upgrade, monitoring, API, developer, administrator,
  disaster-recovery, release, and runbook documentation.

- [ ] The current version is represented in the changelog.

  ```bash
  grep -F "$(lean-ctx --version | awk '{print $NF}')" CHANGELOG.md
  ```

- [ ] API documentation refers to live, supported endpoints.

  ```bash
  grep -nE '/health|/metrics' docs/ga/api-guide.md
  ```

- [ ] Operations documentation includes a verified rollback path.

  ```bash
  grep -n 'Rollback' docs/ga/release-checklist.md docs/ga/upgrade-guide.md
  ```

## Approval

- [ ] All required sections above have recorded, passing evidence.
- [ ] Exceptions are documented with an owner, expiry date, and approver.
- [ ] The designated production owner has approved the release or deployment.

Record final approval with the release tag, source commit, environment, and
evidence location in the change-management system.
