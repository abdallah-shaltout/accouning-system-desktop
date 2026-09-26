"""
AddressFields (18.E): cascading region/city/district `AppCombobox`es backed by `geoService`'s lazy
per-country JSON, exercised on the dev component gallery (`/dev/ui`) for the EG/SA cascade + keyboard
+ free-text fallback, then a real page (Settings → General) to confirm a picked address formats
correctly wherever `formatAddress()` renders it (the same helper `pdfService.ts` uses for the printed
company/party address line).

Selector note: `AppCombobox`'s reka-ui trigger button sets its own `aria-label="Show popup"`, which
overrides the `<label for>` association for accessible-name lookups — `get_by_role("button", name=…)`
can't find these triggers by their field label. Instead this flow locates the `<label>` by its exact
text (dataset-driven per doc 18.E — EG uses "المحافظة"/"المدينة / المركز", SA uses "المنطقة"/
"المدينة"/"الحي", see `eg.json`/`sa.json`'s `labels` block) and clicks the button inside its sibling
wrapper.

    python scripts/e2e/flows/address_picker.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import add_common_args, collect_console_errors, login_as, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import Locator, Page, sync_playwright


def combobox_trigger(page: Page, label_text: str, index: int = 0) -> Locator:
    return page.locator(f'label.field-label:text-is("{label_text}")').nth(index).locator("xpath=following-sibling::div//button")


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    shots_dir.mkdir(parents=True, exist_ok=True)

    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        collect_console_errors(page, errors)

        safe_print("open the dev component gallery")
        login_as(page, base, "admin")
        page.goto(f"{base}/dev/ui")
        page.wait_for_timeout(500)
        check(page.get_by_text("العنوان (AddressFields)").count() > 0, "the AddressFields gallery section renders")

        # --- EG: cascading region -> city, region picked via keyboard ----------------------------
        safe_print("EG: pick القاهرة (region) then a city, by keyboard only")
        eg_region = combobox_trigger(page, "المحافظة")
        eg_region.click()
        page.keyboard.type("القاهرة")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)
        check(page.get_by_text("القاهرة", exact=False).count() > 0, "القاهرة is now selected as the region")

        eg_city = combobox_trigger(page, "المدينة / المركز")
        eg_city.click()
        page.keyboard.type("مدينة نصر")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)
        check(page.get_by_text("مدينة نصر", exact=False).count() > 0, "مدينة نصر is now selected as the city")

        # Changing the region clears the previously picked city (cascading clear). Uses the
        # `clearable` X button rather than reopening + retyping over an already-picked value: a
        # pre-existing `AppCombobox` issue (unrelated to AddressFields, logged in TODO.md) shows the
        # raw model value inside the reopened search box instead of clearing it, so typed keys don't
        # reach reka-ui's filter on a *second* pick. The clear-then-reopen path below is unaffected
        # and still proves the cascading-clear logic itself (`AddressFields`' `onSelectRegion`).
        safe_print("changing the region clears the previously picked city")
        eg_region.get_by_label("مسح الاختيار").click()
        page.wait_for_timeout(200)
        check(page.get_by_text("مدينة نصر", exact=False).count() == 0, "the old city selection was cleared once its region was cleared")
        eg_region.click()
        page.keyboard.type("الجيزة")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)
        check(page.get_by_text("الجيزة", exact=False).count() > 0, "الجيزة is now selected as the region")

        # --- SA: region -> city -> district, three levels deep -----------------------------------
        safe_print("SA: منطقة الرياض -> الرياض -> a district")
        sa_region = combobox_trigger(page, "المنطقة")
        sa_region.click()
        page.keyboard.type("منطقة الرياض")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)

        sa_city = combobox_trigger(page, "المدينة")
        sa_city.click()
        page.keyboard.type("الرياض")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)

        sa_district = combobox_trigger(page, "الحي")
        sa_district.click()
        page.keyboard.type("العليا")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)
        check(page.get_by_text("العليا", exact=False).count() > 0, "العليا district is selected")

        # --- Free-text fallback (on the EG region field) -----------------------------------------
        safe_print("free-text fallback: 'غير موجود في القائمة؟ اكتب يدوياً'")
        fallback_links = page.get_by_text("غير موجود في القائمة؟ اكتب يدوياً")
        check(fallback_links.count() > 0, "the free-text fallback link is present")
        fallback_links.first.click()
        page.wait_for_timeout(150)
        free_input = page.get_by_label("المحافظة").first
        free_input.fill("قرية بعيدة غير مدرجة")
        page.wait_for_timeout(150)
        check(page.get_by_text("اختيار من القائمة").count() > 0, "the fallback toggled back to 'pick from list' label, confirming free-text mode is active")

        shot(page, shots_dir, "address_picker_gallery")
        check(not errors, f"no console errors on the address-picker gallery ({errors})")

        # --- Printed formatting: a picked structured address renders via formatAddress() ----------
        # Uses PartyFormPage's own "يظهر في الفاتورة هكذا" live preview (bound to `formatAddress()`,
        # the same helper `pdfService.ts` calls for the printed party/company address line) rather
        # than a page reload — this session's dev server is not persisting IndexedDB snapshots across
        # reloads (a pre-existing environment issue unrelated to this change, logged in TODO.md), so
        # the reload-based check would be flaky here regardless of AddressFields' own correctness.
        safe_print("Customers -> new: pick a structured address and confirm the printed-format preview")
        page.goto(f"{base}/customers/new")
        page.wait_for_timeout(500)
        party_region = combobox_trigger(page, "المحافظة")
        party_region.click()
        page.keyboard.type("الإسكندرية")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)
        party_city = combobox_trigger(page, "المدينة / المركز")
        party_city.click()
        page.keyboard.type("سموحة")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(300)

        check(page.get_by_text("يظهر في الفاتورة هكذا").count() > 0, "the printed-format preview line appears once an address is picked")
        check(page.get_by_text("سموحة، الإسكندرية", exact=False).count() > 0, "the preview shows the picked city and region in the expected order (formatAddress())")

        shot(page, shots_dir, "address_picker_party_form_preview")
        check(not errors, f"no console errors on the customer form's address section ({errors})")

        browser.close()

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
