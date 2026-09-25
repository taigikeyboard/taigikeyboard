"""Turn e2e run output (per-scenario trace + observed text) into a bug / perf report.

Contract: docs/architecture/e2e-trace-schema.md (trace events, scenario format,
run layout). Stdlib only, so every driver host can run it.

Usage:
  python3 tools/e2e/analyze.py --run <run-dir> [--scenarios e2e/scenarios]
      [--budgets e2e/budgets.json] [--baseline <old report.json>]
Writes <run-dir>/report.md and <run-dir>/report.json; exit 1 when any
scenario fails or any bug / perf finding exists.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SUPPORTED_SCHEMA_VERSION = 1
TRACE_MARKER = "TAIGI_E2E_TRACE_V1"

STEP_TYPES = {"text", "key", "pick"}
# Platform-neutral key names; each driver translates them to its own keys.
KEY_NAMES = {"enter", "space", "backspace", "escape", "capslock", *"0123456789"}
EXPECT_KEYS = {"committed", "first_hanji_candidate"}

# A p95 counts as a regression only past both bounds, so tiny ops' noise and
# small absolute drifts stay quiet.
BASELINE_REGRESSION_RATIO = 1.5
BASELINE_REGRESSION_MIN_DELTA_US = 2000


@dataclass
class Finding:
    kind: str  # bug | perf | trace | unverified
    platform: str
    scenario: str
    message: str


@dataclass
class ScenarioOutcome:
    platform: str
    scenario: str
    status: str  # passed | failed | skipped | error
    detail: str = ""


@dataclass
class Report:
    outcomes: list[ScenarioOutcome] = field(default_factory=list)
    findings: list[Finding] = field(default_factory=list)
    # platform -> op -> {"count", "p50_us", "p95_us", "max_us"}
    latency: dict[str, dict[str, dict[str, int]]] = field(default_factory=dict)

    @property
    def is_failing(self) -> bool:
        return any(o.status not in ("passed", "skipped") for o in self.outcomes) or any(
            f.kind != "unverified" for f in self.findings
        )


def load_scenarios(scenarios_dir: Path) -> dict[str, dict]:
    """Scenario id → scenario; a malformed file raises ValueError naming it."""
    scenarios = {}
    for path in sorted(scenarios_dir.glob("*.json")):
        scenario = json.loads(path.read_text(encoding="utf-8"))
        problems = []
        if scenario.get("id") != path.stem:
            problems.append(f"id {scenario.get('id')!r} must equal the file name {path.stem!r}")
        problems += [f"unknown step type {s.get('type')!r}" for s in scenario.get("steps", []) if s.get("type") not in STEP_TYPES]
        problems += [
            f"unknown key {s.get('value')!r}"
            for s in scenario.get("steps", [])
            if s.get("type") == "key" and s.get("value") not in KEY_NAMES
        ]
        problems += [f"unknown expectation {key!r}" for key in scenario.get("expect", {}) if key not in EXPECT_KEYS]
        if problems:
            raise ValueError(f"{path}: " + "; ".join(problems))
        scenarios[path.stem] = scenario
    return scenarios


def load_method_names(proto_dir: Path) -> dict[tuple[str, int], str]:
    """(domain, method tag) → `oneof method` field name, parsed from the .proto files."""
    names: dict[tuple[str, int], str] = {}
    message_re = re.compile(r"^message (\w+) \{(.*?)^\}", re.DOTALL | re.MULTILINE)
    oneof_re = re.compile(r"oneof method \{(.*?)\}", re.DOTALL)
    field_re = re.compile(r"^\s*\w+\s+(\w+)\s*=\s*(\d+);", re.MULTILINE)
    for proto in sorted(proto_dir.glob("*.proto")):
        for message, body in message_re.findall(proto.read_text(encoding="utf-8")):
            oneof = oneof_re.search(body)
            if oneof is None:
                continue
            # `NextWordRequest` → `nextword`: the trace domain names
            # (`dispatch::trace::classify`) follow the sub-request messages.
            domain = message.removesuffix("Request").lower()
            for name, tag in field_re.findall(oneof.group(1)):
                names[(domain, int(tag))] = name
    return names


def read_trace(run_dir: Path) -> tuple[list[dict], list[str]]:
    """All events from every *.jsonl in a scenario's run dir, plus parse problems."""
    events: list[dict] = []
    problems: list[str] = []
    for path in sorted(run_dir.glob("*.jsonl")):
        with path.open(encoding="utf-8") as lines:
            for number, line in enumerate(lines, 1):
                if not line.strip():
                    continue
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError as error:
                    problems.append(f"{path.name}:{number}: not JSON ({error.msg})")
    return events, problems


