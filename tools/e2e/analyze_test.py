"""Unit tests for the e2e analyzer (tools/e2e/analyze.py)."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import analyze

HEADER = {"t_us": 0, "pid": 1, "tid": 1, "event": "trace_open", "schema_version": 1, "marker": "TAIGI_E2E_TRACE_V1"}
SCENARIO = {
    "id": "demo",
    "steps": [{"type": "text", "value": "tai"}],
    "expect": {"committed": "臺"},
}
BUDGETS = {"engine_request_us": {"*": {"*": {"p95": 20000, "max": 50000}}}}


def request(dur_us: int, *, error: int = 0, domain: str = "composing", tag: int = 11) -> dict:
    return {"event": "engine_request", "domain": domain, "method_tag": tag, "error": error, "dur_us": dur_us}


class AnalyzeTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.scenarios = self.root / "scenarios"
        self.scenarios.mkdir()
        self.run = self.root / "run"

    def write_scenario(self, scenario: dict) -> None:
        (self.scenarios / f"{scenario['id']}.json").write_text(json.dumps(scenario), encoding="utf-8")

    def write_result(self, result: dict, events: list[dict], *, platform: str = "linux", scenario: str = "demo") -> None:
        directory = self.run / platform / scenario
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "result.json").write_text(json.dumps(result), encoding="utf-8")
        (directory / "trace.jsonl").write_text("".join(json.dumps(e) + "\n" for e in events), encoding="utf-8")

    def analyze(self, budgets: dict = BUDGETS, baseline: dict | None = None) -> analyze.Report:
        proto_dir = analyze.REPO_ROOT / "engine" / "protos" / "proto"
        return analyze.analyze(self.run, self.scenarios, budgets, baseline, proto_dir)

    def test_matching_commit_passes_with_latency_summary(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "ran", "observed_text": "臺"}, [HEADER, request(900), request(1100)])
        report = self.analyze()
        self.assertEqual([o.status for o in report.outcomes], ["passed"])
        self.assertFalse(report.is_failing)
        # trace: composing tag 11 = `append` (composing.proto `oneof method`).
        self.assertEqual(report.latency["linux"]["composing.append"], {"count": 2, "p50_us": 900, "p95_us": 1100, "max_us": 1100})

    def test_wrong_commit_fails_with_both_texts(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "ran", "observed_text": "tai"}, [HEADER])
        report = self.analyze()
        self.assertEqual(report.outcomes[0].status, "failed")
        self.assertIn("'tai'", report.outcomes[0].detail)
        self.assertIn("'臺'", report.outcomes[0].detail)
        self.assertTrue(report.is_failing)

    def test_panic_error_and_reject_are_bug_findings(self) -> None:
        self.write_scenario(SCENARIO)
        events = [
            HEADER,
            {"event": "engine_panic", "domain": "lexicon", "method_tag": 12},
            request(10, error=3),
            {"event": "adapter_reject", "reason": "oversize", "req_bytes": 3000000},
        ]
        self.write_result({"status": "ran", "observed_text": "臺"}, events)
        report = self.analyze()
        self.assertEqual([f.kind for f in report.findings], ["bug", "bug", "bug"])
        self.assertTrue(report.is_failing)

    def test_missing_header_is_a_trace_finding(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "ran", "observed_text": "臺"}, [request(10)])
        report = self.analyze()
        self.assertEqual([f.kind for f in report.findings], ["trace"])
        self.assertTrue(report.is_failing)

    def test_skipped_platform_does_not_fail_the_run(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "skipped", "reason": "box powered off"}, [], platform="windows")
        report = self.analyze()
        self.assertEqual((report.outcomes[0].status, report.outcomes[0].detail), ("skipped", "box powered off"))
        self.assertFalse(report.is_failing)

    def test_over_budget_p95_and_max_are_perf_findings(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "ran", "observed_text": "臺"}, [HEADER, request(60000)])
        report = self.analyze()
        messages = [f.message for f in report.findings if f.kind == "perf"]
        self.assertEqual(len(messages), 2, messages)
        self.assertTrue(report.is_failing)

    def test_platform_op_budget_overrides_the_wildcard(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "ran", "observed_text": "臺"}, [HEADER, request(30000)])
        budgets = {"engine_request_us": {"*": {"*": {"p95": 20000}}, "linux": {"composing.append": {"p95": 40000}}}}
        self.assertEqual([f for f in self.analyze(budgets).findings if f.kind == "perf"], [])

    def test_baseline_regression_needs_ratio_and_absolute_delta(self) -> None:
        self.write_scenario(SCENARIO)
        self.write_result({"status": "ran", "observed_text": "臺"}, [HEADER, request(9000)])
        slower = {"latency": {"linux": {"composing.append": {"p95_us": 3000}}}}
        noise = {"latency": {"linux": {"composing.append": {"p95_us": 8000}}}}
        self.assertEqual(len([f for f in self.analyze(baseline=slower).findings if f.kind == "perf"]), 1)
        self.assertEqual([f for f in self.analyze(baseline=noise).findings if f.kind == "perf"], [])

    def test_first_hanji_candidate_unverified_without_candidates_event(self) -> None:
        scenario = {"id": "demo", "expect": {"first_hanji_candidate": {"hanji": "去矣", "tl": "khì--ah"}}}
        self.write_scenario(scenario)
        self.write_result({"status": "ran", "observed_text": ""}, [HEADER])
        report = self.analyze()
        self.assertEqual(report.outcomes[0].status, "passed")
        self.assertEqual([f.kind for f in report.findings], ["unverified"])
        self.assertFalse(report.is_failing)

    def test_first_hanji_candidate_skips_the_literal_slot(self) -> None:
        scenario = {"id": "demo", "expect": {"first_hanji_candidate": {"hanji": "去矣", "tl": "khì--ah"}}}
        self.write_scenario(scenario)
        candidates = {
            "event": "candidates",
            "items": [{"hanji": "khi--ah", "tl": "khi--ah"}, {"hanji": "隙", "tl": "khiah"}],
        }
        self.write_result({"status": "ran", "observed_text": ""}, [HEADER, candidates])
        report = self.analyze()
        self.assertEqual(report.outcomes[0].status, "failed")
        self.assertIn("隙", report.outcomes[0].detail)


class MatchesCellTests(unittest.TestCase):
    def test_reading_only_matches_any_hanji_cell_with_that_reading(self) -> None:
        wanted = {"tl": "tâi-gí-sī-kái"}
        self.assertTrue(analyze.matches_cell({"hanji": "台語是解", "tl": "tâi-gí-sī-kái"}, wanted))
        # The §34 literal slot carries no hanji and never matches.
        self.assertFalse(analyze.matches_cell({"hanji": "", "tl": "tâi-gí-sī-kái"}, wanted))

    def test_named_hanji_must_match_too(self) -> None:
        wanted = {"hanji": "鵝仔是", "tl": "gô-á-sī"}
        self.assertFalse(analyze.matches_cell({"hanji": "餓仔是", "tl": "gô-á-sī"}, wanted))


class ScenarioFileTests(unittest.TestCase):
    def test_repo_scenarios_are_well_formed(self) -> None:
        scenarios = analyze.load_scenarios(analyze.REPO_ROOT / "e2e" / "scenarios")
        self.assertGreaterEqual(len(scenarios), 3)

    def test_unknown_step_type_names_the_file(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        directory = Path(temporary.name)
        (directory / "bad.json").write_text(json.dumps({"id": "bad", "steps": [{"type": "setText"}]}), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "bad.json.*setText"):
            analyze.load_scenarios(directory)


class MethodNameTests(unittest.TestCase):
    def test_names_come_from_the_repo_protos(self) -> None:
        names = analyze.load_method_names(analyze.REPO_ROOT / "engine" / "protos" / "proto")
        # trace: phonetics.proto PhoneticsRequest `strip_tone = 11`;
        # composing.proto ComposingRequest `fetch_at_pos = 31`.
        self.assertEqual(names[("phonetics", 11)], "strip_tone")
        self.assertEqual(names[("composing", 31)], "fetch_at_pos")


if __name__ == "__main__":
    unittest.main()
