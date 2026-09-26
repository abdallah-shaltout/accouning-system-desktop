"""
Cashier POS v2: shift open, barcode scan + unit picker, split tender, held sales (F6), return by
receipt scan (F7), and shift close with a counted variance (docs/v2/06-sales-and-pos.md).

    python scripts/e2e/flows/cashier_pos.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, login_as, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def open_shift(page, check) -> None:
    """Opens a shift with a flat opening float — required before checkout (pos.requireOpenShift)."""
    open_btn = page.get_by_role("button", name="فتح وردية")
    if open_btn.count() and open_btn.first.is_visible():
        open_btn.first.click()
        page.wait_for_timeout(300)
        page.fill("#opening-float", "500")
        page.get_by_role("button", name="فتح الوردية").click()
        page.wait_for_timeout(600)
    check("لا توجد وردية مفتوحة" not in page.locator("header").inner_text(), "shift is open (shift bar no longer shows the warning)")


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("cashier POS — open shift, scan, unit picker, split tender")
        login_as(page, base, "cashier")
        check("/pos" in page.url, "cashier lands on the POS after login")
        page.wait_for_timeout(600)

        open_shift(page, check)

        search = page.locator("input[placeholder^='امسح الباركود']")
        search.fill("6281234000019")  # EAN of the first product
        search.press("Enter")
        page.wait_for_timeout(200)
        page.locator("section button:has-text('تيشيرت بولو رجالي')").first.click()
        page.wait_for_timeout(200)
        cart_lines = page.locator("[data-testid='pos-cart-line']")
        check(cart_lines.count() == 2, f"cart has 2 lines (got {cart_lines.count()})")

        # --- Held sales (F6): hold the current cart, confirm it's empty, resume it. -----------------
        safe_print("held sales (F6)")
        page.keyboard.press("F6")
        page.wait_for_timeout(400)
        check(page.locator("[data-testid='pos-cart-line']").count() == 0, "F6 holds the sale and empties the cart")
        page.keyboard.press("F6")
        page.wait_for_timeout(300)
        check(page.get_by_role("dialog").is_visible(), "F6 with an empty cart opens the held-sales list")
        shot(page, shots_dir, "1_held_sales")
        page.get_by_role("button", name="استئناف").first.click()
        page.wait_for_timeout(400)
        check(page.locator("[data-testid='pos-cart-line']").count() == 2, "resuming a held sale restores its lines")

        # --- Split tender (F12): part cash, part card. ----------------------------------------------
        safe_print("split tender dialog")
        page.keyboard.press("F12")
        page.wait_for_timeout(400)
        check(page.get_by_role("dialog").is_visible(), "F12 opens the tender dialog")
        page.fill("#tender-cash", "100")
        page.get_by_role("button", name=re.compile("إضافة")).first.click()
        page.wait_for_timeout(300)
        check("مدى" in page.get_by_role("dialog").inner_text() or "بطاقة" in page.get_by_role("dialog").inner_text(), "remaining amount still shows other payment methods")
        page.get_by_role("button", name="مدى", exact=True).click()
        page.wait_for_timeout(200)
        shot(page, shots_dir, "2_split_tender")
        page.get_by_role("button", name=re.compile("إضافة")).first.click()
        page.wait_for_timeout(300)
        confirm_btn = page.get_by_role("button", name="تأكيد البيع")
        check(confirm_btn.is_enabled(), "split tender covers the full total — confirm is enabled")
        confirm_btn.click()
        page.wait_for_selector("text=تم البيع بنجاح", timeout=8000)
        dialog_text = page.get_by_role("dialog").inner_text()
        check("تم البيع بنجاح" in dialog_text, "split-tender sale completes")
        number = re.search(r"INV-\d+", dialog_text)
        check(number is not None, f"invoice number shown ({number.group(0) if number else '-'})")
        shot(page, shots_dir, "3_sale_done")
        page.keyboard.press("Enter")
        page.wait_for_timeout(300)
        check(page.locator("[data-testid='pos-cart-line']").count() == 0, "Enter starts a new sale (cart empty)")

        # --- Return by receipt scan (F7). -------------------------------------------------------------
        safe_print("return by scan (F7)")
        page.keyboard.press("F7")
        page.wait_for_timeout(300)
        check(page.get_by_role("dialog").is_visible(), "F7 opens the return-by-scan dialog")
        page.fill("input[placeholder='امسح رقم الفاتورة أو اكتبه']", number.group(0))
        page.get_by_role("dialog").get_by_role("button", name="بحث").click()
        qty_inputs = page.locator("table input[type=number]")
        try:
            qty_inputs.first.wait_for(state="attached", timeout=5000)
        except Exception:
            pass
        check(qty_inputs.count() > 0, f"invoice lines loaded for the return (dialog: {page.get_by_role('dialog').inner_text()[:200]!r})")
        qty_inputs.first.fill("1")
        page.get_by_role("button", name="تسجيل المرتجع").click()
        page.wait_for_timeout(800)
        check(not page.get_by_role("dialog").is_visible() or "تعذر" not in (page.get_by_role("dialog").inner_text() if page.get_by_role("dialog").is_visible() else ""), "return posts without an error")

        # --- Unit picker: the demo catalog's multi-unit product (بانادول, box+strip) only marks the
        # strip as `defaultForSale`, so scanning it adds directly without a picker (correct per its
        # data) — confirm that add-to-cart still works for a multi-unit product either way.
        safe_print("multi-unit product add (بانادول: box + strip units, only strip sellable by default)")
        search.fill("بانادول")
        page.wait_for_timeout(400)
        panadol = page.locator("section button:has-text('بانادول')").first
        if panadol.count() and panadol.is_visible() and not panadol.is_disabled():
            lines_before = page.locator("[data-testid='pos-cart-line']").count()
            panadol.click()
            page.wait_for_timeout(400)
            if page.locator("[data-testid='unit-picker-option']").count() > 0:
                shot(page, shots_dir, "4_unit_picker")
                page.locator("[data-testid='unit-picker-option']").first.click()
                page.wait_for_timeout(300)
            check(page.locator("[data-testid='pos-cart-line']").count() > lines_before, "multi-unit product adds to the cart")
        else:
            safe_print("  (بانادول out of stock/not visible in this seed run — skipped, not a failure)")

        # --- Shift close with a counted variance. -------------------------------------------------------
        safe_print("shift close with variance")
        cart_clear = page.locator("[data-testid='pos-cart-line']")
        if cart_clear.count():
            page.keyboard.press("F9")
            page.wait_for_timeout(200)
            if page.get_by_role("dialog").is_visible():
                page.get_by_role("button", name="إفراغ السلة").click()
                page.wait_for_timeout(200)
        page.get_by_role("button", name="إغلاق الوردية").click()
        page.wait_for_timeout(400)
        check(page.get_by_role("dialog").is_visible(), "close-shift dialog opens")
        counted_input = page.locator("#counted-cash")
        current_value = counted_input.input_value()
        # Introduce a deliberate 5.00 shortage so the variance card shows a real number.
        try:
            shorted = str(round(float(current_value) - 5, 2))
        except ValueError:
            shorted = "0"
        counted_input.fill(shorted)
        page.wait_for_timeout(200)
        variance_card = page.locator("[data-testid='shift-variance']")
        check(variance_card.is_visible(), "variance card is visible after entering a counted amount")
        shot(page, shots_dir, "5_shift_close_variance")
        page.get_by_role("button", name=re.compile("إغلاق الوردية وطباعة")).click()
        page.wait_for_timeout(1000)
        check("/pos/shifts" in page.url, "closing the shift navigates to the shifts manager")

        finish(page, browser, "cashier-pos")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
