import contextlib
import importlib.util
import io
import json
import unittest
from pathlib import Path
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]


def load_module(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


APPLY = load_module("branch_protection_apply", "scripts/apply-branch-protection.py")
VERIFY = load_module("branch_protection_verify", "scripts/verify-branch-protection.py")
POLICY_PATH = ROOT / "security/branch-protection-policy-v1.json"


class Response:
    def __init__(self, body, status=200):
        self.body = json.dumps(body).encode()
        self.status = status

    def read(self):
        return self.body

    def __enter__(self):
        return self

    def __exit__(self, *unused):
        return False


class BranchProtectionTests(unittest.TestCase):
    def test_policy_loading_and_schema_validation(self):
        policy = APPLY.load_policy(POLICY_PATH)
        self.assertEqual(policy["schema_version"], "leanctx.branch-protection-policy/v1")
        self.assertIsNone(policy["github"]["branches"]["main"]["required_pull_request_reviews"])
        APPLY.validate_scanner_sources(ROOT, policy)

        invalid = dict(policy)
        invalid["schema_version"] = "invalid/v1"
        temporary = ROOT / ".branch-protection-invalid.json"
        try:
            temporary.write_text(json.dumps(invalid))
            with self.assertRaises(APPLY.GateError):
                APPLY.load_policy(temporary)
        finally:
            temporary.unlink(missing_ok=True)

    def test_github_api_request_construction(self):
        policy = APPLY.load_policy(POLICY_PATH)
        rule = policy["github"]["branches"]["main"]
        request = APPLY.github_request("owner name", "repo/name", "main/next", rule)
        self.assertEqual(request.get_method(), "PUT")
        self.assertEqual(
            request.full_url,
            "https://api.github.com/repos/owner%20name/repo%2Fname/branches/main%2Fnext/protection",
        )
        payload = json.loads(request.data)
        self.assertEqual(payload["required_status_checks"], rule["required_status_checks"])
        self.assertIsNone(payload["restrictions"])
        self.assertEqual(request.get_header("Content-type"), "application/json")

    def test_drift_detection(self):
        expected = APPLY.load_policy(POLICY_PATH)["github"]["branches"]["main"]
        compliant = {
            "enforce_admins": {"enabled": True},
            "required_status_checks": {"strict": False, "contexts": list(reversed(expected["required_status_checks"]["contexts"]))},
            "required_pull_request_reviews": None,
            "required_signatures": {"enabled": False},
            "allow_force_pushes": {"enabled": False},
            "allow_deletions": {"enabled": False},
            "required_linear_history": {"enabled": False},
        }
        self.assertEqual(VERIFY.drift(expected, compliant), [])
        compliant["allow_deletions"] = {"enabled": True}
        differences = VERIFY.drift(expected, compliant)
        self.assertEqual(differences, [{"field": "allow_deletions", "expected": False, "actual": True}])

    def test_dry_run_output_format(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = APPLY.main(["--root", str(ROOT), "dry-run", "--target", "github"])
        report = json.loads(output.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(report["schema_version"], "leanctx.branch-protection-dry-run/v1")
        self.assertEqual([change["provider"] for change in report["changes"]], ["github"])

    def test_http_calls_are_mocked_for_apply_and_verify(self):
        policy = APPLY.load_policy(POLICY_PATH)
        with patch.object(APPLY.urllib.request, "urlopen", return_value=Response({"ok": True}, 200)) as open_apply:
            report = APPLY.apply(policy, "github", "token", None)
        self.assertEqual(report["changed"], [{"provider": "github", "branch": "main", "status": 200}])
        apply_request = open_apply.call_args.args[0]
        self.assertEqual(apply_request.get_header("Authorization"), "Bearer token")

        github_response = {
            "enforce_admins": {"enabled": True},
            "required_status_checks": {"strict": False, "contexts": policy["github"]["branches"]["main"]["required_status_checks"]["contexts"]},
            "required_pull_request_reviews": None,
            "required_signatures": {"enabled": False},
            "allow_force_pushes": {"enabled": False},
            "allow_deletions": {"enabled": False},
            "required_linear_history": {"enabled": False},
        }
        with patch.object(VERIFY.urllib.request, "urlopen", return_value=Response(github_response)) as open_verify:
            report = VERIFY.verify(policy, "token")
        self.assertTrue(report["compliant"])
        self.assertEqual(open_verify.call_count, 1)


if __name__ == "__main__":
    unittest.main()
