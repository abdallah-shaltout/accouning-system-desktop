"""Render public/og.png (1200x630) and public/logo.png (512x512) from the running dev server.

Usage: python scripts/og.py [base_url]   (default http://localhost:3100)
Arabic needs real text shaping, so the image is rendered by the browser, not drawn with PIL.
"""
import sys
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

base = sys.argv[1] if len(sys.argv) > 1 else 'http://localhost:3100'
public = Path(__file__).resolve().parent.parent / 'public'

LOGO_HTML = """<!doctype html><html><body style="margin:0;background:transparent">
<div style="width:512px;height:512px;border-radius:112px;background:#F3553B;display:grid;place-items:center">
<svg width="300" height="233" viewBox="0 0 36 28"><rect x="0" y="2" width="30" height="9" rx="4.5" fill="#fff"/>
<rect x="6" y="17" width="30" height="9" rx="4.5" fill="#fff"/></svg></div></body></html>"""

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={'width': 1200, 'height': 630})
    page.goto(f'{base}/og-card', wait_until='load', timeout=90000)
    page.evaluate('document.fonts.ready')
    time.sleep(2.5)
    page.screenshot(path=str(public / 'og.png'), clip={'x': 0, 'y': 0, 'width': 1200, 'height': 630})

    logo = browser.new_page(viewport={'width': 512, 'height': 512})
    logo.set_content(LOGO_HTML)
    logo.screenshot(path=str(public / 'logo.png'), omit_background=True)
    browser.close()
print('wrote', public / 'og.png', 'and', public / 'logo.png')
