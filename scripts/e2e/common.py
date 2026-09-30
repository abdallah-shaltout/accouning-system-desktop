"""
Shared helpers for `scripts/e2e/flows/<area>.py`. Each flow file is standalone (own
`if __name__ == "__main__":` block, runnable directly) and also importable by
`scripts/e2e/run.py`, which discovers and runs every flow (or `--only <area>`).

    python scripts/e2e/flows/cashier_pos.py [--base URL] [--shots DIR]
    python scripts/e2e/run.py [--only cashier-pos] [--base URL] [--shots DIR]

Tauri target (plan 21 Part 04 phase D, P4-10): the same 21 flows also run against the real Tauri
desktop window instead of a `chromium.launch()`ed browser, by attaching to WebView2's DevTools
protocol port over CDP. `open_page()`/`finish()`/`reset_backend()`/`expect_saved_file()` below are
the only places a flow needs target-aware behavior — everything else in a flow file is unchanged
between targets (D-4).

How to launch it:

  1. `bun run db:dev` (a separate terminal) — the fixed dev MariaDB server at 127.0.0.1:3499.
  2. `bun run desktop:e2e` — starts the real Tauri window with WebView2's remote-debugging port open
     (9222) and `EQUAL_DB_URL` pointed at a dedicated `equal_e2e` database (`scripts/desktop-e2e.js`).
  3. In another terminal: `python scripts/e2e/run.py --target tauri`. This sets `EQUAL_E2E_TARGET=tauri`
     for every flow (or set it yourself for a single flow file run directly, e.g.
     `EQUAL_E2E_TARGET=tauri python scripts/e2e/flows/cashier_pos.py`).

`EQUAL_E2E_CDP` overrides the CDP endpoint `open_page()` connects to (default
`http://127.0.0.1:9222`, matching `desktop-e2e.js`'s port) — useful if something else already holds
9222. A built exe (`tauri build`) serves the app at `http://tauri.localhost/#` instead of
`http://localhost:1420/#`; pass that with `run.py --base` (see `--list` there to confirm which base
is in effect before a real run).
"""
from __future__ import annotations

import argparse
import json
import os
import time
from pathlib import Path

from playwright.sync_api import Page

DEFAULT_BASE = "http://localhost:1420/#"
PASSWORDS = {"admin": "admin123", "manager": "manager123", "accountant": "acc123", "cashier": "cashier123", "storekeeper": "store123"}

# 18.G: run.py sets this before calling each flow's run() so export_diagnostics() (called by every
# flow right before browser.close()) knows where to write this run's `.diagnostics/runs/<ts>/` dir,
# without changing every flow's run(base, shots_dir) signature.
DIAG_RUN_DIR_ENV = "EQUAL_DIAG_RUN_DIR"

# P4-10: set by `run.py --target tauri` (or by hand for a single flow) to switch open_page()/finish()/
# expect_saved_file() from launching a throwaway browser to attaching to the real Tauri window over CDP.
E2E_TARGET_ENV = "EQUAL_E2E_TARGET"
E2E_CDP_ENV = "EQUAL_E2E_CDP"
DEFAULT_CDP_ENDPOINT = "http://127.0.0.1:9222"

# D-2: where `open_page()` configures the CDP session's download behavior to save files on the
# Tauri target — native Save dialogs (rule 21) can't be driven by Playwright, and `saveFile.ts`'s
# e2e branch (D-3) uses the browser-download fallback instead, landing files here.
E2E_DOWNLOADS_DIR = Path(__file__).resolve().parents[2] / ".diagnostics" / "e2e-downloads"


def is_tauri_target() -> bool:
    return os.environ.get(E2E_TARGET_ENV) == "tauri"


def add_common_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--base", default=DEFAULT_BASE, help=f"app base URL incl. hash route prefix (default {DEFAULT_BASE})")
    parser.add_argument("--shots", default="shots", help="directory to write screenshots into (default ./shots)")


def open_page(p, viewport: dict, **kw):
    """Returns `(browser, page)` for either target (D-2):

    - Browser target (default): unchanged — `p.chromium.launch()` + `browser.new_page(viewport=…, **kw)`.
    - Tauri target (`EQUAL_E2E_TARGET=tauri`): attaches to the already-running Tauri window over CDP
      (`p.chromium.connect_over_cdp`, P4-10) instead of launching a new browser, reuses its one open
      page (`browser.contexts[0].pages[0]`), sets the viewport on it, and flags `localStorage['equal.e2e']`
      so `saveFile.ts`'s e2e branch (D-3) knows to use the browser-download fallback instead of the
      native Save dialog. The CDP session's download behavior is also configured here, so
      `expect_saved_file()` can watch `.diagnostics/e2e-downloads/` for the result.
    """
    if is_tauri_target():
        endpoint = os.environ.get(E2E_CDP_ENV, DEFAULT_CDP_ENDPOINT)
        browser = p.chromium.connect_over_cdp(endpoint)
        page = browser.contexts[0].pages[0]
        page.set_viewport_size(viewport)
        E2E_DOWNLOADS_DIR.mkdir(parents=True, exist_ok=True)
        cdp_session = page.context.new_cdp_session(page)
        cdp_session.send(
            "Browser.setDownloadBehavior",
            {"behavior": "allow", "downloadPath": str(E2E_DOWNLOADS_DIR), "eventsEnabled": True},
        )
        page.evaluate("() => { try { localStorage.setItem('equal.e2e', '1'); } catch (e) {} }")
        return browser, page

    browser = p.chromium.launch()
    page = browser.new_page(viewport=viewport, **kw)
    return browser, page


