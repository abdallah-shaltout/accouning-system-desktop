"""
Purchases v2 (phase 8): a full purchase order (units/discount/tax/landed costs) -> "send to
supplier" -> receiving screen (price-hidden for storekeepers) with a short delivery -> backorder,
a debit note (return) with a required reason, and the card-settlement screen.

    python scripts/e2e/flows/purchases.py [--base URL] [--shots DIR]
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

        safe_print("manager: purchase order v2 with landed costs -> send to supplier")
        login_as(page, base, "manager")
        page.goto(f"{base}/purchases/new")
        page.wait_for_timeout(700)
        pick_combobox(page, page.locator("button[aria-haspopup=listbox]").first, "مصنع")
        page.wait_for_timeout(300)
        product_combo = page.locator("table tbody tr").first.locator("button[aria-haspopup=listbox]")
        pick_combobox(page, product_combo, "قميص")
        page.wait_for_timeout(400)
        page.locator("table tbody tr").first.locator("input[type=number]").first.fill("20")
        page.wait_for_timeout(200)

        # Landed cost row (freight, spread by value).
        page.get_by_role("button", name="إضافة بند تكلفة").click()
        page.wait_for_timeout(300)
        page.get_by_placeholder("شحن…").fill("شحن الدفعة")
        page.get_by_label("المبلغ", exact=True).fill("50")
        page.wait_for_timeout(200)
        shot(page, shots_dir, "purchase_form_landed_costs")
        check(page.get_by_text("تكاليف إضافية").count() > 0, "landed cost section renders")

        page.get_by_role("button", name=re.compile("حفظ كمسودة")).click()
        page.wait_for_url(lambda url: "/purchases/" in url and "/new" not in url, timeout=15000)
        page.wait_for_timeout(500)
        po_url = page.url

        page.get_by_role("button", name=re.compile("إرسال للمورد")).click()
        page.wait_for_timeout(700)
        check("مرسل للمورد" in page.inner_text("body"), "PO status shows ORDERED after send")

        safe_print("storekeeper: receiving screen — prices hidden, short delivery -> backorder")
        login_as(page, base, "storekeeper")
        page.goto(f"{po_url}/receive")
        page.wait_for_timeout(800)
        check("سعر التكلفة" not in page.inner_text("body") or page.locator("th:has-text('سعر التكلفة')").count() == 0, "purchase prices are hidden from the storekeeper")
        shot(page, shots_dir, "receiving_screen_price_hidden")

        qty_input = page.locator("table tbody tr").first.locator("input[type=number]").first
        qty_input.click()
        qty_input.fill("15")
        page.wait_for_timeout(200)
        check(page.get_by_text("إنشاء أمر متبقٍ").count() > 0, "short delivery offers a backorder toggle")
        page.get_by_role("button", name="تأكيد الاستلام").click()
        # The click above opens a confirm dialog (useConfirm()/ConfirmDialog.vue) whose own button
        # has the same accessible name — wait for it to actually mount before clicking, rather than
        # a fixed sleep, which occasionally missed the dialog's render window and silently skipped
        # the click (falling through to wait_for_url with nothing ever confirmed).
        confirm_btn = page.get_by_role("dialog").get_by_role("button", name="تأكيد الاستلام", exact=True)
        confirm_btn.wait_for(state="visible", timeout=5000)
        confirm_btn.click()
        page.wait_for_url(lambda url: "/receive" not in url, timeout=15000)
        page.wait_for_timeout(600)
        check("مستلم" in page.inner_text("body"), "PO shows RECEIVED after receiving")

        safe_print("manager: debit note (return) — required reason, refund method")
        login_as(page, base, "manager")
        page.goto(f"{base}/purchases")
        page.wait_for_timeout(700)
        page.get_by_text("مستلمة").first.click()
        page.wait_for_timeout(400)
        received_rows = page.locator("tbody tr").filter(has_text="مستودعات")
        if received_rows.count():
            received_rows.first.click()
        else:
            page.locator("tbody tr").first.click()
        page.wait_for_timeout(600)
        return_link = page.get_by_role("link", name=re.compile("مرتجع للمورد"))
        if return_link.count():
            return_link.click()
            page.wait_for_timeout(600)
            qty_field = page.locator("table tbody input[type=number]").first
            qty_field.click()
            qty_field.fill("1")
            page.wait_for_timeout(200)
            shot(page, shots_dir, "debit_note_form")
            page.get_by_role("button", name=re.compile("تسجيل المرتجع")).click()
            page.wait_for_timeout(300)
            check(page.get_by_text("سبب الإرجاع مطلوب").count() > 0, "debit note requires a reason before submit")
            page.get_by_label("سبب الإرجاع").fill("عيوب تصنيع")
            page.wait_for_timeout(200)
            page.get_by_role("button", name=re.compile("تسجيل المرتجع")).click()
            page.wait_for_timeout(400)
            confirm = page.get_by_role("dialog").get_by_role("button", name="تأكيد")
            if confirm.count():
                confirm.click()
            page.wait_for_url(lambda url: "/return" not in url, timeout=15000)
            page.wait_for_timeout(400)
            check("المرتجعات للمورد" in page.inner_text("body"), "debit note posted and listed on the PO")

        safe_print("accountant: card settlement screen")
        login_as(page, base, "accountant")
        page.goto(f"{base}/payments/settlements")
        page.wait_for_timeout(900)
        shot(page, shots_dir, "card_settlement_screen")
        rows = page.locator("table tbody tr")
        if rows.count() > 0:
            rows.first.locator("input[type=checkbox]").check()
            page.wait_for_timeout(500)
            check(page.locator("input[type=number]").count() > 0, "deposit amount field is pre-filled after selecting a group")
            page.get_by_role("button", name=re.compile("ترحيل التسوية")).click()
            page.wait_for_timeout(700)
            check("/payments" in page.url, "settlement posts and returns to payments")
        else:
            check(page.get_by_text("لا توجد عمليات غير مسواة").count() > 0, "empty state shown when nothing is unsettled")

        browser.close()

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
