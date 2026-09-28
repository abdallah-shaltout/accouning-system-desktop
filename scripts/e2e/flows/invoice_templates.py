"""
Plan 22 — invoice templates on the print page (/print/invoices/:id):

  - A4: the gallery lists 10 templates; picking each one renders it (data-template=<id>).
  - صورة للموبايل: 10 image templates, each renders; "حفظ كصورة" downloads a real PNG (browser
    fallback of the shared saveFile helper — the desktop build opens the native Save dialog instead).
  - حراري: the gallery is hidden and the unchanged thermal receipt renders.
  - "تعيين كافتراضي" persists: after a reload the chosen A4 template opens by default. The default is
    put back to القياسي at the end so later flows see the stock invoice.
  - No console errors across all of it.

    python scripts/e2e/flows/invoice_templates.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, login_as, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import Page, sync_playwright

A4 = ["standard", "corporate", "sidebar", "banner", "swiss", "bento", "luxury", "compact", "letter", "geometric"]
IMAGE = ["instapay", "ticket", "wallet", "chat", "paper", "noir", "spotlight", "grouped", "poster", "glass"]
DOC = ".print-root [data-template]"


def rendered(page: Page) -> str:
    page.wait_for_selector(DOC, timeout=10000)
    return page.locator(DOC).first.get_attribute("data-template") or ""


def pick_all(page: Page, check, ids: list[str], label: str) -> None:
    options = page.locator("[data-testid=template-gallery] [role=radio]")
    check(options.count() == len(ids), f"{label}: the gallery lists {len(ids)} templates (got {options.count()})")
    for tid in ids:
        page.locator(f"[data-template-option={tid}]").click()
        page.wait_for_timeout(350)
        check(rendered(page) == tid, f"{label}: picking '{tid}' renders it")
        check(page.locator(f"[data-template-option={tid}]").get_attribute("aria-checked") == "true", f"{label}: '{tid}' is marked selected")


def run(base: str, shots_dir: Path) -> int:
    check = make_check()
    errors: list[str] = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1440, "height": 900}, accept_downloads=True)
        collect_console_errors(page, errors)
        login_as(page, base, "admin")

        safe_print("A4: every template renders from the gallery")
        page.goto(f"{base}/print/invoices/sample?mode=a4")
        rendered(page)
        pick_all(page, check, A4, "A4")
        shot(page, shots_dir, "invoice_templates_a4")

        safe_print("thermal: no gallery, receipt unchanged")
        page.get_by_role("button", name="حراري", exact=True).click()
        page.wait_for_timeout(400)
        check(page.locator("[data-testid=template-gallery]").count() == 0, "thermal layout hides the template gallery")
        check(rendered(page) == "thermal", "thermal layout renders the thermal receipt")

        safe_print("image: every template renders, and saves as a PNG")
        page.get_by_role("button", name="صورة للموبايل", exact=True).click()
        page.wait_for_timeout(400)
        pick_all(page, check, IMAGE, "image")
        page.locator("[data-template-option=instapay]").click()
        page.wait_for_timeout(400)
        with page.expect_download(timeout=20000) as dl_info:
            page.get_by_role("button", name="حفظ كصورة").click()
        download = dl_info.value
        check(download.suggested_filename.endswith(".png"), f"the image downloads as a .png ({download.suggested_filename})")
        saved = shots_dir / "invoice_templates_export.png"
        download.save_as(saved)
        check(saved.stat().st_size > 20_000, f"the exported PNG has real content ({saved.stat().st_size} bytes)")
        shot(page, shots_dir, "invoice_templates_image")

        safe_print("set as default persists across a reload")
        page.get_by_role("button", name="A4", exact=True).click()
        page.wait_for_timeout(300)
        page.locator("[data-template-option=corporate]").click()
        page.get_by_role("button", name="تعيين كافتراضي").click()
        page.get_by_text("تم تعيين القالب الافتراضي").first.wait_for(timeout=5000)
        page.wait_for_timeout(1000)  # let the mock backend persist its snapshot before reloading
        page.reload()
        check(rendered(page) == "corporate", "the saved default A4 template opens after a reload")
        # Put the stock default back for every later flow.
        page.locator("[data-template-option=standard]").click()
        page.get_by_role("button", name="تعيين كافتراضي").click()
        page.get_by_text("تم تعيين القالب الافتراضي").first.wait_for(timeout=5000)
        page.wait_for_timeout(1000)

        check(not errors, f"no console errors across the template flow ({errors})")
        finish(page, browser, "invoice-templates")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
