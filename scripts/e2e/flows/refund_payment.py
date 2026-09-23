"""
Manager: refund a line from an invoice, then pay a supplier.

    python scripts/e2e/flows/refund_payment.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, make_check, pick_combobox, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        print("manager refund + supplier payment")
        login_as(page, base, "manager")
        page.goto(f"{base}/invoices")
        page.wait_for_timeout(900)
        page.locator("tbody tr").first.click()
        page.wait_for_timeout(700)
        inv_url = page.url
        page.get_by_role("link", name=re.compile("إرجاع")).click()
        page.wait_for_timeout(700)
        page.locator("tbody input[type=number]").first.fill("1")
        page.get_by_role("button", name=re.compile("تسجيل المرتجع")).click()
        page.wait_for_timeout(300)
        page.get_by_role("dialog").get_by_role("button", name="تأكيد الإرجاع").click()
        page.wait_for_timeout(1000)
        check(page.url == inv_url, "refund returns to the invoice")
        check("المرتجعات" in page.inner_text("body"), "invoice shows the refund")
        shot(page, shots_dir, "4_invoice_after_refund")

        page.goto(f"{base}/payments/new?type=PAID")
        page.wait_for_timeout(800)
        pick_combobox(page, page.locator("button[aria-haspopup=listbox]").first, "مصنع")
        page.wait_for_timeout(600)
        has_docs = page.locator("input[type=radio]").count() > 0
        if has_docs:
            page.get_by_role("button", name=re.compile("حفظ السند")).click()
            page.wait_for_timeout(1000)
            check("/payments" in page.url and "highlight" in page.url, "supplier payment saved")
        else:
            print("  skip supplier has no open POs")
        shot(page, shots_dir, "5_payments")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
