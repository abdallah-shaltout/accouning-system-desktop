"""
v2 phase 12 (docs/v2/13-reports.md) — Reports v2 smoke test: the hub (groups/search), three
upgraded reports (trial balance, P&L with comparison, VAT) with the new ReportShell chrome, two new
reports (aging, gross profit), an Excel export download, and branch/cost-center/currency filter
visibility tied to the feature switches (docs/v2/10 §4).

    python scripts/e2e/flows/reports_v2.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, login_as, make_check, safe_print  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("reports_v2: hub")
        login_as(page, base, "manager")
        page.goto(f"{base}/reports")
        page.wait_for_timeout(800)
        check(page.get_by_text("التقارير المالية").is_visible(), "hub shows the financial statements group")
        check(page.get_by_text("المبيعات").first.is_visible(), "hub shows a sales group")
        check(page.get_by_text("تقارير الإدارة").is_visible(), "hub shows the management group")
        # Search narrows the catalogue.
        page.fill("input[placeholder='ابحث عن تقرير…']", "أعمار")
        page.wait_for_timeout(300)
        check(page.get_by_text("أعمار الديون").is_visible(), "search finds the aging report")
        check(not page.get_by_text("ميزان المراجعة").is_visible(), "search hides non-matching reports")
        page.fill("input[placeholder='ابحث عن تقرير…']", "")
        page.wait_for_timeout(200)

        # Favorite toggle persists (localStorage) and surfaces a "المفضلة" section.
        card = page.locator("a[href*='trial-balance']").locator("..")
        star = card.locator("button[title='إضافة للمفضلة']")
        if star.count():
            star.first.click(force=True)
            page.wait_for_timeout(200)
            check(page.get_by_text("المفضلة").is_visible(), "favoriting a report shows the المفضلة section")

        safe_print("reports_v2: trial balance (upgraded)")
        page.goto(f"{base}/reports/trial-balance")
        page.wait_for_timeout(1000)
        check(page.get_by_text("الميزان متوازن").is_visible(), "trial balance still balances")
        check(page.locator("button:has-text('Excel')").is_visible(), "ReportShell exposes an Excel export button")

        safe_print("reports_v2: profit & loss (upgraded, comparison mode)")
        page.goto(f"{base}/reports/profit-loss")
        page.wait_for_selector("text=مجمل الربح", timeout=10000)
        page.wait_for_timeout(300)
        check(page.get_by_text("صافي الربح").first.is_visible(), "P&L renders net income")
        page.click("button:has-text('الفترة السابقة')")
        page.wait_for_timeout(900)
        check(page.get_by_text("التغير في صافي الربح").first.is_visible(), "comparison mode shows the delta panel")

        safe_print("reports_v2: VAT summary (upgraded)")
        page.goto(f"{base}/reports/vat")
        page.wait_for_timeout(1000)
        check("مطابق لأرصدة حسابات الضريبة" in page.inner_text("body"), "VAT report reconciles with the ledger")

        safe_print("reports_v2: new report — aging")
        page.goto(f"{base}/reports/aging")
        page.wait_for_timeout(1000)
        check(page.locator("table").is_visible(), "aging report renders a table")

        safe_print("reports_v2: new report — gross profit")
        page.goto(f"{base}/reports/gross-profit")
        page.wait_for_timeout(1000)
        check(page.locator("table").is_visible(), "gross profit report renders a table")

        safe_print("reports_v2: Excel export downloads")
        page.goto(f"{base}/reports/trial-balance")
        page.wait_for_timeout(1000)
        with page.expect_download(timeout=15000) as dl_info:
            page.click("button:has-text('Excel')")
        download = dl_info.value
        check(download.suggested_filename.endswith(".xlsx"), f"Excel export downloads an .xlsx file ({download.suggested_filename})")

        safe_print("reports_v2: branch/cost-center/currency filters follow feature switches")
        page.goto(f"{base}/reports/trial-balance")
        page.wait_for_timeout(800)
        # The demo seed has branches/currencies/cost centers ON (Phase 9's seed) — the filter bar
        # (the bordered `#filters` slot rendered by ReportShell, not the header's print/export
        # buttons, which also carry `no-print`) should show three dimension <select>s.
        select_count = page.locator("select").count()
        check(select_count >= 3, f"branch/cost-center/currency filter selects render when features are on ({select_count} selects found)")

        finish(page, browser, "reports-v2")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
