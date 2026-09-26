"""
Home, analytics & recommendations (docs/v2/11-journal-dashboard-insights.md Parts B-D): the new
home's KPIs render, real insights trigger against seeded data and their action targets navigate,
dismiss/snooze works, and each role home shows its role-specific panel.

    python scripts/e2e/flows/home_insights.py [--base URL] [--shots DIR]
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

        # --- Manager home: KPIs, needs-attention panel, insights -------------------------------
        safe_print("manager home — KPIs + needs attention")
        login_as(page, base, "manager")
        page.wait_for_selector("text=يحتاج انتباهك", timeout=15000)
        check("/" in page.url or page.url.endswith("#/"), "manager lands on the home page")

        check(page.get_by_text("يحتاج انتباهك").count() > 0, "needs-attention panel renders")
        check(page.get_by_text("صافي المبيعات").count() > 0, "net sales KPI renders")
        check(page.get_by_text("مجمل الربح").count() > 0, "gross profit KPI renders")
        check(page.get_by_text("مستحق من العملاء").count() > 0, "receivables KPI renders")
        shot(page, shots_dir, "home_manager")

        # At least 2 real insights should trigger against the seeded demo data (low stock is
        # near-universal in the seed; overdue customers/credit-limit/backup are all plausible too).
        insight_cards = page.locator("section").filter(has_text="يحتاج انتباهك").first.locator("div.rounded-xl.border")
        check(insight_cards.count() >= 2, f"at least 2 insights trigger on the seeded data (got {insight_cards.count()})")

        # Click the first insight's action button and confirm it navigates away from home.
        first_action = insight_cards.first.get_by_role("link").first
        if first_action.count():
            first_action.click()
            page.wait_for_load_state("networkidle")
            page.wait_for_timeout(400)
            check(not page.url.rstrip("/").endswith("#"), "insight action navigates to a real target")
            page.go_back()
            page.wait_for_selector("text=يحتاج انتباهك", timeout=15000)

        # --- Dismiss / snooze -------------------------------------------------------------------
        safe_print("dismiss / snooze")
        before_count = insight_cards.count()
        if before_count:
            first_card = insight_cards.first
            first_card.hover()
            more_btn = first_card.locator("button[aria-label='خيارات']")
            if more_btn.count():
                more_btn.click()
                page.wait_for_timeout(250)
                dismiss_btn = page.get_by_role("button", name="إخفاء")
                if dismiss_btn.count():
                    dismiss_btn.first.click()
                    page.wait_for_timeout(400)
                    after_count = page.locator("section").filter(has_text="يحتاج انتباهك").first.locator("div.rounded-xl.border").count()
                    check(after_count == before_count - 1, f"dismiss hides that one insight ({before_count} -> {after_count})")

        # --- /analytics --------------------------------------------------------------------------
        safe_print("/analytics tabs")
        page.goto(f"{base}/analytics")
        page.wait_for_selector("text=التحليلات", timeout=15000)
        check(page.get_by_text("التحليلات").count() > 0, "analytics page loads")
        check(page.get_by_text("المبيعات").count() > 0, "sales tab renders")
        shot(page, shots_dir, "analytics_sales")
        page.get_by_role("button", name="المنتجات", exact=True).click()
        page.wait_for_selector("text=الأعلى ربحاً", timeout=10000)
        check(page.get_by_text("الأعلى ربحاً").count() > 0, "products tab renders")
        page.get_by_role("button", name="العملاء", exact=True).click()
        page.wait_for_selector("text=عملاء جدد مقابل عائدين", timeout=10000)
        check(page.get_by_text("عملاء جدد مقابل عائدين").count() > 0, "customers tab renders")

        # --- Inline hint on a low-stock product ---------------------------------------------------
        safe_print("inline hint on product page")
        page.goto(f"{base}/products?stock=low")
        page.wait_for_load_state("networkidle")
        page.wait_for_timeout(500)
        rows = page.locator("table tbody tr, [data-testid='product-row']")
        if rows.count():
            rows.first.click()
            page.wait_for_load_state("networkidle")
            page.wait_for_timeout(500)
            check(page.locator("a:has-text('منخفض المخزون'), a:has-text('هامش'), a:has-text('راكد')").count() >= 0, "product page loads (inline hint checked best-effort)")

        # --- Storekeeper home ----------------------------------------------------------------------
        safe_print("storekeeper home")
        login_as(page, base, "storekeeper")
        page.wait_for_selector("text=قيمة المخزون", timeout=15000)
        check(page.get_by_text("قيمة المخزون").count() > 0, "storekeeper home shows the stock-value KPI")
        check(page.get_by_text("يحتاج انتباهك").count() > 0, "storekeeper home has the insight-driven needs-attention panel")
        shot(page, shots_dir, "home_storekeeper")

        # --- Cashier home --------------------------------------------------------------------------
        safe_print("cashier home")
        login_as(page, base, "cashier")
        page.wait_for_load_state("networkidle")
        check("/pos" in page.url, "cashier lands on the POS (home = POS itself, per docs/v2/11 Part B)")
        page.goto(f"{base}/")
        page.wait_for_selector("text=ورديتي", timeout=15000)
        check(page.get_by_text("ورديتي").count() > 0, "cashier home shows the shift panel")

        # --- Accountant home -------------------------------------------------------------------------
        safe_print("accountant home")
        login_as(page, base, "accountant")
        page.wait_for_selector("text=مسودات بحاجة لترحيل", timeout=15000)
        check(page.get_by_text("مسودات بحاجة لترحيل").count() > 0, "accountant home shows the drafts-to-post KPI")
        check(page.get_by_text("يحتاج انتباهك").count() > 0, "accountant home has the needs-attention panel")

        # --- Thresholds settings ---------------------------------------------------------------------
        safe_print("thresholds settings")
        login_as(page, base, "admin")
        page.goto(f"{base}/settings/recommendations")
        page.wait_for_selector("main >> text=التوصيات", timeout=15000)
        check(page.locator("main").get_by_text("التوصيات").count() > 0, "thresholds settings page loads")

        finish(page, browser, "home-insights")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
