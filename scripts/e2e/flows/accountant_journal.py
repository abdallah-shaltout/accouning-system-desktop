"""
Accountant: manual journal entry (balanced grid, post, detail page).

    python scripts/e2e/flows/accountant_journal.py [--base URL] [--shots DIR]
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

        print("accountant manual journal")
        login_as(page, base, "accountant")
        page.goto(f"{base}/accounting/journal/new")
        page.wait_for_timeout(800)
        page.locator("input[placeholder^='مثال: سداد']").fill("اختبار: شراء أكياس تغليف")
        rows = page.locator("tbody tr")
        pick_combobox(page, rows.nth(0).locator("button[aria-haspopup=listbox]"), "6130")
        rows.nth(0).locator("input[type=number]").nth(0).fill("250")
        pick_combobox(page, rows.nth(1).locator("button[aria-haspopup=listbox]"), "1110")
        rows.nth(1).locator("input[type=number]").nth(1).fill("250")
        page.wait_for_timeout(200)
        check(page.get_by_text("القيد متوازن").is_visible(), "form shows the entry as balanced")
        shot(page, shots_dir, "3_journal_form")
        page.get_by_role("button", name=re.compile("ترحيل القيد")).click()
        page.wait_for_timeout(1000)
        check(re.search(r"/accounting/journal/je-\d+", page.url) is not None, "journal saved and detail opened")
        check("قيد يدوي" in page.inner_text("body"), "detail shows a manual entry")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
