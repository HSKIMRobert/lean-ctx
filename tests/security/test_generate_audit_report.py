import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("audit_report", ROOT / "scripts/generate-audit-report.py")
REPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REPORT)


class GenerateAuditReportTests(unittest.TestCase):
    def report(self, findings=None, run_at=None):
        value = {
            "schema_version": "leanctx.full-history-evidence/v1",
            "audited_commit": "a" * 40,
            "counts": {"commits": 12, "objects": 30, "findings": len(findings or [])},
            "findings": findings or [],
        }
        if run_at:
            value["audit_run_at"] = run_at
        return value

    def schedule(self):
        return {
            "schema_version": "leanctx.audit-schedule/v1",
            "audits": [{"id": "history-full"}],
            "retention": {"audit_reports_days": 90},
            "scanner_source_paths": [],
            "scanner_source_sha256": {},
        }

    def finding(self, identifier, rule, scanner):
        return {"id": identifier, "rule": rule, "scanner": scanner, "path": "example", "object": "b" * 40}

    def test_summarize_output_format(self):
        output = REPORT.markdown_summary(self.report(), self.schedule())
        self.assertIn("# Full History Audit Summary", output)
        self.assertIn("| Total commits scanned | 12 |", output)
        self.assertIn("Compliance status: SCHEDULED", output)

    def test_markdown_renders_categories_and_next_run(self):
        finding = self.finding("one", "IP001", "full-path")
        output = REPORT.markdown_summary(self.report([finding], "2026-07-27T05:00:00Z"), self.schedule())
        self.assertIn("| Forbidden path findings | 1 |", output)
        self.assertIn("Next scheduled: 2026-08-03T04:00:00Z", output)
        self.assertIn("## Risk assessment: YELLOW", output)

    def test_baseline_comparison_detects_new_findings(self):
        existing = self.finding("known", "IP001", "full-path")
        new = self.finding("new", "SEC001", "full-secret")
        delta = REPORT.compare_reports(self.report([existing, new]), self.report([existing]))
        self.assertEqual(delta["new_finding_ids"], ["new"])
        self.assertEqual(delta["new_findings_count"], 1)

    def test_baseline_comparison_accepts_existing_finding(self):
        finding = self.finding("known", "SEC001", "full-secret")
        delta = REPORT.compare_reports(self.report([finding]), self.report([finding]))
        self.assertEqual(delta["new_findings_count"], 0)

    def test_risk_classification(self):
        self.assertEqual(REPORT.risk_classification(self.report()), "GREEN")
        self.assertEqual(REPORT.risk_classification(self.report([self.finding("path", "IP001", "full-path")])), "YELLOW")
        self.assertEqual(REPORT.risk_classification(self.report([self.finding("secret", "SEC001", "full-secret")])), "RED")
        self.assertEqual(REPORT.risk_classification(self.report([self.finding("other", "POL001", "scanner")])), "RED")

    def test_history_reads_inline_fixture_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            reports = Path(directory)
            (reports / "one.json").write_text(json.dumps(self.report()))
            result = REPORT.history(reports)
        self.assertEqual(result["reports"], [{"report": "one.json", "commits": 12, "findings": 0, "risk": "GREEN"}])


if __name__ == "__main__":
    unittest.main()
