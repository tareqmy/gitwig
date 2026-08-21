#!/usr/bin/env python3
"""Regenerate the Gitwig logo files from Rajdhani Bold + the documented overlays.

The mark is the real Rajdhani 700 lowercase `g` with its counter filled and
re-cut as a leaf, plus a copper HEAD ring and two trailing commit dots. See
"Mark geometry" in README.md -- this script is the executable form of that spec.

Two deliberate deviations from a naive reading of the spec, both of which render
identically on the brand plate:

* The leaf and the ring interior are knocked out with an SVG <mask> rather than
  painted in the ground colour, so the mark is true negative space and stays
  correct on any background instead of only on #1b1614.
* The wordmark ships as outline paths, not a live <text> element. GitHub strips
  web fonts from README SVGs, so <text> silently falls back to Helvetica.

Run from the repository root:

    python3 -m venv .venv && .venv/bin/pip install fonttools
    .venv/bin/python branding/build-logos.py

Writes branding/{logo-mark,app-icon,lockup}.svg and resources/logo-{dark,light}.svg.
The app icon PNG is a separate step -- see "Regenerate the app icon" in README.md.
"""

import pathlib
import re
import sys

from fontTools.misc.transform import Transform
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

FONT = "branding/Rajdhani-Bold.ttf"

VERDIGRIS = "#4db08a"
COPPER = "#bd6b3d"
BONE = "#ece2d8"
GROUND = "#1b1614"

# Mark coordinate system: a 120px glyph with the em-box top-left at (0,0),
# which puts the baseline at 0.8em. Every overlay constant below is quoted
# straight from README.md's "Mark geometry" section.
GLYPH_PX = 120.0
BASELINE = 96.0
LEAF = "M8 8 C21 7 29 15 27 40 C13 41 6 29 8 8 Z"
LEAF_AT = (13, 40)
COUNTER = (15, 42, 32, 46, 12)      # x, y, w, h, r
RING = (13, 109, 7, 4)              # cx, cy, r, stroke-width
DOTS = ((1, 120, 3.5, 0.75), (-5.5, 128, 2, 0.45))

# Lockup layout, in the same units, matched to the canonical branding/lockup.png.
LOCKUP = dict(w=362, h=213, r=16, mark=(78.4, 45.7), text=(157.5, 120.8), size=48.6)

WORD, SPLIT = "gitwig", 3


def _round(data, places=2):
    def shorten(match):
        value = round(float(match.group()), places)
        return str(int(value)) if value == int(value) else str(value)

    return re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", shorten, data)


def _font():
    if not pathlib.Path(FONT).exists():
        sys.exit(f"{FONT} not found -- run this from the repository root.")
    return TTFont(FONT)


def glyph_path(font, char, size, origin_x=0.0, baseline=0.0):
    """One glyph as an SVG path, scaled to `size` with the baseline at `baseline`."""
    scale = size / font["head"].unitsPerEm
    glyph_set = font.getGlyphSet()
    pen = SVGPathPen(glyph_set)
    # Font Y grows upward, SVG Y downward -- hence the negative Y scale.
    transform = Transform(scale, 0, 0, -scale, origin_x, baseline)
    glyph_set[font.getBestCmap()[ord(char)]].draw(TransformPen(pen, transform))
    return _round(pen.getCommands())


def kern_pairs(font):
    """Flat pair-kerning from GPOS lookup type 2 (Rajdhani defines none for 'gitwig')."""
    pairs = {}
    if "GPOS" not in font:
        return pairs
    for lookup in font["GPOS"].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            if getattr(sub, "LookupType", lookup.LookupType) != 2:
                continue
            if sub.Format == 1:
                for first, pairset in zip(sub.Coverage.glyphs, sub.PairSet):
                    for rec in pairset.PairValueRecord:
                        if value := getattr(rec.Value1, "XAdvance", 0) or 0:
                            pairs[(first, rec.SecondGlyph)] = value
            elif sub.Format == 2:
                class1, class2 = sub.ClassDef1.classDefs, sub.ClassDef2.classDefs
                for g1 in sub.Coverage.glyphs:
                    for g2, k2 in class2.items():
                        try:
                            rec = sub.Class1Record[class1.get(g1, 0)].Class2Record[k2]
                        except IndexError:
                            continue
                        if value := getattr(rec.Value1, "XAdvance", 0) or 0:
                            pairs[(g1, g2)] = value
    return pairs


def wordmark(font, size):
    """Return (git_path, wig_path, advance) with the baseline at the origin."""
    upm = font["head"].unitsPerEm
    cmap, glyph_set, hmtx = font.getBestCmap(), font.getGlyphSet(), font["hmtx"]
    kerns = kern_pairs(font)
    names = [cmap[ord(ch)] for ch in WORD]
    scale = size / upm

    drawn, pen_x = [], 0.0
    for index, name in enumerate(names):
        pen = SVGPathPen(glyph_set)
        glyph_set[name].draw(
            TransformPen(pen, Transform(scale, 0, 0, -scale, pen_x * scale, 0))
        )
        drawn.append(pen.getCommands())
        pen_x += hmtx[name][0]
        if index + 1 < len(names):
            pen_x += kerns.get((name, names[index + 1]), 0)

    return _round("".join(drawn[:SPLIT])), _round("".join(drawn[SPLIT:])), pen_x * scale


