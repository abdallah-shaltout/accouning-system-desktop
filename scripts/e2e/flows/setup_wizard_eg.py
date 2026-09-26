"""
Setup wizard — Egypt path (v2 doc 18.D, plans/pending/18-countries-a11y-diagnostics/
phase-d-country-profiles.md): a genuinely fresh company defaults to Egypt in the wizard's
country/currency/tax step (now the *first* content step, before company details — see
`WIZARD_STEPS`), posts with 14% VAT and EGP, and the resulting invoice prints "ج.م" / جنيه wording
with NO ZATCA QR code (`profile.eInvoice === 'none'` for Egypt).

    python scripts/e2e/flows/setup_wizard_eg.py [--base URL] [--shots DIR]

This flow needs a CLEAN IndexedDB (no snapshot) to reach the welcome screen's "ابدأ شركتك" card, so
it clears any persisted snapshot itself before starting — it doesn't depend on run order. Mirrors
`onboarding.py`'s wizard-driving helpers (kept local/duplicated rather than shared, matching that
file's own "standalone, importable" convention) but only exercises what's EG-specific: the country
step's default, the rest of the wizard is skipped/defaulted as fast as possible, then one desk
invoice is created and printed to check the currency wording and the missing QR.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import Page, sync_playwright


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
    shots_dir.mkdir(parents=True, exist_ok=True)

    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900})
        collect_console_errors(page, errors)

        safe_print("fresh company: welcome screen -> setup wizard")
        clear_snapshot(page, base)
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
        page.get_by_text("تجزئة عامة").first.click()
        next_step("businessType")

        # --- Step 2: country/currency/tax — this is the step this flow exists to check ---------
        safe_print("step 2 — country/currency/tax defaults to مصر (EG)")
        country_select = page.get_by_label("الدولة")
        check(country_select.input_value() == "EG", "the wizard's country step defaults to مصر (EG), not السعودية")
        check(page.get_by_text("EGP").count() > 0, "the default country's base currency shows EGP")
        check(page.get_by_text("14%", exact=False).count() > 0, "the VAT switch's description mentions 14%, not 15%")
        shot(page, shots_dir, "eg_country_tax_step")
        next_step("countryTax")

        # --- Step 3: company details — tax-id label/hint should be the Egyptian one -------------
        safe_print("step 3 — company details (Egyptian tax-id label)")
        page.get_by_label("اسم المنشأة (عربي)").fill("شركتي المصرية")
        check(page.get_by_text("رقم التسجيل الضريبي").count() > 0, "the company step shows Egypt's tax-id label, not الرقم الضريبي")
        next_step("company")

        # --- Steps 4-7: fiscal year / branches / CoA / payment methods — accept defaults --------
        safe_print("steps 4-7 — accept wizard defaults")
        next_step("fiscalYear")
        next_step("branches")
        next_step("coa")
        next_step("paymentMethods")

        # --- Step 8: opening balances — skip (optional step) ------------------------------------
        safe_print("step 8 — skip opening balances")
        skip_step()

        # --- Steps 9-10: users / printing — skip (optional) -------------------------------------
        skip_step()  # users
        skip_step()  # printing

        # --- Step 11: ready ----------------------------------------------------------------------
        safe_print("step 11 — ready")
        check("جاهز" in page.locator("h2").first.inner_text(), "the wizard reaches the final 'ready' step")
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

        # --- Settings sanity: general settings shows the Egyptian tax-id label + EGP -------------
        safe_print("settings — general settings reflects Egypt")
        page.goto(f"{base}/settings/general")
        page.wait_for_timeout(800)
        check(page.get_by_text("رقم التسجيل الضريبي").count() > 0, "general settings shows Egypt's tax-id label")
        currency_select = page.locator("select").filter(has_text="جنيه مصري")
        check(currency_select.count() > 0, "general settings' currency dropdown shows EGP selected")

        # --- Desk invoice: create one, then check the printed output ---------------------------
        safe_print("desk invoice — create and print")
        page.goto(f"{base}/sales/invoices/new")
        page.wait_for_timeout(900)
        first_name_input = page.locator("table tbody tr").first.locator("input").first
        first_name_input.fill("قميص")
        page.wait_for_timeout(300)
        # If a datalist match exists, pick it; otherwise the free-text/manual-price path still lets
        # qty/price be filled directly on the row.
        row = page.locator("table tbody tr").first
        row.locator("input").nth(1).fill("1")
        page.wait_for_timeout(200)
        row.locator("input").nth(2).fill("100")
        page.wait_for_timeout(400)

        words = page.locator("text=جنيه")
        check(words.count() > 0, "amount-in-words (tafqit) shows جنيه, not ريال, for an Egyptian company")
        check(page.locator("text=ريال").count() == 0, "no Saudi-riyal wording leaks into an Egyptian invoice")

        post_btn = page.get_by_role("button", name="ترحيل الفاتورة")
        check(post_btn.is_visible(), "post button is visible")
        post_btn.click()
        page.wait_for_timeout(600)
        tender_confirm = page.get_by_role("button", name="تأكيد البيع")
        if tender_confirm.count() and tender_confirm.first.is_visible():
            tender_confirm.first.click()
            page.wait_for_timeout(600)
        page.wait_for_timeout(1000)
        check(not errors, f"posting the invoice has no console errors ({errors})")

        # Find the posted invoice's print route from the invoice list (first row).
        page.goto(f"{base}/sales/invoices")
        page.wait_for_timeout(800)
        first_row_link = page.locator("table tbody tr").first.get_by_role("link").first
        if first_row_link.count():
            first_row_link.click()
            page.wait_for_timeout(600)
        # Some invoice-detail layouts expose a direct "طباعة" action; otherwise the invoice-detail
        # page itself already shows the print preview inline, so just wait a beat either way.
        print_btn = page.get_by_role("link", name="طباعة").or_(page.get_by_role("button", name="طباعة"))
        if print_btn.count():
            print_btn.first.click()
            page.wait_for_timeout(900)
        else:
            page.wait_for_timeout(300)

        check("ج.م" in page.locator("body").inner_text() or page.get_by_text("ج.م").count() > 0, "the printed invoice shows the EGP symbol (ج.م), not a Saudi riyal glyph")
        check(page.get_by_role("img", name="رمز الاستجابة السريعة للفاتورة").count() == 0, "an Egyptian invoice never renders the ZATCA QR code")
        check(page.get_by_text("فاتورة ضريبية مبسطة").count() == 0, "an Egyptian invoice never shows the Saudi 'simplified tax invoice' title")
        shot(page, shots_dir, "eg_invoice_print")
        check(not errors, f"no console errors across the whole EG setup-wizard -> invoice-print flow ({errors})")

        finish(page, browser, "setup-wizard-eg")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
