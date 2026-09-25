"""Vectorize logo/fav.webp into src/assets/brand/mark.svg (docs/v2/16-equal-rebrand-and-ui-kit.md
Phase B). fav.webp is only 192x192 with an alpha channel that isn't usable as a mask, so we
composite it onto white, enlarge, threshold to a binary ink mask, and trace that with potrace
into one `currentColor` path — one vector that works at any size and in either theme.

Usage: python scripts/brand/trace-mark.py [--threshold N] [--out PATH]

Pipeline (matches the doc's recorded feasibility check exactly):
  composite onto white -> grayscale -> Lanczos x4 (768px) -> threshold -> potrace
  (turdsize=3, alphamax=1.0, opticurve=True, opttolerance=0.2) -> one <path> in a 0 0 768 768 viewBox

Gotchas recorded in the doc, both handled below:
  - potrace's `image_to_bmp`/Bitmap traces True (foreground) pixels, so the INK must be passed as
    True and the background as False - i.e. pass the *inverted* threshold mask (ink lighter than
    the paper turns into a filled block otherwise).
  - a threshold that's too dark (< 170) wipes out the green mark's internal face hatching detail.
"""
import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image
import potrace

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "logo" / "fav.webp"
DEFAULT_OUT = ROOT / "src" / "assets" / "brand" / "mark.svg"
ENLARGE_TO = 768


MARGIN = 8  # px of white padding added before tracing


def build_mask(threshold: int) -> np.ndarray:
    im = Image.open(SRC).convert("RGBA")
    # Composite onto opaque white first: fav.webp's alpha channel isn't a usable mask per the doc
    # (the mark itself sits on an opaque white square already), so this is just safety for any
    # translucent edge pixels from the webp encode.
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
    composited = Image.alpha_composite(bg, im).convert("L")
    enlarged = composited.resize((ENLARGE_TO, ENLARGE_TO), Image.LANCZOS)
    arr = np.asarray(enlarged)
    # Ink (the green mark) is darker than the white paper, so ink pixels are the ones BELOW the
    # threshold. potrace traces True/foreground pixels, so the ink mask below is what we pass in
    # directly (not its logical inverse) to get the mark traced, not the background.
    mask = arr < threshold
    # The turban's outline runs close enough to the source image's edge that, without padding,
    # potrace's outermost contour follows the bitmap's own border and comes out as a a full-canvas
    # frame subpath (`M0 0H{w}V{h}H0z`-shaped) wrapping every real contour in an outer even-odd
    # ring. That frame renders as a solid filled square wherever the mark is later recolored on a
    # background of its own (e.g. white-on-tile app icons), instead of just the mark's ink. Padding
    # with a white (background) margin keeps every real contour fully enclosed, so no contour
    # needs the canvas edge itself as a boundary.
    padded = np.zeros((ENLARGE_TO + 2 * MARGIN, ENLARGE_TO + 2 * MARGIN), dtype=bool)
    padded[MARGIN : MARGIN + ENLARGE_TO, MARGIN : MARGIN + ENLARGE_TO] = mask
    return padded


def trace(mask: np.ndarray) -> potrace.Path:
    # Padding the mask with a False (background) margin (see build_mask) still isn't enough on its
    # own: potrace's Bitmap.trace() always emits the bitmap's own outer border as the first curve
    # (an even-odd "root" ring the size of the whole padded canvas), even though nothing was traced
    # there. Left in, it renders as a solid filled square behind the real mark wherever `currentColor`
    # is used on its own background layer (e.g. white-on-tile app icons) instead of a transparent one.
    # It's dropped below, right after tracing, rather than worked around at render time.
    bmp = potrace.Bitmap(mask)
    return bmp.trace(turdsize=3, alphamax=1.0, opticurve=True, opttolerance=0.2)


def path_to_svg_d(path: potrace.Path, canvas_size: int) -> str:
    parts = []
    for curve in path:
        start = curve.start_point
        # Drop the synthetic outer-border ring: its 4 corners sit exactly on the padded canvas
        # bounds (0 and canvas_size), which no real traced contour ever does since the mask has a
        # background margin on every side.
        corners = [start] + [s.end_point for s in curve.segments]
        if all((round(p.x) in (0, canvas_size)) or (round(p.y) in (0, canvas_size)) for p in corners):
            continue
        parts.append(f"M{start.x:.2f},{start.y:.2f}")
        for segment in curve.segments:
            if segment.is_corner:
                c = segment.c
                end = segment.end_point
                parts.append(f"L{c.x:.2f},{c.y:.2f} L{end.x:.2f},{end.y:.2f}")
            else:
                c1, c2 = segment.c1, segment.c2
                end = segment.end_point
                parts.append(f"C{c1.x:.2f},{c1.y:.2f} {c2.x:.2f},{c2.y:.2f} {end.x:.2f},{end.y:.2f}")
        parts.append("Z")
    return " ".join(parts)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--threshold", type=int, default=200, help="grayscale cutoff, 0-255 (doc: pick from 190/215/235 by eye; must stay >= 170 or face hatching is lost)")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()

    if args.threshold < 170:
        print(f"warning: threshold {args.threshold} < 170 will likely wipe out the face hatching (see doc)", file=sys.stderr)

    mask = build_mask(args.threshold)
    path = trace(mask)
    d = path_to_svg_d(path, ENLARGE_TO + 2 * MARGIN)

    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {ENLARGE_TO} {ENLARGE_TO}">\n'
        f'  <g transform="translate({-MARGIN},{-MARGIN})">\n'
        f'    <path fill="currentColor" fill-rule="evenodd" d="{d}" />\n'
        "  </g>\n"
        "</svg>\n"
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(svg, encoding="utf-8")
    print(f"wrote {args.out} ({len(svg)} bytes, threshold={args.threshold})")


if __name__ == "__main__":
    main()
