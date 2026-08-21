#!/usr/bin/env python3
"""Regenerate the Gitwig logo files from the mark geometry + Rajdhani Bold.

The wordmark is converted to outline paths rather than left as an SVG <text>
element: GitHub strips web fonts from README SVGs, so <text> silently falls back
to Helvetica and stops being the brand.

Run from the repository root:

    python3 -m venv .venv && .venv/bin/pip install fonttools
    .venv/bin/python branding/build-logos.py

Writes: resources/logo-dark.svg, resources/logo-light.svg, branding/lockup.svg
The app icon is a separate step -- see "Regenerate the app icon" in README.md.
"""

import pathlib
import re
import sys

from fontTools.misc.transform import Transform
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

FONT = "branding/Rajdhani-Bold.ttf"
WORD = "gitwig"
SPLIT = 3        # "git" | "wig" -- the colour break
SIZE = 52.0      # font-size in SVG user units
BASELINE = 60.0  # wordmark baseline within the 90-unit-tall canvas
MARK_Y = 13      # mark offset within the canvas

VERDIGRIS = "#4db08a"
COPPER = "#bd6b3d"
BONE = "#ece2d8"
GROUND = "#1b1614"

MARK = (
    '    <path d="M38 13 L18 13 L18 37 L38 37" stroke="{v}" stroke-width="5.5"'
    ' fill="none" stroke-linecap="square"/>\n'
    '    <path d="M38 9 L38 45 L27 53" stroke="{v}" stroke-width="5.5"'
    ' fill="none" stroke-linecap="square"/>\n'
    '    <rect x="21" y="50" width="8" height="8" transform="rotate(45 25 54)" fill="{c}"/>'
)


def kern_pairs(font):
    """Flat pair-kerning from GPOS lookup type 2.

    Rajdhani happens to define no pairs for these six letters, but extracting
    them keeps the script correct if the word or the typeface ever changes.
    """
    pairs = {}
    if "GPOS" not in font:
        return pairs
    for lookup in font["GPOS"].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            if getattr(sub, "LookupType", lookup.LookupType) != 2:
                continue
            if sub.Format == 1:
                for first, ps in zip(sub.Coverage.glyphs, sub.PairSet):
                    for rec in ps.PairValueRecord:
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


def round_nums(path_data, places=2):
    """Trim float noise so the committed SVGs stay small and reviewable."""

    def shorten(match):
        value = round(float(match.group()), places)
        return str(int(value)) if value == int(value) else str(value)

    return re.sub(r"-?\d+\.?\d*(?:e-?\d+)?", shorten, path_data)


def outlines():
    """Return (git_path, wig_path, advance_width) with the baseline at origin."""
    font = TTFont(FONT)
    upm = font["head"].unitsPerEm
    cmap, glyph_set, hmtx = font.getBestCmap(), font.getGlyphSet(), font["hmtx"]
    kerns = kern_pairs(font)

    glyphs = [cmap[ord(ch)] for ch in WORD]
    scale = SIZE / upm
    drawn, pen_x = [], 0.0

    for index, name in enumerate(glyphs):
        # Font Y grows upward, SVG Y downward -- hence the negative Y scale.
        transform = Transform(scale, 0, 0, -scale, pen_x * scale, 0)
        pen = SVGPathPen(glyph_set)
        glyph_set[name].draw(TransformPen(pen, transform))
        drawn.append(pen.getCommands())
        pen_x += hmtx[name][0]
        if index + 1 < len(glyphs):
            pen_x += kerns.get((name, glyphs[index + 1]), 0)

    return (
        round_nums("".join(drawn[:SPLIT])),
        round_nums("".join(drawn[SPLIT:])),
        pen_x * scale,
    )


def svg(width, mark_x, text_x, git_fill, git_d, wig_d, plate=None):
    ground = f'  <rect width="{width}" height="90" fill="{plate}"/>\n' if plate else ""
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width * 2}" height="180" \
viewBox="0 0 {width} 90" role="img" aria-label="Gitwig">
{ground}  <!-- Mark: geometry mirrors branding/logo-mark.svg -->
  <g transform="translate({mark_x} {MARK_Y})">
{MARK.format(v=VERDIGRIS, c=COPPER)}
  </g>
  <!-- Wordmark: Rajdhani Bold (700) as outlines, so no font is required to render. -->
  <g transform="translate({text_x} {BASELINE})">
    <path fill="{git_fill}" d="{git_d}"/>
    <path fill="{VERDIGRIS}" d="{wig_d}"/>
  </g>
</svg>
"""


def main():
    if not pathlib.Path(FONT).exists():
        sys.exit(f"{FONT} not found -- run from the repository root.")

    git_d, wig_d, advance = outlines()
    text_x = 86
    right_edge = text_x + advance

    # README hero: tight, balanced canvas over a transparent ground, so the
    # <p align="center"> block in README.md centres the artwork itself rather
    # than a box with dead space on one side.
    hero_width = round(right_edge + 18)
    for name, fill in (("dark", BONE), ("light", GROUND)):
        pathlib.Path(f"resources/logo-{name}.svg").write_text(
            svg(hero_width, 18, text_x, fill, git_d, wig_d)
        )

    # Brand lockup: keep the designed 280x90 plate, but centre the content on it.
    shift = (280 - (right_edge - 18)) / 2 - 18
    pathlib.Path("branding/lockup.svg").write_text(
        svg(280, 18 + shift, text_x + shift, BONE, git_d, wig_d, plate=GROUND)
    )

    print(f"wordmark advance: {advance:.2f} units -> hero canvas {hero_width}x90")
    print("wrote resources/logo-dark.svg, resources/logo-light.svg, branding/lockup.svg")


if __name__ == "__main__":
    main()
