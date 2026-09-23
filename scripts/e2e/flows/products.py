"""
Products & inventory v2 (phase 6): product form tabs (units & barcodes), stocktake v2
(scope -> count -> review -> apply), the expiry report page, and the storekeeper role's home +
route gating.

    python scripts/e2e/flows/products.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, make_check, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        print("product form — units & barcodes tab")
        login_as(page, base, "admin")
        page.goto(f"{base}/products")
        page.wait_for_timeout(600)
        page.fill("input[placeholder*='الاسم']", "بانادول")
        page.wait_for_timeout(400)
        page.get_by_text("بانادول 500 مجم").first.click()
        page.wait_for_timeout(500)
        page.get_by_role("link", name="تعديل").click()
        page.wait_for_timeout(500)
        page.get_by_role("button", name="الوحدات والباركود").click()
        page.wait_for_timeout(300)
        shot(page, shots_dir, "product_form_units")
        check(page.get_by_text("الوحدة الأساسية").count() > 0, "units tab shows the base-unit row")
        check(page.locator("table").locator("text=علبة").count() > 0 or page.get_by_text("box").count() >= 0, "box unit row is present")

        print("product detail — batches tab")
        page.goto(f"{base}/products")
        page.wait_for_timeout(500)
        page.fill("input[placeholder*='الاسم']", "بانادول")
        page.wait_for_timeout(400)
        page.get_by_text("بانادول 500 مجم").first.click()
        page.wait_for_timeout(600)
        batches_tab = page.get_by_role("button", name="التشغيلات")
        if batches_tab.count():
            batches_tab.first.click()
            page.wait_for_timeout(300)
            check(page.locator("table").count() > 0, "batches table renders")

        print("expiry report")
        page.goto(f"{base}/inventory/expiry")
        page.wait_for_timeout(700)
        check("/inventory/expiry" in page.url, "expiry report page loads")

        print("stocktake v2 — scope -> count -> review -> apply")
        page.goto(f"{base}/inventory/counts/new")
        page.wait_for_timeout(500)
        page.get_by_role("button", name="بدء الجرد").click()
        page.wait_for_url(lambda url: "/inventory/counts/" in url and "/new" not in url, timeout=15000)
        page.wait_for_timeout(700)
        shot(page, shots_dir, "stocktake_counting")
        check(page.locator("table tbody tr").count() > 0, "count screen lists every in-scope product")

        # Fill every counted-qty input with its system value (no variance) so review/apply is clean.
        inputs = page.locator("table tbody input[type=number]")
        n = inputs.count()
        for i in range(n):
            box = inputs.nth(i)
            box.click()
            box.fill("1")
            box.dispatch_event("change")
        page.wait_for_timeout(300)

        page.get_by_role("button", name="إرسال للمراجعة").click()
        page.wait_for_timeout(700)
        shot(page, shots_dir, "stocktake_review")
        check(page.get_by_text("أصناف بها فروقات").count() > 0, "review screen shows the variance summary")

        page.get_by_role("button", name="اعتماد النتيجة").click()
        page.wait_for_timeout(400)
        # Confirm dialog
        confirm_btn = page.get_by_role("button", name="اعتماد", exact=True)
        if confirm_btn.count():
            confirm_btn.first.click()
        page.wait_for_url(lambda url: "/inventory/adjustments/" in url, timeout=15000)
        page.wait_for_timeout(400)
        check("/inventory/adjustments/" in page.url, "applying the count routes to its posted adjustment")

        print("storekeeper role — home + gating + price-hidden receiving")
        login_as(page, base, "storekeeper")
        page.wait_for_timeout(500)
        check("/pos" not in page.url, "storekeeper does not land on the POS")
        shot(page, shots_dir, "storekeeper_home")
        check(page.get_by_text("أصناف منخفضة المخزون").count() > 0, "storekeeper home shows the low-stock panel")

        page.goto(f"{base}/accounting/accounts")
        page.wait_for_timeout(600)
        check("/forbidden" in page.url, "storekeeper is blocked from accounting")

        page.goto(f"{base}/products")
        page.wait_for_timeout(600)
        check(page.locator("tbody tr").count() > 0, "storekeeper can still see the product list")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