def mark(font, uid, indent="  "):
    """The `g` mark as an SVG fragment, in the 120px-glyph coordinate system."""
    cx, cy, r, stroke = RING
    x, y, w, h, rx = COUNTER
    body = f"""<mask id="{uid}" maskUnits="userSpaceOnUse" x="-16" y="24" width="88" height="116">
  <rect x="-16" y="24" width="88" height="116" fill="#fff"/>
  <!-- Leaf and ring interior are knocked out, so they read as true negative space. -->
  <path d="{LEAF}" transform="translate({LEAF_AT[0]} {LEAF_AT[1]})" fill="#000"/>
  <circle cx="{cx}" cy="{cy}" r="{r}" fill="#000"/>
</mask>
<g mask="url(#{uid})">
  <path d="{glyph_path(font, 'g', GLYPH_PX, 0, BASELINE)}" fill="{VERDIGRIS}"/>
  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{VERDIGRIS}"/>
</g>
<circle cx="{cx}" cy="{cy}" r="{r}" fill="none" stroke="{COPPER}" stroke-width="{stroke}"/>"""
    for dx, dy, dr, opacity in DOTS:
        body += f'\n<circle cx="{dx}" cy="{dy}" r="{dr}" fill="{COPPER}" opacity="{opacity}"/>'
    return "\n".join(indent + line for line in body.splitlines())


def main():
    font = _font()
    git_d, wig_d, advance = wordmark(font, LOCKUP["size"])
    lw, lh, lr = LOCKUP["w"], LOCKUP["h"], LOCKUP["r"]
    mx, my = LOCKUP["mark"]
    tx, ty = LOCKUP["text"]

    def wordmark_group(git_fill, x, y):
        return (
            f'  <!-- Wordmark: Rajdhani Bold (700) as outlines; no font needed to render. -->\n'
            f'  <g transform="translate({x} {y})">\n'
            f'    <path fill="{git_fill}" d="{git_d}"/>\n'
            f'    <path fill="{VERDIGRIS}" d="{wig_d}"/>\n'
            f'  </g>\n'
        )

    # Mark alone, on transparent. The mask makes it safe on any background.
    pathlib.Path("branding/logo-mark.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="512" height="674" '
        f'viewBox="-16 24 88 116" role="img" aria-label="Gitwig">\n'
        f'{mark(font, "gitwig-mark")}\n</svg>\n'
    )

    # App icon: mark centred on the rounded brand plate.
    pathlib.Path("branding/app-icon.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" '
        f'viewBox="0 0 64 64" role="img" aria-label="Gitwig">\n'
        f'  <rect width="64" height="64" rx="14" fill="{GROUND}"/>\n'
        # Mark content spans x -7.5..55.68, y 34.8..130 -> centre (24.09, 82.4);
        # 0.462 puts its 95.2-unit height at 44 of the icon's 64, leaving even margins.
        f'  <g transform="translate(32 32) scale(0.462) translate(-24.09 -82.4)">\n'
        f'{mark(font, "gitwig-icon", indent="    ")}\n'
        f'  </g>\n</svg>\n'
    )

    # Brand lockup: mark + wordmark on the plate, matching branding/lockup.png.
    pathlib.Path("branding/lockup.svg").write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{lw * 2}" height="{lh * 2}" '
        f'viewBox="0 0 {lw} {lh}" role="img" aria-label="Gitwig">\n'
        f'  <rect width="{lw}" height="{lh}" rx="{lr}" fill="{GROUND}"/>\n'
        f'  <g transform="translate({mx} {my})">\n'
        f'{mark(font, "gitwig-lockup", indent="    ")}\n'
        f'  </g>\n'
        f'{wordmark_group(BONE, tx, ty)}</svg>\n'
    )

    # README heroes: no plate, tight canvas so <p align="center"> centres the art.
    # The mark's negative space means one geometry serves both grounds; only the
    # "git" half of the wordmark changes colour. Mark/text spacing reuses the
    # lockup's relative offset so the two assets stay consistent.
    dx, dy = tx - mx, ty - my                 # text offset from the mark origin
    pad = 8
    ink_x0, ink_x1 = -7.5, dx + advance       # mark's left dot .. wordmark right edge
    ink_y0, ink_y1 = 34.8, 130.0              # glyph top .. lowest trailing dot
    hero_w = round(ink_x1 - ink_x0 + pad * 2)
    hero_h = round(ink_y1 - ink_y0 + pad * 2)
    hero_mark = (pad - ink_x0, pad - ink_y0)
    for name, fill in (("dark", BONE), ("light", GROUND)):
        pathlib.Path(f"resources/logo-{name}.svg").write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{hero_w * 2}" '
            f'height="{hero_h * 2}" viewBox="0 0 {hero_w} {hero_h}" role="img" '
            f'aria-label="Gitwig">\n'
            f'  <g transform="translate({hero_mark[0]:.1f} {hero_mark[1]:.1f})">\n'
            f'{mark(font, f"gitwig-hero-{name}", indent="    ")}\n'
            f'  </g>\n'
            f'{wordmark_group(fill, round(hero_mark[0] + dx, 1), round(hero_mark[1] + dy, 1))}</svg>\n'
        )

    print(f"wordmark advance {advance:.2f} -> hero canvas {hero_w}x{hero_h}")
    print("wrote branding/{logo-mark,app-icon,lockup}.svg, resources/logo-{dark,light}.svg")


if __name__ == "__main__":
    main()
