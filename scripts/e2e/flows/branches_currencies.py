"""
Branches, cost centers & currencies (v2 phase 9, docs/v2/10-branches-currencies-cost-centers.md):
    - the topbar branch switcher (demo seed ships 2 branches + features on)
    - the USD customer's invoice showing FX
    - a cost-center split on a manual journal line
    - a stock transfer between branches
    - feature-switches-OFF: no branch/currency/cost-center chrome anywhere

    python scripts/e2e/flows/branches_currencies.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, login_as, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("branches, cost centers & currencies")
        login_as(page, base, "admin")

        # --- Sidebar brand/branch switcher (features are ON in the demo seed; docs/v2/17 Phase D
        # moved it from the topbar into the sidebar header) -------------------------------------
        page.wait_for_timeout(500)
        switcher = page.locator("[data-slot=sidebar] button", has_text="كل الفروع").or_(page.locator("[data-slot=sidebar] button", has_text="الفرع"))
        check(switcher.count() > 0, "sidebar branch switcher is visible (features on, >1 branch)")
        if switcher.count():
            switcher.first.click()
            page.wait_for_timeout(200)
            jeddah = page.get_by_role("menuitem", name="فرع جدة")
            check(jeddah.count() > 0, "switcher lists the second branch (فرع جدة)")
            if jeddah.count():
                jeddah.click()
                page.wait_for_timeout(200)
            shot(page, shots_dir, "branch_switcher")

        # --- USD invoice with FX ----------------------------------------------------------------
        page.goto(f"{base}/invoices")
        page.wait_for_timeout(700)
        page.locator("input[type=search], input[type=text]").first.fill("Global Trading")
        page.wait_for_timeout(500)
        row = page.locator("tbody tr").first
        check(row.count() > 0, "USD customer's invoice appears in the invoice list")
        if row.count():
            row.click()
            page.wait_for_timeout(700)
            body = page.locator("body").inner_text()
            check("USD" in body or "$" in body, "invoice detail shows the USD currency")
            shot(page, shots_dir, "usd_invoice")

        # --- Cost-center split (manual journal) --------------------------------------------------
        page.goto(f"{base}/accounting/journal")
        page.wait_for_timeout(1200)
        search_box = page.locator("input[type=search], input[type=text]").first
        search_box.fill("موزع على الفروع")
        page.wait_for_timeout(800)
        je_link = page.get_by_text("JE-", exact=False).first
        found = je_link.count() > 0
        check(found, "cost-center split journal entry appears in the journal list")
        if found:
            je_link.click()
            page.wait_for_timeout(600)
            shot(page, shots_dir, "cost_center_split")

        # --- Stock transfer screen ---------------------------------------------------------------
        page.goto(f"{base}/inventory/transfers")
        page.wait_for_timeout(700)
        transfer_page_ok = "/forbidden" not in page.url and "/not-found" not in page.url
        check(transfer_page_ok, "stock transfers screen loads")
        if transfer_page_ok:
            shot(page, shots_dir, "stock_transfer")

        # --- Cost-center P&L report ---------------------------------------------------------------
        page.goto(f"{base}/reports/cost-centers")
        page.wait_for_timeout(900)
        cc_report_ok = "/forbidden" not in page.url and "/not-found" not in page.url
        check(cc_report_ok, "cost-center P&L report loads")
        if cc_report_ok:
            shot(page, shots_dir, "cost_center_pnl")

        # --- Feature switches OFF: no branch/currency/cost-center chrome anywhere ---------------
        page.goto(f"{base}/settings/general")
        page.wait_for_timeout(700)
        dims_card = page.get_by_text("الأبعاد (الفروع", exact=False).locator("xpath=ancestor::*[contains(@class,'rounded-xl') or contains(@class,'rounded-2xl')][1]")
        toggles = dims_card.locator("[role=switch]")
        for i in range(toggles.count()):
            t = toggles.nth(i)
            if t.get_attribute("aria-checked") == "true":
                t.click()
                page.wait_for_timeout(150)
        save_btn = page.get_by_role("button", name="حفظ الإعدادات")
        if save_btn.count():
            save_btn.click()
            page.wait_for_timeout(600)

        page.goto(f"{base}/dashboard")
        page.wait_for_timeout(700)
        switcher_after = page.locator("[data-slot=sidebar] button", has_text="كل الفروع").or_(page.locator("[data-slot=sidebar] button", has_text="فرع"))
        check(switcher_after.count() == 0, "feature-switches-off: no branch switcher in the sidebar")
        shot(page, shots_dir, "features_off_dashboard")

        finish(page, browser, "branches-currencies")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
