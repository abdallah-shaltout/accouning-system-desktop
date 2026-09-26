"""
Discovers and runs every `scripts/e2e/flows/<area>.py`, or just one with `--only`.

    python scripts/e2e/run.py [<shots_dir>] [--only area] [--base URL]
    python scripts/e2e/run.py --update-baseline    # accept current durations as the new perf baseline

Flows run in a fixed, dependency-aware order (cashier-pos creates the invoice that role-gating
and refund-payment then look at; reports runs last so it sees everything posted before it). Each
flow's `run(base, shots_dir)` returns 0/1; this exits non-zero on the first failure, printing which
flow failed.

18.G (close the dev loop): every flow's last line before `browser.close()` is now
`finish(page, browser, "<area>")` (scripts/e2e/common.py), which pulls
`window.__equal.diag.export()` and writes it to `.diagnostics/runs/<ts>/<flow>.jsonl` — one JSONL
per flow, all 5 channels, same shape as the dev-server's `.diagnostics/logs/<channel>.jsonl`. This
runner then:
  - times each flow and compares it to `docs/diagnostics/perf-baseline.json`; >25% slower creates or
    refreshes a `PERF-` ledger issue (`--update-baseline` accepts the current run as the new baseline
    instead of comparing).
  - reads each flow's exported `error`-channel entries back out of its JSONL and turns each into a
    `BUG-` ledger finding (same fingerprint the app's own error toast/support bundle would show); a
    flow that failed its own assertions with nothing on the `error` channel still gets one finding
    so a real regression is never invisible to the ledger.
  - all findings go through the exact same upsert mechanism as B7's fingerprint auto-stubbing
    (`scripts/diagnostics/run.ts --ingest`), so there is one issue-creation code path, not two.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import safe_print  # noqa: E402

FLOWS_DIR = Path(__file__).resolve().parent / "flows"
ROOT = Path(__file__).resolve().parents[2]
DIAGNOSTICS_RUNS_DIR = ROOT / ".diagnostics" / "runs"
PERF_BASELINE_PATH = ROOT / "docs" / "diagnostics" / "perf-baseline.json"

# 18.G: >25% slower than the recorded baseline creates/refreshes a PERF- issue. Documented choice —
# short enough to catch a real regression, loose enough that normal machine/CPU-load noise between
# runs (this suite has no fixed CI hardware) doesn't spam the ledger on every run.
PERF_REGRESSION_THRESHOLD = 0.25

# Bundle-size regression threshold (18.G task 4): the `dist/` output is compared against the
# baseline's `bundleSizeBytes` the same way flow duration is compared to its own baseline — a size
# jump of more than 15% is treated as a regression. Chosen tighter than the 25% perf threshold
# because bundle size is far less noisy than wall-clock timing (no machine-load variance), so a
# smaller real jump (a heavy new dependency, an accidentally-bundled asset) is still worth flagging
# without tripping on routine incremental growth from normal feature work.
BUNDLE_SIZE_REGRESSION_THRESHOLD = 0.15

# Explicit order: later flows assume state left by earlier ones (e.g. role-gating and
# refund-payment both read the invoice list that cashier-pos just created into). purchases/expenses
# (v2 phase 8) run after products (batches) and before reports (which then sees their postings too).
# `onboarding` (v2 phase 5) runs FIRST: it clears the persisted IndexedDB snapshot to prove a
# genuinely fresh install (no company yet) goes through the wizard end to end — every later flow's
# `ensure_demo_data()` (scripts/e2e/common.py) reseeds the demo dataset it needs regardless.
# `setup_wizard_eg` (v2 doc 18.D) runs right after `onboarding`, for the same reason: it clears the
# snapshot again to drive a second, genuinely fresh install through the Egypt path (14% VAT, EGP,
# no ZATCA QR) — placed here so no later flow's demo-data assumptions sit on top of its EG company.
# `full_persona_pass` (v2 phase 13b) runs LAST: it's the final consolidated persona-day story and
# posts/decides its own approval request, so it should see everything every earlier flow left behind.
ORDER = [
    "onboarding", "setup_wizard_eg", "cashier_pos", "desk_invoice", "role_gating", "accountant_journal", "refund_payment",
    "products", "purchases", "expenses", "branches_currencies", "reports", "reports_v2", "report_print", "labels_templates",
    "home_insights", "full_persona_pass",
]


def area_name(module_name: str) -> str:
    return module_name.replace("_", "-")


def load_flow(module_name: str):
    path = FLOWS_DIR / f"{module_name}.py"
    spec = importlib.util.spec_from_file_location(module_name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def discover() -> list[str]:
    found = sorted(p.stem for p in FLOWS_DIR.glob("*.py") if p.stem != "__init__")
    # Known flows first, in dependency order; any new/unlisted flow file runs after, alphabetically.
    ordered = [m for m in ORDER if m in found]
    ordered += [m for m in found if m not in ORDER]
    return ordered


def load_baseline() -> dict:
    if not PERF_BASELINE_PATH.exists():
        return {"flows": {}, "bundleSizeBytes": None}
    return json.loads(PERF_BASELINE_PATH.read_text(encoding="utf-8"))


def save_baseline(baseline: dict) -> None:
    PERF_BASELINE_PATH.parent.mkdir(parents=True, exist_ok=True)
    PERF_BASELINE_PATH.write_text(json.dumps(baseline, indent=2, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")


def dist_size_bytes() -> int | None:
    dist = ROOT / "dist"
    if not dist.exists():
        return None
    return sum(f.stat().st_size for f in dist.rglob("*") if f.is_file())


def read_exported_errors(run_dir: Path, area: str) -> list[dict]:
    """Reads this flow's `error`-channel entries back out of the JSONL `finish()` (common.py) just
    wrote — the same `window.__equal.diag.export()` data the support-bundle export and
    `/dev/diagnostics` Errors tab use, so an e2e failure is tracked through the identical pipeline
    as a real user's error toast, not a parallel Python-only notion of "error"."""
    jsonl_path = run_dir / f"{area}.jsonl"
    if not jsonl_path.exists():
        return []
    found = []
    for line in jsonl_path.read_text(encoding="utf-8").splitlines():
        try:
            entry = json.loads(line)
        except json.JSONDecodeError:
            continue
        if entry.get("channel") == "error":
            found.append(entry)
    return found


