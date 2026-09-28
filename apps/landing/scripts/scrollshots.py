"""Scroll like a visitor and capture the viewport at each step (plan 07).

Full-page screenshots cannot show sections that use content-visibility:auto (Chrome skips painting
off-screen content), so visual QA of the whole page uses this instead.

Usage: python scripts/scrollshots.py [url] [width] [height] [out_dir]
Writes step-NN.png per viewport and a contact sheet sheet.png.
"""
import sys
import time
from pathlib import Path

from PIL import Image
from playwright.sync_api import sync_playwright

url = sys.argv[1] if len(sys.argv) > 1 else 'http://localhost:3200/'
width = int(sys.argv[2]) if len(sys.argv) > 2 else 1400
height = int(sys.argv[3]) if len(sys.argv) > 3 else 900
out = Path(sys.argv[4] if len(sys.argv) > 4 else '.qa/scroll')
out.mkdir(parents=True, exist_ok=True)

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page(viewport={'width': width, 'height': height})
    errors: list[str] = []
    page.on('console', lambda m: errors.append(f'{m.type}: {m.text}') if m.type == 'error' else None)
    page.on('pageerror', lambda e: errors.append(f'pageerror: {e}'))
    page.goto(url, wait_until='load', timeout=90000)
    page.evaluate('document.fonts.ready')
    time.sleep(2)
    shots = []
    y = 0
    i = 0
    while True:
        total = page.evaluate('document.documentElement.scrollHeight')
        page.mouse.wheel(0, 0 if i == 0 else height * 0.85)
        time.sleep(1.6)
        path = out / f'step-{i:02d}.png'
        page.screenshot(path=str(path))
        shots.append(path)
        y = page.evaluate('window.scrollY')
        i += 1
        if y + height >= total - 2 or i > 40:
            break
    browser.close()

thumbs = [Image.open(s).resize((width // 3, height // 3)) for s in shots]
cols = 5
rows = (len(thumbs) + cols - 1) // cols
sheet = Image.new('RGB', (cols * (width // 3), rows * (height // 3)), 'white')
for n, t in enumerate(thumbs):
    sheet.paste(t, ((n % cols) * (width // 3), (n // cols) * (height // 3)))
sheet.save(out / 'sheet.png')
print('steps', len(shots), '| errors', len(errors))
for e in errors:
    print(e.encode('ascii', 'backslashreplace').decode())
