"""
Desk invoice form + quotations + invoice list v2 (docs/v2/06-sales-and-pos.md §2, §6).

    python scripts/e2e/flows/desk_invoice.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, pick_combobox, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("desk invoice form — accountant")
        login_as(page, base, "accountant")
        page.goto(f"{base}/sales/invoices/new")
        page.wait_for_timeout(900)
        check("فاتورة مبيعات جديدة" in page.locator("h1, [class*=title]").first.inner_text(), "desk invoice form loads")

        # Pick a customer with a VAT number so invoice type auto-switches to STANDARD.
        combo = page.locator(".xl\\:grid-cols-\\[1fr_340px\\]").get_by_placeholder("عميل نقدي").first
        if combo.count():
            pick_combobox(page, combo, "شركة")
            page.wait_for_timeout(400)

        # Fill the first grid line via the datalist-backed input.
        first_name_input = page.locator("table tbody tr").first.locator("input").first
        first_name_input.fill("تيشيرت بولو رجالي")
        page.wait_for_timeout(200)
        row = page.locator("table tbody tr").first
        row.locator("input").nth(1).fill("2")
        row.locator("input").nth(2).fill("50")
        page.wait_for_timeout(400)
        shot(page, shots_dir, "1_desk_invoice_form")

        totals_card = page.locator("text=الإجمالي").first
        check(totals_card.count() > 0, "totals card renders")

        # Amount in words (tafqit) should show some Arabic text under the totals card.
        words = page.locator("text=فقط")
        check(words.count() > 0, "amount-in-words (tafqit) line is shown")

        post_btn = page.get_by_role("button", name="ترحيل الفاتورة")
        check(post_btn.is_visible(), "post button is visible")
        post_btn.click()
        page.wait_for_timeout(600)
        # Either the tender dialog opens (non-zero total) or it posts directly.
        if page.get_by_role("dialog").is_visible():
            page.wait_for_timeout(300)
            confirm = page.get_by_role("button", name="تأكيد البيع")
            if confirm.is_visible():
                confirm.click()
        page.wait_for_url(lambda url: "/invoices/" in url and "/new" not in url, timeout=10000)
        check("/invoices/" in page.url, "posting the desk invoice navigates to its detail page")
        shot(page, shots_dir, "2_desk_invoice_posted")

        safe_print("invoice list v2 — filters + footer totals")
        page.goto(f"{base}/invoices")
        page.wait_for_timeout(900)
        check(page.locator("text=آجل متأخر").count() > 0, "saved view 'آجل متأخر' is offered")
        check(page.locator("text=اليوم — فرعي").count() > 0, "saved view 'اليوم — فرعي' is offered")
        shot(page, shots_dir, "3_invoice_list_v2")

        safe_print("quotations — save draft, list, convert")
        page.goto(f"{base}/sales/quotations/new")
        page.wait_for_timeout(900)
        q_row = page.locator("table tbody tr").first
        q_row.locator("input").first.fill("قميص رجالي قطن كلاسيك")
        page.wait_for_timeout(200)
        q_row.locator("input").nth(1).fill("1")
        q_row.locator("input").nth(2).fill("80")
        page.wait_for_timeout(300)
        page.get_by_role("button", name="حفظ كمسودة").click()
        page.wait_for_url(lambda url: "/sales/quotations/" in url and "/new" not in url, timeout=10000)
        check("/sales/quotations/" in page.url, "quotation draft saved and detail opened")
        shot(page, shots_dir, "4_quotation_detail")

        page.goto(f"{base}/sales/quotations")
        page.wait_for_timeout(900)
        check(page.locator("table tbody tr, [role=row]").count() > 0, "quotations list shows the saved draft")

        browser.close()

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