def build_e2e_findings(run_dir: Path, area: str, flow_failed: bool) -> list[dict]:
    findings = []
    for err in read_exported_errors(run_dir, area):
        info = err.get("err") or {}
        fingerprint = info.get("fingerprint") or f"e2e:{area}:{(err.get('msg') or '')[:160]}"
        message = err.get("msg") or info.get("message") or "(no message)"
        findings.append({
            "kind": "bug",
            "fingerprint": fingerprint,
            "area": area,
            "title": f"{info.get('name') or 'error'} in {area}: {message[:120]}",
            "body": (
                f"المصدر: `{err.get('source', area)}` (e2e flow `{area}`)\n\n"
                f"```\n{message}\n{info.get('stack', '')}\n```\n\n"
                f"Run dir: `{run_dir.relative_to(ROOT)}`"
            ),
            # scripts/diagnostics/run.ts's ingest() drops a finding whose error_name is in its own
            # EXCLUDED_ERROR_NAMES (e.g. ApiError — an expected user-facing validation message, not
            # a bug) — same filter B7's fingerprint auto-stubbing already applies, so this flow
            # doesn't create ledger noise just because it went through a different ingestion path.
            "error_name": info.get("name"),
        })
    if flow_failed and not findings:
        # The flow's own assertions failed (make_check()/check()) but nothing reached the `error`
        # channel — still worth a ledger entry so a real regression isn't invisible to the ledger
        # just because it was a failed assertion rather than a thrown/logged error.
        findings.append({
            "kind": "bug",
            "fingerprint": f"e2e:{area}:assertion-failure",
            "area": area,
            "title": f"{area} flow failed an assertion",
            "body": (
                f"المصدر: e2e flow `{area}`\n\n"
                f"A `check(...)` in `scripts/e2e/flows/{area.replace('-', '_')}.py` failed — see the "
                "run's console output for which one. No structured error was exported for this run "
                f"(see `{run_dir.relative_to(ROOT)}/{area}.jsonl` if present)."
            ),
        })
    return findings


def build_perf_findings(area: str, duration_ms: int, baseline: dict, update: bool) -> tuple[list[dict], bool]:
    flows = baseline.setdefault("flows", {})
    prior = flows.get(area)
    findings: list[dict] = []
    changed = False
    if update or prior is None:
        flows[area] = duration_ms
        changed = True
        return findings, changed

    if prior > 0 and (duration_ms - prior) / prior > PERF_REGRESSION_THRESHOLD:
        pct = round((duration_ms - prior) / prior * 100)
        findings.append({
            "kind": "perf",
            "fingerprint": f"perf:{area}:duration",
            "area": area,
            "title": f"{area} flow ran {pct}% slower than baseline",
            "body": (
                f"Baseline: {prior} ms. Actual: {duration_ms} ms. Threshold: "
                f"{int(PERF_REGRESSION_THRESHOLD * 100)}%.\n\n"
                f"المصدر: `scripts/e2e/flows/{area.replace('-', '_')}.py`\n\n"
                "لتحديث الأساس بعد تحسين متعمد: `python scripts/e2e/run.py --update-baseline`."
            ),
        })
    return findings, changed


