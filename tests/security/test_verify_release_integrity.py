import hashlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "release_integrity", ROOT / "scripts/verify-release-integrity.py"
)
INTEGRITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INTEGRITY)


class ReleaseIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        self.artifact = self.directory / "lean-ctx-test.tar.gz"
        self.artifact.write_bytes(b"release artifact\n")
        self.write_fixture()

    def tearDown(self):
        self.temp.cleanup()

    def write_fixture(self):
        artifact_hash = hashlib.sha256(self.artifact.read_bytes()).hexdigest()
        sums = f"{artifact_hash}  {self.artifact.name}\n".encode()
        (self.directory / "SHA256SUMS").write_bytes(sums)
        (self.directory / "SBOM.txt").write_text("lean-ctx 3.9.14 MIT\n")
        manifest = {
            "schema_version": "leanctx.release-manifest/v1",
            "tag": "v3.9.14",
            "commit": "a" * 40,
            "timestamp": "2026-07-29T05:00:00Z",
            "artifacts": {self.artifact.name: {"sha256": artifact_hash,
                                                 "size": self.artifact.stat().st_size}},
            "sbom_sha256": hashlib.sha256((self.directory / "SBOM.txt").read_bytes()).hexdigest(),
            "checksums_sha256": hashlib.sha256(sums).hexdigest(),
        }
        (self.directory / "release-manifest.json").write_text(json.dumps(manifest))

    def test_valid_manifest_and_artifacts_verify(self):
        report = INTEGRITY.verify_release("v3.9.14", self.directory)
        self.assertTrue(report["verified"])
        self.assertEqual(report["errors"], [])

    def test_manifest_schema_validation_rejects_unknown_field(self):
        manifest = json.loads((self.directory / "release-manifest.json").read_text())
        manifest["unexpected"] = True
        with self.assertRaises(INTEGRITY.GateError):
            INTEGRITY.validate_manifest(manifest)

    def test_artifact_digest_mismatch_fails(self):
        self.artifact.write_bytes(b"tampered\n")
        report = INTEGRITY.verify_release("v3.9.14", self.directory)
        self.assertFalse(report["verified"])
        self.assertIn("artifact digest mismatch", report["errors"][0])

    def test_checksum_parser_handles_gnu_format_and_rejects_paths(self):
        digest = "b" * 64
        self.assertEqual(INTEGRITY.parse_checksums(f"{digest} *asset.tar.gz\n".encode()),
                         {"asset.tar.gz": digest})
        with self.assertRaises(INTEGRITY.GateError):
            INTEGRITY.parse_checksums(f"{digest}  ../asset.tar.gz\n".encode())

    def test_sbom_parser_returns_package_and_license(self):
        entries = INTEGRITY.parse_sbom(b"lean-ctx v3.9.14 MIT\n")
        self.assertEqual(entries, [{"package": "lean-ctx v3.9.14", "license": "MIT"}])
        with self.assertRaises(INTEGRITY.GateError):
            INTEGRITY.parse_sbom(b"\n")

    def test_download_uses_http_and_fetches_checksum_listed_artifacts(self):
        digest = "c" * 64
        responses = {
            "SHA256SUMS": f"{digest}  lean-ctx-test.tar.gz\n".encode(),
            "SBOM.txt": b"package license\n",
            "release-manifest.json": b"{}\n",
            "lean-ctx-test.tar.gz": b"archive\n",
        }

        def fake_urlopen(request, timeout):
            name = request.full_url.rsplit("/", 1)[1]
            return io.BytesIO(responses[name])

        with mock.patch.object(INTEGRITY.urllib.request, "urlopen", side_effect=fake_urlopen):
            report = INTEGRITY.download_release("v3.9.14", self.directory / "download", "owner/repo")
        self.assertEqual(report["downloaded"], [
            "SHA256SUMS", "SBOM.txt", "release-manifest.json", "lean-ctx-test.tar.gz"
        ])
        self.assertEqual((self.directory / "download" / "lean-ctx-test.tar.gz").read_bytes(), b"archive\n")


if __name__ == "__main__":
    unittest.main()
