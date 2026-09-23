"""
Reports still balance: trial balance, balance sheet, VAT report reconciliation.

    python scripts/e2e/flows/reports.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, make_check  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        print("reports")
        login_as(page, base, "manager")
        page.goto(f"{base}/reports/trial-balance")
        page.wait_for_timeout(1200)
        check(page.get_by_text("الميزان متوازن").is_visible(), "trial balance still balances")
        page.goto(f"{base}/reports/balance-sheet")
        page.wait_for_timeout(1000)
        check(page.get_by_text("الأصول = الالتزامات + حقوق الملكية").is_visible(), "balance sheet still balances")
        page.goto(f"{base}/reports/vat")
        page.wait_for_timeout(1000)
        check("مطابق لأرصدة حسابات الضريبة" in page.inner_text("body"), "VAT report reconciles with the ledger")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