def build_bundle_size_finding(baseline: dict, update: bool) -> tuple[list[dict], bool]:
    size = dist_size_bytes()
    if size is None:
        return [], False
    prior = baseline.get("bundleSizeBytes")
    if update or prior is None:
        baseline["bundleSizeBytes"] = size
        return [], True

    findings: list[dict] = []
    if prior > 0 and (size - prior) / prior > BUNDLE_SIZE_REGRESSION_THRESHOLD:
        pct = round((size - prior) / prior * 100)
        findings.append({
            "kind": "perf",
            "fingerprint": "bundle-size",
            "area": "build",
            "title": f"dist/ bundle grew {pct}% since the last baseline",
            "body": (
                f"Baseline: {prior:,} bytes. Actual: {size:,} bytes. Threshold: "
                f"{int(BUNDLE_SIZE_REGRESSION_THRESHOLD * 100)}%.\n\n"
                "Run `bun run build` then `python scripts/e2e/run.py --update-baseline` to accept a "
                "deliberate size increase, or find what got newly bundled first."
            ),
        })
    return findings, False


def ingest_findings(findings: list[dict]) -> None:
    if not findings:
        return
    import tempfile

    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False, encoding="utf-8") as f:
        json.dump(findings, f, ensure_ascii=False)
        temp_path = f.name
    try:
        subprocess.run(
            ["bun", "run", "scripts/diagnostics/run.ts", "--ingest", temp_path],
            cwd=str(ROOT), check=True,
        )
    finally:
        os.unlink(temp_path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("shots_dir", nargs="?", default="shots", help="directory to write screenshots into")
    parser.add_argument("--only", help="run a single area (e.g. cashier-pos)")
    parser.add_argument("--base", default="http://localhost:1420/#", help="app base URL incl. hash route prefix")
    parser.add_argument("--update-baseline", action="store_true", help="record current durations/bundle size as the new perf baseline instead of comparing")
    args = parser.parse_args()

    modules = discover()
    if args.only:
        wanted = args.only.replace("-", "_")
        if wanted not in modules:
            print(f"unknown area '{args.only}' — known: {', '.join(area_name(m) for m in modules)}")
            return 1
        modules = [wanted]

    run_ts = time.strftime("%Y%m%dT%H%M%S")
    run_dir = DIAGNOSTICS_RUNS_DIR / run_ts
    run_dir.mkdir(parents=True, exist_ok=True)
    os.environ["EQUAL_DIAG_RUN_DIR"] = str(run_dir)

    baseline = load_baseline()
    baseline_changed = False
    all_findings: list[dict] = []

    shots_dir = Path(args.shots_dir)
    failed: list[str] = []
    for module_name in modules:
        area = area_name(module_name)
        print(f"=== {area} ===")
        flow = load_flow(module_name)
        started = time.perf_counter()
        crash: str | None = None
        try:
            rc = flow.run(args.base, shots_dir)
        except SystemExit as e:
            rc = e.code if isinstance(e.code, int) else 1
        except Exception as e:  # noqa: BLE001 - one flow crashing (e.g. a Playwright strict-mode
            # violation from an unrelated selector ambiguity) must not abort every later flow, and
            # must still leave a ledger trail — this used to propagate to a bare, uncaught traceback
            # that killed the whole run.py process (18.G hardening, found while testing this file).
            import traceback

            rc = 1
            crash = "".join(traceback.format_exception(e))
            safe_print(f"  CRASH {area}: {type(e).__name__}: {e}")
        duration_ms = int((time.perf_counter() - started) * 1000)

        if crash:
            last_line = crash.strip().splitlines()[-1] if crash.strip() else "unknown error"
            all_findings.append({
                "kind": "bug",
                "fingerprint": f"e2e:{area}:crash:{last_line[:160]}",
                "area": area,
                "title": f"{area} flow crashed (unhandled exception): {last_line[:120]}",
                "body": f"المصدر: e2e flow `{area}`\n\n```\n{crash}\n```",
            })
        else:
            all_findings += build_e2e_findings(run_dir, area, flow_failed=bool(rc))
        perf_findings, changed = build_perf_findings(area, duration_ms, baseline, args.update_baseline)
        all_findings += perf_findings
        baseline_changed = baseline_changed or changed

        if rc:
            failed.append(area)
            print(f"=== {area}: FAILED ({duration_ms} ms) ===\n")
        else:
            print(f"=== {area}: ok ({duration_ms} ms) ===\n")

    bundle_findings, bundle_changed = build_bundle_size_finding(baseline, args.update_baseline)
    all_findings += bundle_findings
    baseline_changed = baseline_changed or bundle_changed

    if baseline_changed:
        save_baseline(baseline)
        print(f"perf baseline updated: {PERF_BASELINE_PATH.relative_to(ROOT)}")

    ingest_findings(all_findings)
    if all_findings:
        print(f"diagnostics: {len(all_findings)} finding(s) ingested into the issue ledger (docs/diagnostics/ISSUES.md)")

    print(f"diagnostics run archived under {run_dir.relative_to(ROOT)}")

    if failed:
        print(f"FAILED: {', '.join(failed)}")
        return 1
    print("all flows passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
