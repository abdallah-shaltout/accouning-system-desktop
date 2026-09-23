"""
Role gating: a cashier is blocked from accounting routes and doesn't see them in the sidebar.

    python scripts/e2e/flows/role_gating.py [--base URL] [--shots DIR]
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

        print("role gating")
        login_as(page, base, "cashier")
        check("/pos" in page.url, "cashier lands on the POS after login")

        page.goto(f"{base}/accounting/accounts")
        page.wait_for_timeout(700)
        check("/forbidden" in page.url, "cashier is blocked from the chart of accounts")
        nav = page.locator("nav[aria-label='القائمة الرئيسية']").inner_text()
        check("دليل الحسابات" not in nav and "التقارير" not in nav, "cashier sidebar hides accounting & reports")

        page.goto(f"{base}/invoices")
        page.wait_for_timeout(900)
        check(page.locator("tbody tr").count() > 0, "cashier can still see the invoice list")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
