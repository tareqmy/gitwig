#!/usr/bin/env python3
"""Rasterize the brand SVGs to PNG with correct transparency.

Two traps this works around, both of which silently corrupt the output:

1. `qlmanage` always emits a *square* thumbnail. A non-square SVG gets clipped
   rather than letterboxed, which quietly amputated the mark's trailing commit
   dots. Every source is therefore wrapped in a square canvas before rendering.

2. `qlmanage` composites onto opaque white, so "make white transparent" cannot
   recover partial alpha -- the dots at opacity 0.75/0.45 come back as opaque
   pastels that glow on a dark ground. Instead each source is rendered twice,
   over white and over black, and the true alpha is solved per pixel:

       over white:  W = C*a + (1-a)
       over black:  B = C*a
       =>           a = 1 - (W - B)      and   C = B / a

   `-alpha Disassociate` performs that final divide, since B is exactly the
   premultiplied form of the colour.

ImageMagick's own SVG renderer is not an option here: it drops masked geometry
and emits a bare plate.

Usage (from the repository root):

    python3 branding/rasterize.py
"""

import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

# source svg -> (output png, height in px; width follows the content aspect).
# lockup.png is deliberately absent: it is the hand-authored reference the
# vectors are matched against, so regenerating it would destroy the baseline.
TARGETS = [
    ("branding/logo-mark.svg", "branding/logo-mark.png", 480),
    ("branding/app-icon.svg", "resources/icon.png", 1024),
]
RENDER_AT = 1600


def need(tool):
    if not shutil.which(tool):
        sys.exit(f"{tool} not found; it is required to rasterize the brand assets.")


def square_viewbox(svg):
    """Re-wrap an SVG in a square viewBox so qlmanage cannot clip it."""
    match = re.search(r'viewBox="([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)"', svg)
    if not match:
        sys.exit("no viewBox found")
    x, y, w, h = (float(v) for v in match.groups())
    side = max(w, h) * 1.1                      # 10% margin, trimmed off later
    cx, cy = x + w / 2, y + h / 2
    box = f'viewBox="{cx - side / 2:.3f} {cy - side / 2:.3f} {side:.3f} {side:.3f}"'
    svg = svg[: match.start()] + box + svg[match.end() :]
    return re.sub(r'\swidth="[^"]*"\s+height="[^"]*"',
                  f' width="{RENDER_AT}" height="{RENDER_AT}"', svg, count=1)


def with_backdrop(svg, colour):
    """Insert an opaque full-bleed rect as the first child."""
    box = re.search(r'viewBox="([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)"', svg)
    x, y, w, h = box.groups()
    rect = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{colour}"/>'
    return re.sub(r"(<svg[^>]*>)", r"\1" + rect, svg, count=1)


def render(svg_text, tmp, tag):
    src = tmp / f"{tag}.svg"
    src.write_text(svg_text)
    out = tmp / tag
    out.mkdir(exist_ok=True)
    subprocess.run(
        ["qlmanage", "-t", "-s", str(RENDER_AT), "-o", str(out), str(src)],
        capture_output=True, check=False,
    )
    png = out / f"{tag}.svg.png"
    if not png.exists():
        sys.exit(f"qlmanage produced no output for {tag}")
    return png


def main():
    for tool in ("qlmanage", "magick"):
        need(tool)
    if not pathlib.Path("branding/logo-mark.svg").exists():
        sys.exit("run this from the repository root")

    for source, target, height in TARGETS:
        svg = square_viewbox(pathlib.Path(source).read_text())
        with tempfile.TemporaryDirectory() as td:
            tmp = pathlib.Path(td)
            on_white = render(with_backdrop(svg, "#ffffff"), tmp, "white")
            on_black = render(with_backdrop(svg, "#000000"), tmp, "black")
            alpha = tmp / "alpha.png"
            # a = 1 - (W - B)
            subprocess.run(
                ["magick", str(on_white), str(on_black), "-compose", "Difference",
                 "-composite", "-colorspace", "Gray", "-negate", str(alpha)],
                check=True,
            )
            # Un-premultiply: C = B / a. (`-alpha Disassociate` looks like the
            # right tool but zeroes the channel outright, so divide explicitly.)
            straight = tmp / "straight.png"
            subprocess.run(
                ["magick", str(on_black), str(alpha),
                 "-compose", "Divide", "-composite", str(straight)],
                check=True,
            )
            # Attach the solved alpha, then crop to ink.
            subprocess.run(
                ["magick", str(straight), str(alpha), "-alpha", "off",
                 "-compose", "CopyOpacity", "-composite",
                 "-trim", "+repage", "-resize", f"x{height}", f"PNG32:{target}"],
                check=True,
            )
        size = subprocess.run(["magick", "identify", "-format", "%wx%h", target],
                              capture_output=True, text=True).stdout
        print(f"{source:28} -> {target:24} {size}")


if __name__ == "__main__":
    main()
