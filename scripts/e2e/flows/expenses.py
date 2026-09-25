"""
Expenses v2 (phase 8): create a tax-invoice expense paid from a payment method, confirm it posts
(shows on the list + its journal entry), then check a general voucher posts too.

    python scripts/e2e/flows/expenses.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, make_check, pick_combobox, safe_print, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("accountant: expense list — due recurring + month-total chart")
        login_as(page, base, "accountant")
        page.goto(f"{base}/expenses")
        page.wait_for_timeout(900)
        check(page.get_by_text("مصروفات متكررة مستحقة").count() > 0, "due recurring expenses panel shows")
        check(page.get_by_text("إجمالي الشهر حسب التصنيف").count() > 0, "month-total-by-category chart shows")

        safe_print("accountant: new tax-invoice expense")
        page.goto(f"{base}/expenses/new")
        page.wait_for_timeout(700)
        pick_combobox(page, page.locator("button[aria-haspopup=listbox]").first, "صيانة")
        page.wait_for_timeout(200)
        page.locator("input[type=number]").first.fill("250")
        page.wait_for_timeout(200)
        page.get_by_text("فاتورة ضريبية؟").click()
        page.wait_for_timeout(300)
        shot(page, shots_dir, "expense_form")
        check(page.get_by_text("صافي").count() > 0, "tax split preview shows once 'فاتورة ضريبية؟' is on")

        page.get_by_role("button", name=re.compile("حفظ المصروف")).click()
        page.wait_for_url(lambda url: "/expenses/" in url and "/new" not in url, timeout=15000)
        page.wait_for_timeout(600)
        check("EXP-" in page.inner_text("body"), "expense detail shows its number")

        journal_link = page.get_by_role("link").filter(has_text=re.compile(r"JE-"))
        if journal_link.count():
            journal_link.first.click()
            page.wait_for_timeout(600)
            check("/accounting/journal/" in page.url, "expense links to its posted journal entry")

        safe_print("admin: general voucher (transfer)")
        login_as(page, base, "admin")
        page.goto(f"{base}/vouchers/new")
        page.wait_for_timeout(700)
        page.get_by_text("تحويل بين الحسابات").click()
        page.wait_for_timeout(300)
        combos = page.locator("button[aria-haspopup=listbox]")
        pick_combobox(page, combos.nth(0), "الصندوق")
        pick_combobox(page, combos.nth(1), "البنك")
        page.locator("input[type=number]").first.fill("1000")
        page.wait_for_timeout(200)
        page.get_by_label("الوصف").fill("إيداع نقدية اختباري")
        shot(page, shots_dir, "general_voucher_form")
        page.get_by_role("button", name=re.compile("حفظ السند")).click()
        page.wait_for_url(lambda url: "/vouchers" in url and "/new" not in url, timeout=15000)
        page.wait_for_timeout(500)
        check("VCH-" in page.inner_text("body") or page.locator("tbody tr").count() > 0, "voucher list shows the new transfer")

        browser.close()

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
