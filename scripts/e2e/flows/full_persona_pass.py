"""
Full persona pass (docs/v2/01-personas.md, the final v2 phase-13b consolidated story): walks each
of the 4 personas' "day" as described in the personas doc, in one continuous run, exercising the
phase-13b surfaces the other piecemeal flows don't touch on their own — the notifications drawer,
the async approvals queue, the command palette's per-module providers/context commands, and the
keyboard-shortcuts sheet — woven into the same cashier / storekeeper / accountant / owner story the
doc tells. The piecemeal flows (cashier_pos, purchases, accountant_journal, home_insights, …) each
cover their own area in depth; this one is the thin thread that ties the whole day together and
proves the newest cross-cutting surfaces work against real seeded data.

    python scripts/e2e/flows/full_persona_pass.py [--base URL] [--shots DIR]
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

        # =========================================================================================
        # 1. Cashier's day (docs/v2/01-personas.md §1): sells, tries a discount over their limit
        #    with no manager around, so it queues an async approval request instead of blocking.
        # =========================================================================================
        print("[cashier] opens the drawer, scans, requests a discount without a manager present")
        login_as(page, base, "cashier")
        check("/pos" in page.url, "cashier lands on the POS")
        page.wait_for_timeout(500)

        open_btn = page.get_by_role("button", name="فتح وردية")
        if open_btn.count() and open_btn.first.is_visible():
            open_btn.first.click()
            page.wait_for_timeout(300)
            page.fill("#opening-float", "500")
            page.get_by_role("button", name="فتح الوردية").click()
            page.wait_for_timeout(600)

        search = page.locator("input[placeholder^='امسح الباركود']")
        search.fill("6281234000019")
        search.press("Enter")
        page.wait_for_timeout(300)
        cart_lines = page.locator("[data-testid='pos-cart-line']")
        check(cart_lines.count() > 0, "a scanned item is in the cart")
        cart_lines.first.click()
        page.wait_for_timeout(200)
        page.keyboard.press("F8")
        page.wait_for_timeout(300)
        check(page.get_by_role("dialog").is_visible(), "F8 opens the line-discount dialog")
        discount_input = page.get_by_role("dialog").locator("input[type=number]").first
        discount_input.fill("90")  # comfortably over any cashier's maxDiscountPct
        page.get_by_role("dialog").get_by_role("button", name="تطبيق").click()
        page.wait_for_timeout(300)
        check("يلزم اعتماد مدير" in page.get_by_role("dialog").inner_text(), "a discount over the limit asks for manager approval")
        no_manager_link = page.get_by_text("لا يوجد مدير حالياً")
        check(no_manager_link.count() > 0, "the async approval-request escape hatch is offered")
        no_manager_link.first.click()
        page.wait_for_timeout(500)
        check(not page.get_by_role("dialog").is_visible(), "the dialog closes after the request is queued — the cashier isn't blocked")
        shot(page, shots_dir, "1_cashier_approval_request")

        # =========================================================================================
        # 2. Owner/manager's day (docs/v2/01-personas.md §4): checks notifications, decides the
        #    queued approval, browses the command palette (a new provider group), opens the
        #    keyboard-shortcuts sheet, then visits the approvals page directly.
        # =========================================================================================
        print("[manager] notifications drawer, command palette, shortcuts sheet, decides the approval")
        login_as(page, base, "manager")
        page.wait_for_timeout(500)

        bell = page.get_by_role("button", name="الإشعارات")
        check(bell.count() > 0, "the notifications bell is in the topbar")
        bell.click()
        page.wait_for_timeout(300)
        check(page.get_by_text("طلب اعتماد بانتظار المراجعة").count() > 0, "the queued approval shows up as a notification event")
        shot(page, shots_dir, "2_notifications_drawer")
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)

        # Command palette: a provider this phase adds (invoices), by group.
        print("[manager] command palette — invoices provider")
        page.keyboard.press("Control+k")
        page.wait_for_timeout(300)
        check(page.get_by_placeholder(re.compile("ابحث")).count() > 0 or page.locator("[role=dialog], [role=listbox]").count() > 0, "command palette opens")
        page.keyboard.type("INV")
        page.wait_for_timeout(400)
        check(page.get_by_text("الفواتير").count() > 0, "the invoices group appears in results")
        shot(page, shots_dir, "3_command_palette_invoices")
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)

        # Keyboard shortcuts sheet (F1) — not on POS, which has its own.
        print("[manager] keyboard shortcuts sheet")
        page.goto(f"{base}/accounting/journal")
        page.wait_for_load_state("networkidle")
        page.wait_for_timeout(400)
        page.keyboard.press("F1")
        page.wait_for_timeout(300)
        check(page.get_by_text("اختصارات لوحة المفاتيح").count() > 0, "F1 opens the global keyboard-shortcuts sheet")
        shot(page, shots_dir, "4_keyboard_shortcuts")
        page.keyboard.press("Escape")
        page.wait_for_timeout(200)

        # Approvals page: decide the cashier's queued request.
        print("[manager] approvals page — decide the queued discount request")
        page.goto(f"{base}/approvals")
        page.wait_for_load_state("networkidle")
        page.wait_for_timeout(500)
        check(page.get_by_text("طلبات الاعتماد").count() > 0, "approvals page loads")
        pending_row = page.get_by_role("button", name="اعتماد").first
        check(pending_row.count() > 0, "at least one pending approval is listed")
        shot(page, shots_dir, "5_approvals_page")
        pending_row.click()
        page.wait_for_timeout(300)
        page.get_by_role("dialog").get_by_role("button", name="اعتماد").click()
        page.wait_for_timeout(500)
        page.get_by_role("tab", name="معتمدة").click()
        page.wait_for_timeout(400)
        check(page.get_by_text("لا توجد طلبات").count() == 0, "the decided request now shows under 'معتمدة' (approved tab, not empty)")

        # =========================================================================================
        # 3. Storekeeper's day (docs/v2/01-personas.md §2): checks the stock-panel home, opens the
        #    inventory reports (whose insight box now surfaces real rule-catalogue hits).
        # =========================================================================================
        print("[storekeeper] stock-panel home, inventory report insights")
        login_as(page, base, "storekeeper")
        page.wait_for_timeout(500)
        check(page.get_by_text("قيمة المخزون").count() > 0, "storekeeper home shows the stock-value KPI")

        page.goto(f"{base}/reports/inventory")
        page.wait_for_load_state("networkidle")
        page.wait_for_timeout(600)
        check(page.get_by_text("تقرير المخزون").count() > 0, "inventory report loads")
        shot(page, shots_dir, "6_inventory_report_insights")

        # =========================================================================================
        # 4. Accountant's day (docs/v2/01-personas.md §3): prints the trial balance — "طباعة / PDF"
        #    opens the official document preview (letterhead, signatures); the native PDF path
        #    (`report.typ`) is covered by `cargo run --bin report_smoke`.
        # =========================================================================================
        print("[accountant] trial balance — official print preview")
        login_as(page, base, "accountant")
        page.wait_for_timeout(400)
        page.goto(f"{base}/reports/trial-balance")
        page.wait_for_load_state("networkidle")
        page.wait_for_timeout(600)
        check(page.get_by_text("ميزان المراجعة").count() > 0, "trial balance report loads")
        pdf_btn = page.get_by_role("button", name=re.compile("PDF"))
        check(pdf_btn.count() > 0, "report has a طباعة / PDF button")
        pdf_btn.first.click()
        page.wait_for_selector("[data-testid=report-print-dialog]", timeout=5000)
        html = page.locator("[data-testid=report-print-frame]").get_attribute("srcdoc") or ""
        check("تقرير رسمي" in html and "ميزان المراجعة" in html, "print opens the official document preview, not a print of the screen")
        page.keyboard.press("Escape")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
