"""
Dev helper: log in with a demo account and screenshot app routes (light + optional dark).

    python scripts/screenshot.py <out_dir> <route> [<route> ...] [--dark] [--user admin] [--size 1440x900] [--base URL]

Requires `bun run dev` (or `npx vite`) running on http://localhost:1420 (or wherever --base points)
and Python Playwright. Console errors are printed so broken pages are caught without opening a browser.
"""
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

DEFAULT_BASE = "http://localhost:1420/#"
PASSWORDS = {"admin": "admin123", "manager": "manager123", "accountant": "acc123", "cashier": "cashier123"}


def main() -> None:
    args = sys.argv[1:]
    out = Path(args.pop(0))
    dark = "--dark" in args
    user = "admin"
    size = (1440, 900)
    base = DEFAULT_BASE
    routes = []
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--dark":
            pass
        elif a == "--user":
            i += 1
            user = args[i]
        elif a == "--size":
            i += 1
            w, h = args[i].split("x")
            size = (int(w), int(h))
        elif a == "--base":
            i += 1
            base = args[i]
        else:
            routes.append(a)
        i += 1
    out.mkdir(parents=True, exist_ok=True)

    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": size[0], "height": size[1]}, device_scale_factor=1)
        errors: list[str] = []
        page.on("console", lambda m: errors.append(f"[{m.type}] {m.text}") if m.type in ("error", "warning") else None)
        page.on("pageerror", lambda e: errors.append(f"[pageerror] {e}"))

        page.goto(f"{base}/login")
        page.evaluate(f"localStorage.setItem('app_theme', '{'dark' if dark else 'light'}')")
        page.reload()
        page.wait_for_selector("input[type=password]")
        page.fill("input[type=text]", user)
        page.fill("input[type=password]", PASSWORDS.get(user, "admin123"))
        page.click("button[type=submit]")
        page.wait_for_timeout(1200)

        for route in routes:
            errors.clear()
            page.goto(f"{base}{route}")
            page.wait_for_timeout(1500)
            name = route.strip("/").replace("/", "_").replace("?", "_").replace("=", "-").replace("&", "_") or "home"
            path = out / f"{name}{'_dark' if dark else ''}.png"
            page.screenshot(path=str(path), full_page=True)
            print(f"{route} -> {path}")
            for e in errors:
                print("   ", e)
        browser.close()


if __name__ == "__main__":
    main()
