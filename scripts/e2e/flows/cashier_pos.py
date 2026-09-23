"""
Cashier POS sale, keyboard-driven (barcode scan, checkout with F12, confirm with Enter).

    python scripts/e2e/flows/cashier_pos.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import re
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

        print("cashier POS sale")
        login_as(page, base, "cashier")
        check("/pos" in page.url, "cashier lands on the POS after login")
        page.wait_for_timeout(600)
        search = page.locator("input[placeholder^='امسح الباركود']")
        search.fill("6281234000019")  # EAN of the first product
        search.press("Enter")
        page.wait_for_timeout(200)
        page.locator("section button:has-text('تيشيرت بولو رجالي')").first.click()
        page.wait_for_timeout(200)
        cart_lines = page.locator("aside ul > li")
        check(cart_lines.count() == 2, f"cart has 2 lines (got {cart_lines.count()})")
        page.keyboard.press("F12")
        page.wait_for_timeout(400)
        check(page.get_by_role("dialog").is_visible(), "F12 opens checkout")
        shot(page, shots_dir, "1_checkout")
        page.keyboard.press("Enter")
        page.wait_for_timeout(1200)
        dialog_text = page.get_by_role("dialog").inner_text()
        check("تم البيع بنجاح" in dialog_text, "sale completes with Enter")
        number = re.search(r"INV-\d+", dialog_text)
        check(number is not None, f"invoice number shown ({number.group(0) if number else '-'})")
        shot(page, shots_dir, "2_sale_done")
        page.keyboard.press("Enter")
        page.wait_for_timeout(300)
        check(page.locator("aside ul > li").count() == 0, "Enter starts a new sale (cart empty)")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
