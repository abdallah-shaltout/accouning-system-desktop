"""Capture real Equal screens from the desktop app's dev server for the landing page.

Usage (from the repo root, with `bun run dev` running on :1420):
  python apps/landing/scripts/app-shots.py [out_dir] [--explore]

Loads the demo dataset, signs in as admin, and screenshots each screen in SCREENS at a laptop-size
window (1280x1000 @2x, light theme). The demo company's display currency is switched to EGP for the
session (the app's own `ج.م` path in MoneyText) because the landing targets Egypt; nothing else is
staged. Output is PNG; the landing serves them through @nuxt/image (AVIF/WebP at build time).
`--explore` also saves extra candidate pages for picking new screens.
"""
import sys
import time
from pathlib import Path
from typing import Callable

from playwright.sync_api import Page, sync_playwright

BASE = 'http://localhost:1420/#'
args = [a for a in sys.argv[1:] if not a.startswith('--')]
flags = {a for a in sys.argv[1:] if a.startswith('--')}
OUT = Path(args[0] if args else 'apps/landing/public/screens')

SET_EGP = """() => {
  const pinia = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia
  const s = pinia._s.get('settings')
  if (s && s.settings) s.settings.currency = 'EGP'
}"""


def add_items(*items: tuple[str, int]) -> Callable[[Page], None]:
    """Stage a cashier mid-sale by clicking product cards. The cart persists between visits."""

    def prepare(page: Page) -> None:
        for name, times in items:
            card = page.get_by_text(name, exact=False).first
            for _ in range(times):
                card.click()
                time.sleep(0.25)

    return prepare


Clip = dict[str, float]


class Shot:
    """One landing image: a hash route, optional staging, optional crop (CSS px) and viewport size."""

    def __init__(self, route: str, prepare: Callable[[Page], None] | None = None, clip: Clip | None = None, width: int = 1280, height: int = 1000):
        self.route, self.prepare, self.clip, self.width, self.height = route, prepare, clip, width, height


SCREENS: dict[str, Shot] = {
    # Product cards — one shared portrait ratio (~0.775) so the three card images line up exactly.
    # Three items, and a window just tall enough that the totals sit right under them.
    'crop-cart': Shot('/pos', add_items(('قميص رجالي قطن', 2), ('بنطال جينز رجالي', 1), ('حزام جلد طبيعي', 1)),
                      {'x': 0, 'y': 48, 'width': 420, 'height': 542}, height=590),
    'crop-products': Shot('/products', None, {'x': 0, 'y': 0, 'width': 656, 'height': 846}, width=900),
    # 640 px wide: the report stacks into a portrait layout (sidebar collapses).
    'crop-health': Shot('/reports/business-health', None, {'x': 0, 'y': 0, 'width': 640, 'height': 826}, width=640, height=900),
    # Team panels — one shared landscape ratio (1.8).
    'crop-users': Shot('/users', None, {'x': 16, 'y': 60, 'width': 1008, 'height': 560}),
    'crop-roles': Shot('/settings/roles', None, {'x': 64, 'y': 178, 'width': 1120, 'height': 622}, width=1440),
    # Full screens — one per accordion feature (FeatureScene). 'pos' adds a fourth item to the cart above.
    'pos': Shot('/pos', add_items(('نظارة شمسية', 1))),
    'invoices': Shot('/invoices'),
    'stock': Shot('/products'),
    'purchases': Shot('/purchases'),
    'reports': Shot('/reports/profit-loss'),
}
EXPLORE: dict[str, Shot] = {
    'dashboard': Shot('/'),
    'approvals': Shot('/approvals'),
    'shifts': Shot('/pos/shifts'),
    # Its insight line hard-codes "ر.س" whatever the store currency (app bug), so it is not used.
    'analytics': Shot('/analytics'),
    'backup': Shot('/settings/backup'),
}


def boot(page: Page) -> None:
    page.goto(f'{BASE}/welcome', wait_until='load')
    time.sleep(1.5)
    if '/welcome' in page.url:
        page.get_by_role('button', name='تحميل البيانات', exact=True).click()
        page.wait_for_url(lambda url: '/welcome' not in url, timeout=60000)
        time.sleep(0.5)
    page.goto(f'{BASE}/login')
    page.evaluate('localStorage.clear()')
    page.reload()
    page.wait_for_selector('input[type=password]', timeout=60000)
    page.fill('input[type=text]', 'admin')
    page.fill('input[type=password]', 'admin123')
    page.click('button[type=submit]')
    time.sleep(1.5)


def capture(page: Page, name: str, shot: Shot) -> None:
    page.set_viewport_size({'width': shot.width, 'height': shot.height})
    # Before navigating too, so text a page builds on load (e.g. analytics insights) uses EGP.
    page.evaluate(SET_EGP)
    page.goto(f'{BASE}{shot.route}')
    time.sleep(1.8)
    page.evaluate(SET_EGP)
    if shot.prepare:
        shot.prepare(page)
    time.sleep(0.6)
    # Move the pointer off any hover state and drop focus rings before the shot.
    page.mouse.move(2, shot.height - 2)
    page.evaluate('document.activeElement && document.activeElement.blur()')
    time.sleep(0.4)
    page.screenshot(path=str(OUT / f'{name}.png'), clip=shot.clip)
    print('saved', name)


with sync_playwright() as p:
    OUT.mkdir(parents=True, exist_ok=True)
    browser = p.chromium.launch()
    ctx = browser.new_context(viewport={'width': 1280, 'height': 1000}, device_scale_factor=2, color_scheme='light')
    page = ctx.new_page()
    boot(page)
    todo = {**SCREENS, **(EXPLORE if '--explore' in flags else {})}
    for name, shot in todo.items():
        capture(page, name, shot)
    browser.close()