def check_trace_integrity(events: list[dict]) -> list[str]:
    headers = [e for e in events if e.get("event") == "trace_open"]
    if not headers:
        return ["no trace_open header — the build is not traced or the trace was never opened"]
    problems = []
    for header in headers:
        if header.get("schema_version") != SUPPORTED_SCHEMA_VERSION:
            problems.append(f"schema_version {header.get('schema_version')} unsupported")
        if header.get("marker") != TRACE_MARKER:
            problems.append(f"marker {header.get('marker')!r} is not {TRACE_MARKER}")
    return problems


def check_engine_errors(events: list[dict]) -> list[str]:
    problems = []
    for event in events:
        kind = event.get("event")
        if kind == "engine_panic":
            problems.append(f"engine panic in {event.get('domain')} tag {event.get('method_tag')}")
        elif kind == "adapter_reject":
            problems.append(f"adapter rejected a request ({event.get('reason')}, {event.get('req_bytes')} bytes)")
        elif kind == "engine_request" and event.get("error", 0) != 0:
            problems.append(
                f"engine error {event.get('error')} in {event.get('domain')} tag {event.get('method_tag')}"
                f" (req_id {event.get('req_id')})"
            )
    return problems


def is_hanji(text: str) -> bool:
    return any("㐀" <= ch <= "鿿" or "\U00020000" <= ch <= "\U0003134f" for ch in text)


def matches_cell(cell: dict, wanted: dict) -> bool:
    """A candidate cell against a scenario's `{tl[, hanji]}`: the displayed
    reading always; the hanji when the scenario names one (its source may
    give only the reading), else any hanji cell."""
    if cell.get("tl") != wanted["tl"] or not is_hanji(cell.get("hanji", "")):
        return False
    return "hanji" not in wanted or cell.get("hanji") == wanted["hanji"]


def check_expectations(scenario: dict, observed_text: str, events: list[dict]) -> tuple[list[str], list[str]]:
    """(failures, unverifiable) for the scenario's `expect` block."""
    failures: list[str] = []
    unverifiable: list[str] = []
    expect = scenario.get("expect", {})
    if "committed" in expect and observed_text != expect["committed"]:
        failures.append(f"committed {observed_text!r}, expected {expect['committed']!r}")
    wanted = expect.get("first_hanji_candidate")
    if wanted is not None:
        last_list = next((e for e in reversed(events) if e.get("event") == "candidates"), None)
        if last_list is None:
            unverifiable.append("first_hanji_candidate: no `candidates` event in the trace")
        else:
            first = next((c for c in last_list.get("items", []) if is_hanji(c.get("hanji", ""))), None)
            if first is None or not matches_cell(first, wanted):
                failures.append(f"first hanji candidate {first!r}, expected {wanted!r}")
    return failures, unverifiable


def percentile(sorted_values: list[int], fraction: float) -> int:
    index = max(0, math.ceil(fraction * len(sorted_values)) - 1)
    return sorted_values[index]


def collect_durations(events: list[dict], method_names: dict[tuple[str, int], str], durations: dict[str, list[int]]) -> None:
    """Append each `engine_request`'s `dur_us` to `durations[op]`."""
    for event in events:
        if event.get("event") != "engine_request":
            continue
        domain, tag = event.get("domain", "none"), event.get("method_tag", 0)
        op = f"{domain}.{method_names.get((domain, tag), f'tag{tag}')}"
        durations.setdefault(op, []).append(int(event.get("dur_us", 0)))


def summarize_latency(durations: dict[str, list[int]]) -> dict[str, dict[str, int]]:
    summary = {}
    for op, values in sorted(durations.items()):
        values.sort()
        summary[op] = {
            "count": len(values),
            "p50_us": percentile(values, 0.50),
            "p95_us": percentile(values, 0.95),
            "max_us": values[-1],
        }
    return summary


def check_budgets(platform: str, latency: dict[str, dict[str, int]], budgets: dict) -> list[Finding]:
    """Limits come from `engine_request_us[platform][op]`, most specific wins
    (`*` = any platform / any op): budgets are calibrated per device."""
    table = budgets.get("engine_request_us", {})
    findings = []
    for op, stats in latency.items():
        limits: dict[str, int] = {}
        for platform_key in ("*", platform):
            for op_key in ("*", op):
                limits.update(table.get(platform_key, {}).get(op_key, {}))
        for metric in ("p95", "max"):
            limit = limits.get(metric)
            value = stats[f"{metric}_us"]
            if limit is not None and value > limit:
                findings.append(Finding("perf", platform, "*", f"{op} {metric} {value}µs > budget {limit}µs"))
    return findings


def check_baseline(platform: str, latency: dict[str, dict[str, int]], baseline: dict) -> list[Finding]:
    previous = baseline.get("latency", {}).get(platform, {})
    findings = []
    for op, stats in latency.items():
        old = previous.get(op)
        if old is None:
            continue
        new_p95, old_p95 = stats["p95_us"], old["p95_us"]
        if new_p95 > old_p95 * BASELINE_REGRESSION_RATIO and new_p95 - old_p95 > BASELINE_REGRESSION_MIN_DELTA_US:
            findings.append(Finding("perf", platform, "*", f"{op} p95 {new_p95}µs regressed from {old_p95}µs"))
    return findings


