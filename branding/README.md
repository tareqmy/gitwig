# Gitwig brand — final (16e)

The mark is the real Rajdhani 700 lowercase g in verdigris #4db08a with:
- the bowl's counter filled and re-cut as a leaf (negative space, tip to the
  top-left) — the leaf exists only as the ground color showing through;
- a copper HEAD ring capping the tail's end, with two smaller copper commits
  fading off below it (opacity 0.75 and 0.45).

The leaf cut and the ring interior are knocked out with an SVG `<mask>` rather
than painted in the ground color, so they are true negative space and the mark
stays correct on any background — not only on #1b1614. The wordmark is converted
to outline paths, so nothing here needs a font installed or a network fetch.
There is still no single-color version: the mark needs both verdigris and copper.

## Files
- `lockup.png` — mark + wordmark on the brand ground, 1068×630. The reference
  the vectors are matched against.
- `logo-mark.png` — the mark alone on transparency, 319×480.
- `logo-mark.svg` / `lockup.svg` / `app-icon.svg` — vector versions. Self-contained:
  masked negative space and outlined wordmark, no font or network needed.
- `build-logos.py` — regenerates every SVG above from the font plus the geometry
  in this file. Edit the numbers here, re-run it, don't hand-edit the SVGs.
- `Rajdhani-Bold.ttf` — the wordmark typeface (Rajdhani 700, Indian Type Foundry).
  `Rajdhani-OFL.txt` is its SIL Open Font License 1.1 and must stay beside it.
- `theme.json` — palette and TUI color roles, machine-readable.

## Mark geometry (relative to a 120px glyph, em-box top-left at 0,0)
- Counter patch: rounded rect at (15, 42), 32×46, r12, glyph color.
- Leaf cut: path M8 8 C21 7 29 15 27 40 C13 41 6 29 8 8 Z, translated (13, 40),
  ground color.
- HEAD ring: r7, stroke 4 copper #bd6b3d, ground fill, center (13, 109).
- Trailing commits: copper dots r3.5 @ (1, 120) op 0.75 and r2 @ (-5.5, 128) op 0.45.

## Wordmark
Rajdhani 700, lowercase `gitwig` — `git` in #ece2d8, `wig` in #4db08a.
Caps GITWIG (letter-spacing ~0.12em) for small labels only.

## Palette
| Role | Hex |
| --- | --- |
| Ground (brand) | #1b1614 |
| Ground (terminal) | #14100e |
| Foreground | #ece2d8 |
| Verdigris (primary accent) | #4db08a |
| Copper (secondary) | #bd6b3d |
| Dim / muted | #7d746c |
| Selection | #1d2b23 |

## UI type
JetBrains Mono for all TUI text. Keybinding letters in verdigris, muted labels
in dim, selected rows on #1d2b23.

## Where these land in the app

Every surface carrying the mark or the palette, so a future brand change has one
checklist instead of a search:

| Surface | File | Source |
| --- | --- | --- |
| README hero (dark / light) | `resources/logo-dark.svg`, `resources/logo-light.svg` | `build-logos.py` |
| App icon (1024×1024 PNG) | `resources/icon.png` | `app-icon.svg` |
| Chocolatey package icon | `dist/chocolatey/gitwig.nuspec` → `iconUrl` | `resources/icon.png` via jsDelivr |
| About popup mark | `src/popups/about.rs` | the mark, redrawn in box-drawing |
| Empty-state glyph | `src/ui/draw.rs` (`draw_empty_state`) | HEAD ring only |
| Brand colors in code | `src/ui/style.rs` → `BRAND_VERDIGRIS()`, `BRAND_COPPER()` | `theme.json` |
| Brand TUI theme | `config/themes/gitwig.theme` | `theme.json` |
| Theme shipped on first launch | `src/config.rs` — `include_str!`s the file above | — |
| Theme documentation | `docs/configuration.md` → Themes | — |

`accent`/`warning` map to `accent_verdigris`/`accent_copper`. `danger` (`#b2402e`)
and `success` (`#3c8a6b`) are **derived**, not in `theme.json`: the TUI needs a
conflict red and a committed-green the brand palette does not define. Both follow
its logic — a redder sibling of copper, and a deeper verdigris.

## Regenerating

Vectors, from the geometry above:

```sh
python3 -m venv .venv && .venv/bin/pip install fonttools
.venv/bin/python branding/build-logos.py
```

Raster (`logo-mark.png` and `resources/icon.png`):

```sh
python3 branding/rasterize.py
```

Do **not** rasterize these by hand with `qlmanage ... | magick -transparent white`.
That pipeline silently corrupts them in two ways, both of which happened once:

- `qlmanage` emits a *square* thumbnail and **clips** a non-square SVG instead of
  letterboxing it, which amputated the mark's two trailing commit dots.
- `qlmanage` composites onto opaque white, so knocking white out cannot recover
  partial alpha. The dots at opacity 0.75/0.45 came back as opaque pastels that
  glow on a dark ground.

`rasterize.py` avoids both: it wraps each source in a square canvas, renders it
twice (over white and over black), and solves the true alpha per pixel from
`a = 1 - (W - B)`, `C = B / a`.

`lockup.png` is deliberately not regenerated — it is the hand-authored reference
the vectors are matched against, so overwriting it would destroy the baseline.

ImageMagick alone cannot rasterize these at all: its built-in SVG renderer drops
masked geometry and emits a bare plate.
