"""
Accountant: manual journal entry (spreadsheet grid, "=" balancing, post, detail page), reversal
dialog (date + reason), and the fiscal-year closing wizard.

    python scripts/e2e/flows/accountant_journal.py [--base URL] [--shots DIR]
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

        safe_print("accountant manual journal (grid entry)")
        login_as(page, base, "accountant")
        page.goto(f"{base}/accounting/journal/new")
        page.wait_for_timeout(800)
        page.locator("input[placeholder^='مثال: سداد']").fill("اختبار: شراء أكياس تغليف")
        rows = page.locator("tbody tr")
        pick_combobox(page, rows.nth(0).locator("button[aria-haspopup=listbox]"), "6130")
        rows.nth(0).locator("input.num").nth(0).fill("250")
        pick_combobox(page, rows.nth(1).locator("button[aria-haspopup=listbox]"), "1110")
        # "=" balancing shortcut: typing "=" in the credit cell should fill the remaining difference (250).
        credit_cell = rows.nth(1).locator("input.num").nth(1)
        credit_cell.fill("=")
        page.wait_for_timeout(200)
        check(credit_cell.input_value() == "250", f"'=' balancing shortcut filled the difference (got {credit_cell.input_value()!r})")
        check(page.get_by_text("القيد متوازن").is_visible(), "form shows the entry as balanced")
        shot(page, shots_dir, "3_journal_form")
        page.get_by_role("button", name=re.compile(r"^ترحيل القيد ")).click()
        page.wait_for_url(re.compile(r"/accounting/journal/je-"), timeout=10000)
        check(re.search(r"/accounting/journal/je-\d+", page.url) is not None, "journal saved and detail opened")
        page.get_by_text("قيد يدوي").first.wait_for(timeout=5000)
        check("قيد يدوي" in page.inner_text("body"), "detail shows a manual entry")

        safe_print("reversal dialog (date + reason)")
        page.get_by_role("button", name=re.compile("عكس القيد")).first.click()
        page.wait_for_timeout(300)
        dialog = page.get_by_role("dialog")
        dialog.get_by_role("button", name=re.compile("^عكس القيد$")).click()  # submit with no reason first
        page.wait_for_timeout(200)
        check(page.get_by_text("سبب العكس مطلوب").is_visible(), "reversal requires a reason")
        dialog.locator("textarea").fill("خطأ في اختيار الحساب — اختبار آلي")
        dialog.get_by_role("button", name=re.compile("^عكس القيد$")).click()
        page.wait_for_timeout(1000)
        page.get_by_text("هذا القيد يعكس").wait_for(timeout=5000)
        check(re.search(r"/accounting/journal/je-\d+", page.url) is not None, "reversal entry created and opened")
        check("هذا القيد يعكس" in page.inner_text("body"), "reversal entry links back to the original")

        safe_print("journal list: day grouping + drafts view")
        page.goto(f"{base}/accounting/journal")
        page.wait_for_timeout(600)
        check(page.get_by_text("مسودات بحاجة لترحيل").is_visible(), "drafts-to-post saved view is offered")
        shot(page, shots_dir, "4_journal_list")

        safe_print("fiscal-year close wizard (pre-checks screen)")
        page.goto(f"{base}/accounting/fiscal-years")
        page.wait_for_timeout(600)
        lock_button = page.get_by_role("button", name=re.compile("إقفال السنة")).first
        if lock_button.count():
            lock_button.click()
            page.wait_for_timeout(500)
            check(page.get_by_text("ميزان المراجعة متوازن").is_visible(), "closing wizard shows the trial-balance pre-check")
            shot(page, shots_dir, "5_close_wizard")
            page.keyboard.press("Escape")

        browser.close()

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
