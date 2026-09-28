"""Screenshot the landing for visual QA (plan 07).

Usage:
  python scripts/shots.py [url] [width] [out_dir] [--full] [--scroll]

Waits for fonts + the hero load timeline, optionally scrolls the page end to end so every
once-only ScrollTrigger reveal fires, then captures the viewport or the full page.
"""
import sys
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

args = [a for a in sys.argv[1:] if not a.startswith('--')]
flags = {a for a in sys.argv[1:] if a.startswith('--')}
url = args[0] if len(args) > 0 else 'http://localhost:3100/'
width = int(args[1]) if len(args) > 1 else 1400
out = Path(args[2] if len(args) > 2 else '.qa')
out.mkdir(parents=True, exist_ok=True)

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={'width': width, 'height': 900}, device_scale_factor=1)
    errors: list[str] = []
    page.on('console', lambda m: errors.append(f'{m.type}: {m.text}') if m.type in ('error', 'warning') else None)
    page.on('pageerror', lambda e: errors.append(f'pageerror: {e}'))
    page.goto(url, wait_until='load', timeout=90000)
    page.evaluate('document.fonts.ready')
    time.sleep(2.2)
    if '--scroll' in flags or '--full' in flags:
        height = page.evaluate('document.documentElement.scrollHeight')
        y = 0
        while y < height:
            page.mouse.wheel(0, 500)
            y += 500
            time.sleep(0.12)
        time.sleep(1.5)
        page.evaluate('window.scrollTo(0, 0)')
        time.sleep(1.2)
    name = f'shot-{width}{"-full" if "--full" in flags else ""}.png'
    page.screenshot(path=str(out / name), full_page='--full' in flags)
    sw = page.evaluate('document.documentElement.scrollWidth')
    print('saved', out / name, '| scrollWidth', sw, '| innerWidth', width)
    for e in errors:
        print(e.encode('ascii', 'backslashreplace').decode())
    browser.close()
