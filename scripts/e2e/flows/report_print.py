"""
Official report printing — every "طباعة / PDF" renders a real document (letterhead, meta strip,
sectioned tables, signatures), never a print of the on-screen component:

  - Trial balance: the preview dialog opens with the official layout; signatures / orientation
    toggles re-render the document; Ctrl+P (matched by key code) opens the same preview; Escape closes.
  - Balance sheet / P&L / ledger: their hand-built statement layouts (side-by-side columns,
    section + subtotal + grand rows, account banner).
  - A report without its own spec (gross profit) still prints officially (generic conversion) with
    an automatic totals row.
  - Sales report: pre-tax net sales is below the tax-inclusive total (tax-inclusive pricing fix).
  - Journal list "دفتر اليومية PDF" lands on the day-book report with the preview already open.
  - Journal entry, invoice list and Z-report print buttons open official documents too.

    python scripts/e2e/flows/report_print.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, make_check, shot  # noqa: E402

from playwright.sync_api import Page, sync_playwright

DIALOG = "[data-testid=report-print-dialog]"
FRAME = "[data-testid=report-print-frame]"


def frame_html(page: Page) -> str:
    return page.locator(FRAME).get_attribute("srcdoc") or ""


def report_model(page: Page) -> dict:
    """The ReportDocument the dialog is rendering (dev build exposes component instances)."""
    raw = page.evaluate(
        """() => { const el = document.querySelector('[data-testid=report-print-dialog]');
             let c = el && el.__vueParentComponent; while (c && !(c.setupState && 'effective' in c.setupState)) c = c.parent;
             return c ? JSON.stringify(c.setupState.effective) : '{}'; }"""
    )
    return json.loads(raw)


def open_report_preview(page: Page, base: str, route: str) -> None:
    page.goto(f"{base}{route}")
    page.wait_for_selector("[data-testid=report-print-open]:not([disabled])", timeout=15000)
    page.wait_for_timeout(300)
    page.locator("[data-testid=report-print-open]").click()
    page.wait_for_selector(DIALOG, timeout=5000)
    page.wait_for_timeout(500)


def close_preview(page: Page) -> None:
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)
        login_as(page, base, "admin")

        print("report_print: trial balance preview is a real document")
        open_report_preview(page, base, "/reports/trial-balance")
        html = frame_html(page)
        check("تقرير رسمي" in html and "ميزان المراجعة" in html, "preview renders the official document (badge + title)")
        check('class="hdr"' in html and "الرقم الضريبي" in html, "document carries the company letterhead with the VAT number")
        check("تاريخ الإصدار" in html and "أُعدّ بواسطة" in html, "meta strip shows issue date and prepared-by")
        check('class="total"' in html, "trial balance table closes with a total row")
        check("المدير المالي" in html, "financial statements print signature boxes by default")
        check("@bottom-left" in html and "counter(pages)" in html, "every printed page gets 'page x of y'")
        check("<nav" not in html and "sidebar" not in html.lower(), "the app chrome is not part of the printed document")
        frame_font_ok = page.evaluate(
            """async () => { const d = document.querySelector('[data-testid=report-print-frame]').contentDocument;
                 await d.fonts.ready; return [...d.fonts].some(f => f.family.includes('ReportFont') && f.status === 'loaded'); }"""
        )
        check(frame_font_ok, "the report font loads inside the preview")
        shot(page, shots_dir, "report_print_trial_balance")

        page.get_by_role("switch").first.click()
        page.wait_for_timeout(300)
        check("المدير المالي" not in frame_html(page), "turning off خانات التوقيع removes the signature row")
        page.get_by_role("tab", name="عرضي").click()
        page.wait_for_timeout(300)
        check("A4 landscape" in frame_html(page), "switching to عرضي re-renders as A4 landscape")
        close_preview(page)
        check(page.locator(DIALOG).count() == 0, "Escape closes the preview")

        print("report_print: Ctrl+P opens the official preview instead of printing the screen")
        page.keyboard.press("Control+p")
        page.wait_for_selector(DIALOG, timeout=5000)
        check(page.locator(DIALOG).is_visible(), "Ctrl+P on a report opens the print preview")
        close_preview(page)

        print("report_print: statement layouts")
        open_report_preview(page, base, "/reports/balance-sheet")
        model = report_model(page)
        blocks = model.get("blocks", [])
        check(any(b.get("type") == "columns" for b in blocks), "balance sheet prints assets | liabilities + equity side by side")
        check('class="grand"' in frame_html(page), "balance sheet closes each side with a grand-total row")
        shot(page, shots_dir, "report_print_balance_sheet")
        close_preview(page)

        open_report_preview(page, base, "/reports/profit-loss")
        html = frame_html(page)
        check('class="section"' in html and 'class="subtotal"' in html and 'class="grand"' in html, "income statement has section / subtotal / grand rows")
        close_preview(page)

        page.goto(f"{base}/reports/trial-balance")
        page.wait_for_timeout(1200)
        page.locator("table tbody tr").first.click()
        page.wait_for_selector("[data-testid=report-print-open]:not([disabled])", timeout=15000)
        page.locator("[data-testid=report-print-open]").click()
        page.wait_for_selector(DIALOG, timeout=5000)
        page.wait_for_timeout(400)
        html = frame_html(page)
        check('class="banner"' in html and "رصيد أول المدة" in html, "ledger prints the account banner and the opening-balance row")
        close_preview(page)

        print("report_print: generic conversion (report without its own spec)")
        open_report_preview(page, base, "/reports/gross-profit")
        html = frame_html(page)
        check("تقرير رسمي" in html and 'class="total"' in html, "gross profit prints officially with an automatic totals row")
        close_preview(page)

        print("report_print: sales report — pre-tax net sales")
        open_report_preview(page, base, "/reports/sales")
        kpi_items = [i for b in report_model(page).get("blocks", []) if b.get("type") == "kpis" for i in b["items"]]
        vals = {i["label"]: float(i["value"].replace(",", "")) for i in kpi_items if i["value"].replace(",", "").replace(".", "").replace("-", "").isdigit()}
        net, gross, vat = vals.get("صافي المبيعات (قبل الضريبة)"), vals.get("الإجمالي شامل الضريبة"), vals.get("ضريبة القيمة المضافة")
        check(net is not None and gross is not None and vat is not None, "sales print shows net / VAT / gross KPIs")
        check(abs((net or 0) + (vat or 0) - (gross or 0)) < 0.05 and (net or 0) < (gross or 0), f"net before tax + VAT = total incl. tax ({net} + {vat} = {gross})")
        close_preview(page)

        print("report_print: journal list → day book with the preview already open")
        page.goto(f"{base}/accounting/journal")
        page.wait_for_timeout(1000)
        page.get_by_role("button", name="دفتر اليومية PDF").click()
        page.wait_for_selector(DIALOG, timeout=15000)
        check("/reports/day-book" in page.url and "print=1" not in page.url, "day-book button opens the report and its preview (flag consumed)")
        check("قيد " in frame_html(page) and "بتاريخ" in frame_html(page), "day book prints each entry under its own header band")
        close_preview(page)

        print("report_print: journal entry voucher")
        # The day-book screen links every entry to its detail page.
        page.goto(f"{base}/reports/day-book")
        page.wait_for_timeout(1500)
        page.locator("a[href*='/accounting/journal/']:not([href$='/new'])").first.click()
        page.wait_for_selector("[data-testid=journal-print]", timeout=10000)
        page.locator("[data-testid=journal-print]").click()
        page.wait_for_selector(DIALOG, timeout=5000)
        page.wait_for_timeout(300)
        html = frame_html(page)
        check("قيد يومية" in html and "أعدّه" in html and "اعتمده" in html, "journal entry prints as a voucher with prepared / reviewed / approved boxes")
        check("فقط" in html, "journal voucher states the amount in words")
        close_preview(page)

        print("report_print: invoice register")
        page.goto(f"{base}/invoices")
        page.wait_for_timeout(1200)
        page.locator("[data-testid=invoices-print]").click()
        page.wait_for_selector(DIALOG, timeout=5000)
        page.wait_for_timeout(300)
        html = frame_html(page)
        check("سجل فواتير المبيعات" in html and "A4 landscape" in html, "invoice list prints an official landscape register")
        close_preview(page)

        print("report_print: Z report")
        page.goto(f"{base}/pos/shifts")
        page.wait_for_timeout(1200)
        z = page.get_by_role("link", name="تقرير Z")
        if z.count():
            z.first.click()
            page.wait_for_selector("[data-testid=shift-print]:not([disabled])", timeout=10000)
            page.locator("[data-testid=shift-print]").click()
            page.wait_for_selector(DIALOG, timeout=5000)
            page.wait_for_timeout(300)
            html = frame_html(page)
            check("تقرير إغلاق الوردية" in html and "تسوية النقدية" in html and "الكاشير" in html, "Z report prints the cash reconciliation with signatures")
            close_preview(page)
        else:
            print("  (skip: no closed shift in this run)")

        browser.close()

    print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
