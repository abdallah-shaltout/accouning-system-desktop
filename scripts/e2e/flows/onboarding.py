"""
Onboarding v2 (phase 5, docs/v2/05-onboarding.md): a genuinely fresh company (no persisted
snapshot) goes through the welcome screen, the 11-step setup wizard (business type, company,
country/tax, fiscal year, branches, chart of accounts with live tree preview, payment methods,
opening balances, users, printing, ready), posts the opening balances entry AND closes 3900 to
zero (invariant 9), imports customers from a real .xlsx via the generic ImportWizard, then
completes a first POS sale — end to end, as one flow.

    python scripts/e2e/flows/onboarding.py [--base URL] [--shots DIR]

This flow needs a CLEAN IndexedDB (no snapshot) to reach the welcome screen's "ابدأ شركتك" card,
so it clears any persisted snapshot itself before starting — it doesn't depend on run order.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import Page, sync_playwright


def make_customers_xlsx(path: Path) -> None:
    """A real .xlsx (not a stub) with Arabic headers matching customersDescriptor's synonyms —
    proves the generic ImportWizard's template/upload/mapping round-trips a real file, not a mock."""
    import openpyxl

    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "العملاء"
    ws.append(["الاسم", "الهاتف", "البريد الإلكتروني", "الرصيد الافتتاحي", "الجهة"])
    ws.append(["أحمد سالم", "0501112222", "", 500, "مدين"])
    ws.append(["فهد العتيبي", "0503334444", "", 0, "مدين"])
    wb.save(str(path))


def set_dark_theme(page: Page, dark: bool = True) -> None:
    """Flips light/dark for a screenshot pair. `useTheme.ts` defaults `themeMode` to 'light' and
    only reacts to the OS scheme when explicitly set to 'system', so Playwright's
    `emulate_media(color_scheme=...)` alone has no effect — this mirrors what `setTheme()` /
    `apply()` (src/modules/core/controllers/useTheme.ts) do: toggle the root class and persist the
    choice, so the app's own CSS (`.dark` selectors) picks it up immediately, no reload needed."""
    page.evaluate(
        """(dark) => {
            document.documentElement.classList.toggle('dark', dark);
            document.documentElement.classList.toggle('light', !dark);
            try { localStorage.setItem('app_theme', dark ? 'dark' : 'light'); } catch {}
        }""",
        dark,
    )
    page.wait_for_timeout(200)


