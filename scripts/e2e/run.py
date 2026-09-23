"""
Discovers and runs every `scripts/e2e/flows/<area>.py`, or just one with `--only`.

    python scripts/e2e/run.py [<shots_dir>] [--only area] [--base URL]

Flows run in a fixed, dependency-aware order (cashier-pos creates the invoice that role-gating
and refund-payment then look at; reports runs last so it sees everything posted before it). Each
flow's `run(base, shots_dir)` returns 0/1; this exits non-zero on the first failure, printing which
flow failed.
"""
from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

FLOWS_DIR = Path(__file__).resolve().parent / "flows"

# Explicit order: later flows assume state left by earlier ones (e.g. role-gating and
# refund-payment both read the invoice list that cashier-pos just created into).
ORDER = ["cashier_pos", "role_gating", "accountant_journal", "refund_payment", "products", "reports"]


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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("shots_dir", nargs="?", default="shots", help="directory to write screenshots into")
    parser.add_argument("--only", help="run a single area (e.g. cashier-pos)")
    parser.add_argument("--base", default="http://localhost:1420/#", help="app base URL incl. hash route prefix")
    args = parser.parse_args()

    modules = discover()
    if args.only:
        wanted = args.only.replace("-", "_")
        if wanted not in modules:
            print(f"unknown area '{args.only}' — known: {', '.join(area_name(m) for m in modules)}")
            return 1
        modules = [wanted]

    shots_dir = Path(args.shots_dir)
    failed: list[str] = []
    for module_name in modules:
        area = area_name(module_name)
        print(f"=== {area} ===")
        flow = load_flow(module_name)
        try:
            rc = flow.run(args.base, shots_dir)
        except SystemExit as e:
            rc = e.code if isinstance(e.code, int) else 1
        if rc:
            failed.append(area)
            print(f"=== {area}: FAILED ===\n")
        else:
            print(f"=== {area}: ok ===\n")

    if failed:
        print(f"FAILED: {', '.join(failed)}")
        return 1
    print("all flows passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
