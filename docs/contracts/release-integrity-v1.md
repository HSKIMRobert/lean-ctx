# Release Integrity v1

`leanctx.release-manifest/v1` binds a published lean-ctx release to its source
tag and commit. The release job builds artifacts from the tagged source, writes
`SHA256SUMS`, emits the locked Rust dependency inventory in `SBOM.txt`, and
then writes `release-manifest.json`. All three metadata files are release
assets alongside the archives.

## Artifact chain

```
source commit → build artifacts → SHA256SUMS → SBOM.txt → release-manifest.json
```

`SHA256SUMS` records the SHA-256 digest of every release archive. `SBOM.txt`
is Cargo's locked dependency tree, one package-and-license record per line. The
manifest records the expected tag, full source commit, creation timestamp,
per-artifact digest and size, plus the SHA-256 digests of the SBOM and checksum
file.

## Manifest schema

The manifest has exactly these fields:

- `schema_version`: `leanctx.release-manifest/v1`
- `tag`: the Git tag used for the release
- `commit`: 40-character source commit ID
- `timestamp`: UTC ISO-8601 timestamp ending in `Z`
- `artifacts`: map of archive name to `{ "sha256", "size" }`
- `sbom_sha256`: SHA-256 of `SBOM.txt`
- `checksums_sha256`: SHA-256 of `SHA256SUMS`

## Downstream verification

Download all release assets and run:

```bash
python3 scripts/verify-release-integrity.py verify \
  --tag v3.9.14 --dir ./release-files
```

For a clean offline directory, first retrieve the metadata and checksum-listed
artifacts from the public GitHub release, then repeat verification:

```bash
python3 scripts/verify-release-integrity.py download \
  --tag v3.9.14 --dir ./release-files
python3 scripts/verify-release-integrity.py verify \
  --tag v3.9.14 --dir ./release-files
```

The verifier emits a JSON report, checks the requested tag, validates the
closed manifest schema, hashes the SBOM and checksum file, and checks every
archive's digest and size against both metadata records.

## Failure handling

Any missing file, malformed metadata, unexpected artifact set, tag mismatch,
or digest/size mismatch returns exit code 1. Consumers must reject the release,
delete the untrusted download directory, and obtain assets again from the
release URL. This contract does not provide a signing or identity assertion;
sigstore/cosign attestation is intentionally outside v1.