def clear_snapshot(page: Page, base: str) -> None:
    """Deletes the persisted IndexedDB snapshot (src/mocks/persist.ts's `mock-db` database) so the
    next /welcome visit is a genuinely fresh install, regardless of what earlier flows in this
    run (or a previous run) left behind."""
    page.goto(f"{base}/welcome")
    page.wait_for_timeout(300)
    page.evaluate("() => new Promise((resolve) => { const r = indexedDB.deleteDatabase('mock-db'); r.onsuccess = r.onerror = r.onblocked = () => resolve(true); })")
    page.evaluate("localStorage.clear()")
    page.reload()
    page.wait_for_timeout(500)


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    xlsx_path = shots_dir / "onboarding_customers.xlsx"
    shots_dir.mkdir(parents=True, exist_ok=True)
    make_customers_xlsx(xlsx_path)

    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("fresh company: welcome screen -> setup wizard")
        clear_snapshot(page, base)
        check("/welcome" in page.url, "a clean install lands on the welcome screen")
        page.get_by_role("button", name="ابدأ الآن").first.click()
        page.wait_for_timeout(1200)
        check("/setup" in page.url, "'ابدأ شركتك' opens the setup wizard")

        def next_step(label: str) -> None:
            btn = page.get_by_role("button", name="التالي")
            btn.wait_for(state="visible", timeout=10000)
            btn.click()
            page.wait_for_timeout(800)
            check(page.locator("h2").first.inner_text() != "", f"wizard advances past '{label}'")

        def skip_step() -> None:
            btn = page.get_by_role("button", name="تخطي الآن")
            btn.wait_for(state="visible", timeout=10000)
            btn.click()
            page.wait_for_timeout(800)

        # --- Step 1: business type -----------------------------------------------------------
        safe_print("step 1 — business type")
        check(page.get_by_text("نوع النشاط").count() > 0, "step 1 heading renders")
        page.get_by_text("تجزئة عامة").first.click()
        next_step("businessType")

        # --- Step 2: country/currency/tax (v2 doc 18.D: now runs BEFORE company details) --------
        # This flow tests the pre-existing Saudi path end to end, so it explicitly picks السعودية —
        # the wizard's own default is Egypt now (see setup_wizard_eg.py for that path).
        safe_print("step 2 — country, currency, tax (السعودية)")
        page.get_by_label("الدولة").select_option("SA")
        page.wait_for_timeout(200)
        check(page.get_by_text("SAR").count() > 0, "picking السعودية sets the base currency to SAR")
        next_step("countryTax")

        # --- Step 3: company details ----------------------------------------------------------
        safe_print("step 3 — company details")
        page.get_by_label("اسم المنشأة (عربي)").fill("شركتي التجريبية")
        check(page.get_by_label("الرقم الضريبي").count() > 0, "the Saudi tax-id label renders")
        next_step("company")

        # --- Step 4: fiscal year + go-live -----------------------------------------------------
        safe_print("step 4 — fiscal year & go-live date")
        next_step("fiscalYear")

        # --- Step 5: branches --------------------------------------------------------------------
        safe_print("step 5 — branches")
        check(page.get_by_text("الفرع الرئيسي").count() > 0, "the main branch is pre-filled")
        next_step("branches")

        # --- Step 6: chart of accounts (live tree preview) --------------------------------------
        safe_print("step 6 — chart of accounts template picker")
        check(page.get_by_text("معاينة الشجرة").count() > 0, "the CoA step shows a live tree preview")
        check(page.get_by_text("1110").count() > 0, "the standard template's cash account (1110) appears in the preview")
        next_step("coa")

        # --- Step 7: payment methods --------------------------------------------------------------
        safe_print("step 7 — payment methods")
        next_step("paymentMethods")

        # --- Step 8: opening balances --------------------------------------------------------------
        safe_print("step 8 — opening balances")
        check("الأرصدة الافتتاحية" in page.locator("h2").first.inner_text(), "step 8 is the opening-balances step")

        # Cash tab: enter a cash-drawer balance.
        amount_inputs = page.locator("table input[type=number]")
        check(amount_inputs.count() > 0, "the cash/banks tab lists at least one account row")
        amount_inputs.first.fill("10000")

        # Customers tab: import a real .xlsx via the generic ImportWizard.
        safe_print("importing customers from a real .xlsx")
        page.get_by_role("tab", name="العملاء").click()
        page.wait_for_timeout(300)
        page.get_by_role("button", name="استيراد من إكسل").click()
        page.wait_for_timeout(400)
        check(page.get_by_role("button", name="تحميل القالب").count() > 0, "the import wizard offers a template download")
        page.set_input_files("input[type=file]", str(xlsx_path))
        page.wait_for_timeout(600)
        page.get_by_role("button", name="متابعة").click()
        page.wait_for_timeout(500)
        check(page.get_by_text("أحمد سالم").count() > 0, "the validation table shows the imported row (auto-mapped Arabic headers)")
        shot(page, shots_dir, "import_validation_table_light")
        set_dark_theme(page)
        shot(page, shots_dir, "import_validation_table_dark")
        set_dark_theme(page, dark=False)
        page.get_by_role("button", name="استيراد (2)").click()
        page.wait_for_timeout(1500)
        check(not errors, f"customer import has no console errors ({errors})")

        # Stock tab: inline-create a product, then set its opening quantity.
        safe_print("stock tab — inline-create a missing product")
        page.get_by_role("tab", name="المخزون").click()
        page.wait_for_timeout(300)
        page.get_by_role("button", name="إضافة صنف").click()
        page.wait_for_timeout(200)
        page.locator("table tbody tr").first.locator("button[title='منتج جديد']").click()
        page.wait_for_timeout(300)
        page.get_by_label("اسم المنتج").fill("منتج تجريبي")
        page.get_by_label("رمز المنتج (SKU)").fill("SKU-ONBOARD-1")
        page.get_by_label("تكلفة الوحدة").fill("60")
        page.get_by_role("button", name="إضافة", exact=True).click()
        page.wait_for_timeout(800)
        check(page.get_by_text("منتج تجريبي").count() > 0, "the inline-created product appears in the stock row")
        qty_input = page.locator("table tbody tr").first.locator("input[type=number]").first
        qty_input.fill("50")
        page.wait_for_timeout(200)

        # Review tab: post the opening entry + closing entry.
        safe_print("review tab — post the opening entry")
        page.get_by_role("tab", name="المراجعة").click()
        page.wait_for_timeout(400)
        shot(page, shots_dir, "opening_review_light")
        set_dark_theme(page)
        shot(page, shots_dir, "opening_review_dark")
        set_dark_theme(page, dark=False)
        page.get_by_role("button", name="ترحيل الأرصدة الافتتاحية").click()
        page.wait_for_timeout(2000)
        check("تم الترحيل" in page.locator("body").inner_text(), "the opening entry posts (button flips to 'تم الترحيل')")
        check(not errors, f"posting the opening entry has no console errors ({errors})")

        # --- Steps 9-10: users / printing (skip — optional) --------------------------------------
        next_step("opening")
        skip_step()  # users
        skip_step()  # printing

        # --- Step 11: ready ------------------------------------------------------------------------
        safe_print("step 11 — ready")
        check("جاهز" in page.locator("h2").first.inner_text(), "the wizard reaches the final 'ready' step")
        check("0.00" in page.locator("body").inner_text(), "the ready step shows the opening-balance-equity account at zero (invariant 9)")
        page.get_by_role("button", name="ابدأ العمل").click()
        page.wait_for_url(lambda u: "/login" in u, timeout=15000)
        check("/login" in page.url, "finishing the wizard goes to the login screen")

        # --- Login as the admin the wizard created ------------------------------------------------
        safe_print("login as the wizard-created admin")
        page.wait_for_selector("input[type=password]")
        page.fill("input[type=text]", "admin")
        page.fill("input[type=password]", "admin123")
        page.click("button[type=submit]")
        page.wait_for_url(lambda u: "/login" not in u, timeout=15000)
        page.wait_for_timeout(800)
        check("/login" not in page.url, "logs in with the wizard-created admin credentials")

        # --- First sale via POS ----------------------------------------------------------------
        safe_print("first sale via POS")
        page.goto(f"{base}/pos")
        page.wait_for_timeout(1000)
        open_btn = page.get_by_role("button", name="فتح وردية")
        if open_btn.count() and open_btn.first.is_visible():
            open_btn.first.click()
            page.wait_for_timeout(300)
            page.fill("#opening-float", "500")
            page.get_by_role("button", name="فتح الوردية").click()
            page.wait_for_timeout(3000)

        search = page.locator("input[placeholder^='امسح الباركود']")
        search.fill("منتج تجريبي")
        page.wait_for_timeout(500)
        match = page.locator("section button:has-text('منتج تجريبي')").first
        check(match.count() > 0, "the wizard-imported product is sellable from the POS catalog")
        match.click()
        page.wait_for_timeout(300)
        cart_lines = page.locator("[data-testid='pos-cart-line']")
        check(cart_lines.count() == 1, "the product adds to the cart")

        page.keyboard.press("F12")
        page.wait_for_timeout(500)
        cash_btn = page.get_by_role("button", name="نقداً", exact=False)
        if cash_btn.count():
            cash_btn.first.click()
            page.wait_for_timeout(300)
        confirm_btn = page.get_by_role("button", name="تأكيد البيع")
        check(confirm_btn.count() > 0, "the tender dialog offers a confirm-sale action")
        confirm_btn.click()
        page.wait_for_timeout(1500)
        check("تم البيع بنجاح" in page.locator("body").inner_text(), "the first sale completes successfully")
        shot(page, shots_dir, "first_sale_completed")
        check(not errors, f"no console errors across the whole onboarding -> first-sale flow ({errors})")

        browser.close()

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
