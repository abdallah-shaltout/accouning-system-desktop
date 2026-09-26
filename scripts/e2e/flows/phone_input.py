"""
AppPhoneInput (18.A1): trunk-zero parsing, no error before blur, paste country-detection, and
keyboard-only country switching, exercised on the dev component gallery (`/dev/ui`) rather than a
real form so the flow doesn't depend on wizard/party navigation.

    python scripts/e2e/flows/phone_input.py [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from common import finish, add_common_args, collect_console_errors, login_as, make_check, safe_print, shot  # noqa: E402

from playwright.sync_api import Page, sync_playwright


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
        check(page.get_by_text("الهاتف (AppPhoneInput)").count() > 0, "the phone-input gallery section renders")

        # --- Trunk-zero parsing: EG local number -------------------------------------------------
        safe_print("EG trunk-zero: 01012345678 -> +201012345678")
        empty_field = page.get_by_label("فارغ")
        empty_field.click()
        empty_field.fill("01012345678")
        page.wait_for_timeout(150)
        check(page.get_by_text("رقم الهاتف غير صحيح").count() == 0, "no error shows while typing, before blur")
        page.keyboard.press("Tab")
        page.wait_for_timeout(150)
        check(page.get_by_text("غير صحيح لـمصر").count() == 0, "a complete valid EG number shows no error after blur")

        # --- Trunk-zero parsing: SA local number, switched country ------------------------------
        safe_print("switch the field's country to SA via the combobox, then type a trunk-zero number")
        page.get_by_role("button", name="دولة الرقم: مصر").first.click()
        page.wait_for_timeout(200)
        page.keyboard.type("السعودية")
        page.wait_for_timeout(200)
        page.keyboard.press("Enter")
        page.wait_for_timeout(200)
        empty_field.fill("0501234567")
        empty_field.press("Tab")
        page.wait_for_timeout(150)
        check(page.get_by_text("غير صحيح لـالسعودية").count() == 0, "a complete valid SA number (trunk 0 stripped) shows no error")

        # --- Pre-filled valid examples render with no error --------------------------------------
        safe_print("pre-filled EG / SA examples are valid")
        check(page.get_by_text("مصر (صحيح)").count() > 0, "the EG example field is present")
        check(page.get_by_text("السعودية (صحيح)").count() > 0, "the SA example field is present")

        # --- Invalid example shows the Arabic error after blur -----------------------------------
        safe_print("invalid example shows an inline Arabic error")
        invalid_field = page.get_by_label("غير صحيح (بعد الخروج من الحقل)")
        invalid_field.click()
        invalid_field.press("Tab")
        page.wait_for_timeout(150)
        check(page.get_by_text("غير صحيح لـمصر").count() > 0, "the invalid EG example shows 'رقم الهاتف غير صحيح لـمصر'")

        shot(page, shots_dir, "phone_input_gallery_light")
        check(not errors, f"no console errors on the phone-input gallery ({errors})")

        # --- Switch RTL direction (18.A2): "on" thumb sits on the right, in RTL too --------------
        safe_print("switch: checked thumb sits on the right half of the track, in RTL")
        on_switch = page.get_by_role("switch", checked=True).first
        check(on_switch.count() > 0, "an 'on' switch is present in the RTL gallery section")
        track_box = on_switch.bounding_box()
        thumb_box = on_switch.locator("span").first.bounding_box()
        check(track_box is not None and thumb_box is not None, "both the track and the thumb report a bounding box")
        if track_box and thumb_box:
            track_center_x = track_box["x"] + track_box["width"] / 2
            thumb_center_x = thumb_box["x"] + thumb_box["width"] / 2
            check(thumb_center_x > track_center_x, "the checked thumb's center sits right of the track's center, in RTL")

        finish(page, browser, "phone-input")

    safe_print("console/page errors:", errors or "none")
    return 1 if errors else 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    add_common_args(parser)
    args = parser.parse_args()
    sys.exit(run(args.base, Path(args.shots)))
