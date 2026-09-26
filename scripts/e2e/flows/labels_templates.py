"""
Phase 11b: remaining PDF document templates + the label builder
(docs/v2/12-documents-pdf-excel.md §3-4, docs/v2/07-products-and-inventory.md §6).

Covers, against the browser dev-mode fallback (no Tauri IPC available here, so every
`pdfService.render*` call takes the "not isTauri()" branch — a toast + `{ ok: false }` — and the
caller falls back to the v1 browser print route, exactly per docs/v2/12 §2's documented dev-mode
behavior):

  - A voucher's print button goes through `pdfService` (not straight to the old `/print/vouchers/`
    route) and doesn't throw — it should fall back to that same v1 route once `pdfService.render`
    reports `{ ok: false }` outside Tauri.
  - A purchase order's print button, same check.
  - The label builder loads, lets you search and pick a product, shows a template picker, and the
    print action doesn't error (outside Tauri it shows a toast instead of invoking Rust).

    python scripts/e2e/flows/labels_templates.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, login_as, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import sync_playwright


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("admin: voucher print button goes through pdfService (browser fallback, no crash)")
        login_as(page, base, "admin")
        page.goto(f"{base}/vouchers")
        page.wait_for_timeout(800)
        first_row = page.locator("tbody tr").first
        check(first_row.count() > 0, "voucher list has at least one row")
        first_row.click()
        page.wait_for_timeout(600)
        check("/vouchers/" in page.url, "opened a voucher detail page")
        print_btn = page.get_by_role("button", name="طباعة")
        check(print_btn.count() > 0, "voucher detail page has a طباعة button")
        print_btn.first.click()
        # Outside Tauri, pdfService.render() shows an info toast ("الفعلي متاح في نسخة سطح
        # المكتب") and returns ok:false, so the page's print() falls back to the v1
        # /print/vouchers/:id route — confirm that fallback actually happens, not a dead click.
        page.wait_for_url(lambda url: "/print/vouchers/" in url, timeout=8000)
        check("/print/vouchers/" in page.url, "voucher print falls back to the v1 print route outside Tauri")
        shot(page, shots_dir, "voucher_print_fallback")
        page.go_back()
        page.wait_for_timeout(500)

        safe_print("manager: purchase order print button goes through pdfService (browser fallback, no crash)")
        login_as(page, base, "manager")
        page.goto(f"{base}/purchases")
        page.wait_for_timeout(800)
        ordered_row = page.locator("tbody tr", has=page.get_by_text("مرسل", exact=False)).first
        target_row = ordered_row if ordered_row.count() > 0 else page.locator("tbody tr").first
        check(target_row.count() > 0, "purchase order list has at least one row")
        target_row.click()
        page.wait_for_timeout(600)
        check("/purchases/" in page.url, "opened a purchase order detail page")
        po_print_btn = page.get_by_role("button", name=re.compile("طباعة أمر الشراء"))
        if po_print_btn.count() > 0:
            po_print_btn.first.click()
            page.wait_for_url(lambda url: "/print/purchases/" in url, timeout=8000)
            check("/print/purchases/" in page.url, "purchase order print falls back to the v1 print route outside Tauri")
        else:
            safe_print("  (skip: this purchase order isn't in ORDERED status, no طباعة أمر الشراء button)")

        safe_print("storekeeper: label builder loads, product search + pick, template picker, print doesn't error")
        login_as(page, base, "storekeeper")
        page.goto(f"{base}/catalog/labels")
        page.wait_for_timeout(900)
        check(page.get_by_text("منشئ الملصقات").count() > 0, "label builder page loaded")

        search_input = page.locator("input[type=search]")
        check(search_input.count() > 0, "label builder has a product search box")
        search_input.first.fill("قميص")
        page.wait_for_timeout(400)
        results = page.locator("button", has_text="SKU")
        # Search results render as buttons with the SKU shown; fall back to any dropdown button
        # under the search box if that text-based locator doesn't match this run's seed data.
        candidate = results.first if results.count() > 0 else page.locator("div.absolute.z-10 button").first
        check(candidate.count() > 0, "product search returns at least one result")
        candidate.click()
        page.wait_for_timeout(300)
        check(page.locator("table tbody tr").count() > 0, "picked product appears in the pick table")

        template_select = page.locator("select").first
        check(template_select.count() > 0, "label builder has a template picker")
        shot(page, shots_dir, "label_builder_preview")

        print_btn = page.get_by_role("button", name=re.compile("طباعة"))
        check(print_btn.count() > 0, "label builder has a طباعة button")
        print_btn.first.click()
        page.wait_for_timeout(600)
        # Outside Tauri this shows a toast ("الفعلي متاح في نسخة سطح المكتب") rather than
        # navigating anywhere or throwing — the flow's real assertion is "no console error",
        # checked below via `errors`.
        check(page.get_by_text("سطح المكتب", exact=False).count() > 0, "print action shows the desktop-only toast outside Tauri (no crash)")

        finish(page, browser, "labels-templates")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
