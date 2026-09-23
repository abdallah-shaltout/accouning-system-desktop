"""
Dev smoke test for the main flows (mock data). Requires the dev server on :1420.

    python scripts/e2e_flows.py <screenshot_dir>

Flows: cashier POS sale via keyboard, role gating, accountant manual journal entry,
refund, supplier payment. Exits non-zero on the first failed check.
"""
import re
import sys
from pathlib import Path

from playwright.sync_api import Page, sync_playwright

BASE = "http://localhost:1420/#"
OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "shots")
OUT.mkdir(parents=True, exist_ok=True)
errors: list[str] = []


def shot(page: Page, name: str) -> None:
    page.screenshot(path=str(OUT / f"flow_{name}.png"))


def login(page: Page, user: str, password: str) -> None:
    # Hash navigation doesn't reload the SPA, so clear the stored session and reload explicitly.
    # (A reload also re-seeds the in-memory mock data.)
    page.goto(f"{BASE}/login")
    page.evaluate("localStorage.clear()")
    page.reload()
    page.wait_for_selector("input[type=password]")
    page.fill("input[type=text]", user)
    page.fill("input[type=password]", password)
    page.click("button[type=submit]")
    page.wait_for_timeout(900)


def pick_combobox(page: Page, trigger, query: str) -> None:
    trigger.click()
    page.keyboard.type(query)
    page.wait_for_timeout(150)
    page.keyboard.press("Enter")
    page.wait_for_timeout(150)


def check(cond: bool, msg: str) -> None:
    print(("  ok   " if cond else "  FAIL ") + msg)
    if not cond:
        raise SystemExit(1)


with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={"width": 1440, "height": 900})
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)

    # --- 1. Cashier sale, keyboard-driven -------------------------------------------------------
    print("cashier POS sale")
    login(page, "cashier", "cashier123")
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
    shot(page, "1_checkout")
    page.keyboard.press("Enter")
    page.wait_for_timeout(1200)
    dialog_text = page.get_by_role("dialog").inner_text()
    check("تم البيع بنجاح" in dialog_text, "sale completes with Enter")
    number = re.search(r"INV-\d+", dialog_text)
    check(number is not None, f"invoice number shown ({number.group(0) if number else '-'})")
    shot(page, "2_sale_done")
    page.keyboard.press("Enter")
    page.wait_for_timeout(300)
    check(page.locator("aside ul > li").count() == 0, "Enter starts a new sale (cart empty)")

    # --- 2. Role gating --------------------------------------------------------------------------
    print("role gating")
    page.goto(f"{BASE}/accounting/accounts")
    page.wait_for_timeout(700)
    check("/forbidden" in page.url, "cashier is blocked from the chart of accounts")
    nav = page.locator("nav[aria-label='القائمة الرئيسية']").inner_text()
    check("دليل الحسابات" not in nav and "التقارير" not in nav, "cashier sidebar hides accounting & reports")

    page.goto(f"{BASE}/invoices")
    page.wait_for_timeout(900)
    first_row = page.locator("tbody tr").first.inner_text()
    check(number.group(0) in first_row, "new invoice is first in the invoice list")

    # --- 3. Accountant: manual journal entry ----------------------------------------------------
    print("accountant manual journal")
    login(page, "accountant", "acc123")
    page.goto(f"{BASE}/accounting/journal/new")
    page.wait_for_timeout(800)
    page.locator("input[placeholder^='مثال: سداد']").fill("اختبار: شراء أكياس تغليف")
    rows = page.locator("tbody tr")
    pick_combobox(page, rows.nth(0).locator("button[aria-haspopup=listbox]"), "5300")
    rows.nth(0).locator("input[type=number]").nth(0).fill("250")
    pick_combobox(page, rows.nth(1).locator("button[aria-haspopup=listbox]"), "1110")
    rows.nth(1).locator("input[type=number]").nth(1).fill("250")
    page.wait_for_timeout(200)
    check(page.get_by_text("القيد متوازن").is_visible(), "form shows the entry as balanced")
    shot(page, "3_journal_form")
    page.get_by_role("button", name=re.compile("ترحيل القيد")).click()
    page.wait_for_timeout(1000)
    check(re.search(r"/accounting/journal/je-\d+", page.url) is not None, "journal saved and detail opened")
    check("قيد يدوي" in page.inner_text("body"), "detail shows a manual entry")

    # --- 4. Manager: refund + supplier payment --------------------------------------------------
    print("manager refund + supplier payment")
    login(page, "manager", "manager123")
    page.goto(f"{BASE}/invoices")
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
    shot(page, "4_invoice_after_refund")

    page.goto(f"{BASE}/payments/new?type=PAID")
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
    shot(page, "5_payments")

    # --- 5. Reports still balance after all of the above ---------------------------------------
    print("reports")
    page.goto(f"{BASE}/reports/trial-balance")
    page.wait_for_timeout(1200)
    check(page.get_by_text("الميزان متوازن").is_visible(), "trial balance still balances")
    page.goto(f"{BASE}/reports/balance-sheet")
    page.wait_for_timeout(1000)
    check(page.get_by_text("الأصول = الالتزامات + حقوق الملكية").is_visible(), "balance sheet still balances")
    page.goto(f"{BASE}/reports/vat")
    page.wait_for_timeout(1000)
    check("مطابق لأرصدة حسابات الضريبة" in page.inner_text("body"), "VAT report reconciles with the ledger")

    browser.close()

print("console/page errors:", errors or "none")
sys.exit(1 if errors else 0)