def reset_backend(page: Page, base: str) -> None:
    """Tauri target only (D-2): every flow's `ensure_demo_data()` call needs to start from fresh demo
    data the same way the browser target's fresh IndexedDB gives it — but MariaDB persists across
    flows in the Tauri target, so there is no "fresh snapshot" for free. This logs in as admin and
    runs the same NavUser dev action a developer would click ("تحميل بيانات تجريبية جديدة" ->
    `devToolsService.reloadDemoData`), which — because `usesRust('setup')` is true here — calls
    `setup_import_snapshot` with `mode: 'demo'` and the debug-only `replaceExisting: true`
    (`core/services/devToolsService.ts`), replacing whatever the database currently holds.
    """
    page.goto(f"{base}/login")
    page.evaluate("() => { try { localStorage.clear(); localStorage.setItem('equal.e2e', '1'); } catch (e) {} }")
    page.reload()
    page.wait_for_selector("input[type=password]", timeout=30000)
    page.fill("input[type=text]", "admin")
    page.fill("input[type=password]", PASSWORDS["admin"])
    page.click("button[type=submit]")
    page.wait_for_timeout(900)

    # The sidebar's NavUser trigger shows the logged-in user's display name (the demo admin account's
    # is "المدير" — src/mocks/seed/index.ts), not the "admin" username — click it to open the dropdown,
    # then the dev-only "تحميل بيانات تجريبية جديدة" item (NavUser.vue, isDev-gated).
    page.get_by_role("button").filter(has_text="المدير").first.click()
    page.get_by_text("تحميل بيانات تجريبية جديدة", exact=True).click()
    page.get_by_role("button", name="تحميل", exact=True).click()
    page.get_by_text("تم تحميل البيانات التجريبية").first.wait_for(timeout=60000)
    page.wait_for_timeout(500)
    # reloadDemoData() reloads the page itself (NavUser.vue) — wait for the app to be back up before
    # the caller navigates anywhere else.
    page.wait_for_selector("body", timeout=30000)
    page.wait_for_timeout(500)


def ensure_demo_data(page: Page, base: str) -> None:
    """
    Browser target: with no persisted IndexedDB snapshot, the router sends `/login` to `/welcome`
    (see src/router/index.ts's `isFreshInstall()` guard). Flows need real data, so load the demo
    dataset once via the welcome screen's "استكشف ببيانات تجريبية" card. A persisted snapshot
    survives reloads (src/mocks/persist.ts), so this is a no-op on later calls in the same run.

    Tauri target (D-2): MariaDB persists across flows (no fresh IndexedDB to rely on), so every
    call resets the backend to fresh demo data via `reset_backend()` instead.
    """
    if is_tauri_target():
        reset_backend(page, base)
        return
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
    # (A reload also re-runs mock DB boot — a persisted snapshot survives it; see persist.ts.) Keep
    # `equal.e2e` (D-3's saveFile.ts branch) across the clear — it isn't session state.
    page.goto(f"{base}/login")
    page.evaluate(
        "() => { const e2e = localStorage.getItem('equal.e2e'); localStorage.clear(); "
        "if (e2e) { try { localStorage.setItem('equal.e2e', e2e); } catch (err) {} } }"
    )
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


def safe_print(*args: object) -> None:
    """print() that can't crash the whole run over a console codepage that can't encode Arabic
    (e.g. Windows' default cp1252) — falls back to replacing unencodable characters instead of
    raising, which previously masked real failures/output with a UnicodeEncodeError."""
    text = " ".join(str(a) for a in args)
    try:
        print(text)
    except UnicodeEncodeError:
        import sys

        enc = sys.stdout.encoding or "ascii"
        print(text.encode(enc, errors="replace").decode(enc, errors="replace"))


def make_check(errors_sink: list[str] | None = None):
    def check(cond: bool, msg: str) -> None:
        safe_print(("  ok   " if cond else "  FAIL ") + msg)
        if not cond:
            raise FlowError(1)

    return check


def shot(page: Page, out_dir: Path, name: str) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    page.screenshot(path=str(out_dir / f"flow_{name}.png"))


def collect_console_errors(page: Page, errors: list[str]) -> None:
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)