def analyze(run_dir: Path, scenarios_dir: Path, budgets: dict, baseline: dict | None, proto_dir: Path) -> Report:
    method_names = load_method_names(proto_dir)
    scenarios = load_scenarios(scenarios_dir)
    report = Report()
    platform_durations: dict[str, dict[str, list[int]]] = {}
    for result_path in sorted(run_dir.glob("*/*/result.json")):
        result = json.loads(result_path.read_text(encoding="utf-8"))
        platform, scenario_id = result_path.parent.parent.name, result_path.parent.name
        scenario = scenarios.get(scenario_id)
        if scenario is None:
            report.outcomes.append(ScenarioOutcome(platform, scenario_id, "error", "no such scenario"))
            continue
        status = result.get("status")
        if status != "ran":
            outcome_status = "skipped" if status == "skipped" else "error"
            report.outcomes.append(ScenarioOutcome(platform, scenario_id, outcome_status, result.get("reason", "")))
            continue
        events, parse_problems = read_trace(result_path.parent)
        collect_durations(events, method_names, platform_durations.setdefault(platform, {}))
        for message in parse_problems + check_trace_integrity(events):
            report.findings.append(Finding("trace", platform, scenario_id, message))
        for message in check_engine_errors(events):
            report.findings.append(Finding("bug", platform, scenario_id, message))
        failures, unverifiable = check_expectations(scenario, result.get("observed_text", ""), events)
        for message in unverifiable:
            report.findings.append(Finding("unverified", platform, scenario_id, message))
        report.outcomes.append(
            ScenarioOutcome(platform, scenario_id, "failed" if failures else "passed", "; ".join(failures))
        )
    for platform, durations in sorted(platform_durations.items()):
        latency = summarize_latency(durations)
        report.latency[platform] = latency
        report.findings.extend(check_budgets(platform, latency, budgets))
        if baseline is not None:
            report.findings.extend(check_baseline(platform, latency, baseline))
    return report


def render_markdown(report: Report) -> str:
    counts = {s: sum(o.status == s for o in report.outcomes) for s in ("passed", "failed", "skipped", "error")}
    lines = [
        "# E2E report",
        "",
        f"**{'FAIL' if report.is_failing else 'PASS'}** — "
        + ", ".join(f"{n} {s}" for s, n in counts.items())
        + f", {len(report.findings)} findings",
        "",
        "## Scenarios",
        "",
        "| Platform | Scenario | Status | Detail |",
        "|---|---|---|---|",
    ]
    lines += [f"| {o.platform} | {o.scenario} | {o.status} | {o.detail} |" for o in report.outcomes]
    if report.findings:
        lines += ["", "## Findings", "", "| Kind | Platform | Scenario | Finding |", "|---|---|---|---|"]
        lines += [f"| {f.kind} | {f.platform} | {f.scenario} | {f.message} |" for f in report.findings]
    for platform, latency in report.latency.items():
        lines += ["", f"## Engine latency — {platform}", "", "| Op | Count | p50 µs | p95 µs | max µs |", "|---|---|---|---|---|"]
        lines += [f"| {op} | {s['count']} | {s['p50_us']} | {s['p95_us']} | {s['max_us']} |" for op, s in latency.items()]
    return "\n".join(lines) + "\n"


def to_json(report: Report) -> dict:
    return {
        "failing": report.is_failing,
        "outcomes": [asdict(o) for o in report.outcomes],
        "findings": [asdict(f) for f in report.findings],
        "latency": report.latency,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--run", type=Path, required=True, help="run dir: <platform>/<scenario>/{result.json,*.jsonl}")
    parser.add_argument("--scenarios", type=Path, default=REPO_ROOT / "e2e" / "scenarios")
    parser.add_argument("--budgets", type=Path, default=REPO_ROOT / "e2e" / "budgets.json")
    parser.add_argument("--baseline", type=Path, help="report.json of an earlier run")
    parser.add_argument("--protos", type=Path, default=REPO_ROOT / "engine" / "protos" / "proto")
    args = parser.parse_args(argv)

    budgets = json.loads(args.budgets.read_text(encoding="utf-8"))
    baseline = json.loads(args.baseline.read_text(encoding="utf-8")) if args.baseline else None
    report = analyze(args.run, args.scenarios, budgets, baseline, args.protos)
    (args.run / "report.md").write_text(render_markdown(report), encoding="utf-8")
    (args.run / "report.json").write_text(json.dumps(to_json(report), ensure_ascii=False, indent=2), encoding="utf-8")
    print(args.run / "report.md")
    return 1 if report.is_failing else 0


if __name__ == "__main__":
    sys.exit(main())
