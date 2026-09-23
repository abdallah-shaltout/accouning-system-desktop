"""
Shared helpers for `scripts/e2e/flows/<area>.py`. Each flow file is standalone (own
`if __name__ == "__main__":` block, runnable directly) and also importable by
`scripts/e2e/run.py`, which discovers and runs every flow (or `--only <area>`).

    python scripts/e2e/flows/cashier_pos.py [--base URL] [--shots DIR]
    python scripts/e2e/run.py [--only cashier-pos] [--base URL] [--shots DIR]
"""
from __future__ import annotations

import argparse
from pathlib import Path

from playwright.sync_api import Page

DEFAULT_BASE = "http://localhost:1420/#"
PASSWORDS = {"admin": "admin123", "manager": "manager123", "accountant": "acc123", "cashier": "cashier123", "storekeeper": "store123"}


def add_common_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--base", default=DEFAULT_BASE, help=f"app base URL incl. hash route prefix (default {DEFAULT_BASE})")
    parser.add_argument("--shots", default="shots", help="directory to write screenshots into (default ./shots)")


def ensure_demo_data(page: Page, base: str) -> None:
    """
    With no persisted IndexedDB snapshot, the router sends `/login` to `/welcome` (see
    src/router/index.ts's `isFreshInstall()` guard). Flows need real data, so load the demo
    dataset once via the welcome screen's "استكشف ببيانات تجريبية" card. A persisted snapshot
    survives reloads (src/mocks/persist.ts), so this is a no-op on later calls in the same run.
    """
    page.goto(f"{base}/welcome")
    page.wait_for_timeout(300)
    if "/welcome" not in page.url:
        return
    page.get_by_role("button", name="تحميل البيانات", exact=True).click()
    page.wait_for_url(lambda url: "/welcome" not in url, timeout=30000)
    page.wait_for_timeout(300)


def login(page: Page, base: str, user: str, password: str) -> None:
    ensure_demo_data(page, base)
    # Hash navigation doesn't reload the SPA, so clear the stored session and reload explicitly.
    # (A reload also re-runs mock DB boot — a persisted snapshot survives it; see persist.ts.)
    page.goto(f"{base}/login")
    page.evaluate("localStorage.clear()")
    page.reload()
    page.wait_for_selector("input[type=password]")
    page.fill("input[type=text]", user)
    page.fill("input[type=password]", password)
    page.click("button[type=submit]")
    page.wait_for_timeout(900)


def login_as(page: Page, base: str, user: str) -> None:
    login(page, base, user, PASSWORDS[user])


def pick_combobox(page: Page, trigger, query: str) -> None:
    trigger.click()
    page.keyboard.type(query)
    page.wait_for_timeout(150)
    page.keyboard.press("Enter")
    page.wait_for_timeout(150)


class FlowError(SystemExit):
    pass


def make_check(errors_sink: list[str] | None = None):
    def check(cond: bool, msg: str) -> None:
        print(("  ok   " if cond else "  FAIL ") + msg)
        if not cond:
            raise FlowError(1)

    return check


def shot(page: Page, out_dir: Path, name: str) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    page.screenshot(path=str(out_dir / f"flow_{name}.png"))


def collect_console_errors(page: Page, errors: list[str]) -> None:
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