def export_diagnostics(page: Page, area: str) -> dict | None:
    """18.G: pulls `window.__equal.diag.export()` (src/modules/diagnostics/services/diagnosticsService.ts
    — the IndexedDB-backed export already used by the support-bundle feature) and writes one JSONL
    file per flow into this run's `.diagnostics/runs/<ts>/<flow>.jsonl` (one line per LogEntry,
    across all 5 channels, so it's grep-able the same way `.diagnostics/logs/<channel>.jsonl` is).

    Call this the LAST thing before `browser.close()` in every flow — after that the IndexedDB
    ring buffer this reads from is gone. Returns the parsed export dict (channel -> LogEntry[]) so
    the caller can also inspect it (run.py uses it for perf comparisons), or None if
    EQUAL_DIAG_RUN_DIR wasn't set (e.g. a flow run standalone, outside run.py) or the export failed
    for any reason — never lets a diagnostics hiccup fail the flow itself.
    """
    run_dir = os.environ.get(DIAG_RUN_DIR_ENV)
    if not run_dir:
        return None
    try:
        exported = page.evaluate(
            "() => (window.__equal && window.__equal.diag) ? window.__equal.diag.export() : null"
        )
    except Exception as e:  # noqa: BLE001 - diagnostics export must never fail the flow
        safe_print(f"  (diagnostics export skipped for {area}: {e})")
        return None
    if exported is None:
        return None

    out_dir = Path(run_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / f"{area}.jsonl"
    with out_path.open("w", encoding="utf-8") as f:
        for channel, entries in exported.items():
            for entry in entries:
                f.write(json.dumps(entry, ensure_ascii=False) + "\n")
    return exported


def finish(page: Page, browser, area: str) -> dict | None:
    """Export diagnostics then close the browser — the one line every flow's `run()` should call
    in place of a bare `browser.close()`, so nothing forgets the export before the ring buffer
    (IndexedDB, page-scoped) goes away with the page/browser.

    Tauri target (D-2): `browser` here is a CDP connection onto the app's own browser process
    (`open_page()`), so closing it would kill the running Tauri window out from under the next flow
    — this disconnects instead, leaving the app running."""
    exported = export_diagnostics(page, area)
    if is_tauri_target():
        browser.close()  # Playwright API name is unchanged, but over CDP this only disconnects this
        # client session — see playwright.dev/python/docs/api/class-browser#browser-close: "In case
        # this browser is obtained using `browser_type.connect(...)`, clears all created contexts
        # belonging to this browser and disconnects from the browser server."
        return exported
    browser.close()
    return exported


def expect_saved_file(page: Page, action, suffix: str, timeout_ms: int = 20000):
    """Runs `action()` (a click that triggers a `saveFile()` export) and returns an object with a
    `.suggested_filename` and a `.save_as(path)`, working the same way on both targets so a flow
    doesn't need an `if is_tauri_target()` of its own (D-4 replaces `page.expect_download()` blocks
    with this in `invoice_templates.py`/`reports_v2.py`).

    - Browser target: unchanged — `page.expect_download()`, which sees the native `<a download>`
      Playwright always drives directly.
    - Tauri target: `saveFile.ts`'s e2e branch (D-3) also uses the `<a download>` fallback (native
      Save dialogs can't be automated), but the download lands through the CDP session's configured
      `downloadPath` (`open_page()`) instead of Playwright's own download manager, so this instead
      snapshots `.diagnostics/e2e-downloads/` before and after `action()` and waits for exactly one
      new file ending in `suffix` to appear (polling — CDP download-completed events aren't observed
      by Playwright's sync API the way `page.expect_download()` observes its own downloads).
    """
    if not is_tauri_target():
        with page.expect_download(timeout=timeout_ms) as dl_info:
            action()
        return dl_info.value

    E2E_DOWNLOADS_DIR.mkdir(parents=True, exist_ok=True)
    before = {f.name for f in E2E_DOWNLOADS_DIR.glob(f"*{suffix}")}
    action()

    deadline = time.time() + (timeout_ms / 1000)
    new_file: Path | None = None
    while time.time() < deadline:
        after = {f.name for f in E2E_DOWNLOADS_DIR.glob(f"*{suffix}")}
        new_names = after - before
        if new_names:
            candidate = E2E_DOWNLOADS_DIR / sorted(new_names)[-1]
            # Wait for the OS/browser to finish writing (a Chromium in-progress download uses a
            # `.crdownload` sibling, but polling name+stable size is target-agnostic and simple).
            size_a = candidate.stat().st_size
            time.sleep(0.2)
            if candidate.exists() and candidate.stat().st_size == size_a:
                new_file = candidate
                break
        time.sleep(0.2)

    if new_file is None:
        raise FlowError(1)

    class _SavedFile:
        suggested_filename = new_file.name

        @staticmethod
        def save_as(dest) -> None:
            import shutil

            shutil.copyfile(new_file, dest)

    return _SavedFile()
