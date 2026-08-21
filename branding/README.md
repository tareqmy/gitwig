# Gitwig brand — Verdigris

Locked direction: mark 2b + Rajdhani wordmark (5f) on the Verdigris palette —
oxidized-copper green on warm charcoal, copper rivet. Kept deliberately distinct
from Ferronote (cool #0f1419 ground, iron-orange accent).

## Files
- `logo-mark.svg` — the g mark, verdigris strokes + copper rivet on transparent. Scales to 14px.
- `logo-mark-mono.svg` — single-color version (uses `currentColor`).
- `app-icon.svg` — mark on the #1b1614 rounded plate (icons, favicon, stickers).
- `lockup.svg` — mark + wordmark on brand ground. The wordmark is **converted to
  outline paths**, so it needs no font installed and renders identically everywhere.
- `Rajdhani-Bold.ttf` — the wordmark typeface (Rajdhani 700, Indian Type Foundry),
  committed so the lockup can be regenerated. `Rajdhani-OFL.txt` is its SIL Open
  Font License 1.1, which must stay alongside the font.
- `theme.json` — palette and TUI color roles, machine-readable.

## Mark
Square-cap strokes in verdigris #4db08a, stroke width 5.5/64, copper rivet
#bd6b3d rotated 45° on the descender tip. Gap between bowl and stem ≈ one
stroke width. Geometry (viewBox 0 0 64 64):
- bowl: M38 13 L18 13 L18 37 L38 37
- stem + tail: M38 9 L38 45 L27 53
- rivet: 8×8 square centered (25, 54), rotated 45°

## Wordmark
Rajdhani 700, lowercase `gitwig` — `git` in #ece2d8, `wig` in #4db08a.
Caps GITWIG (letter-spacing ~0.12em) for small labels only.

Always ship the wordmark as **outlines, never as `<text>`**: GitHub strips web fonts
from README SVGs, so a `<text>` element silently falls back to Helvetica and stops
being the brand. Set at font-size 52 with the baseline at y=60; Rajdhani has no
kerning pairs for these six letters, so plain advance widths are correct.

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

Every surface that carries the mark or the palette, so a future brand change has
one checklist instead of a search:

| Surface | File | Source |
| --- | --- | --- |
| README hero (dark / light) | `resources/logo-dark.svg`, `resources/logo-light.svg` | `lockup.svg` |
| App icon (1024×1024 PNG) | `resources/icon.png` | `app-icon.svg` |
| Chocolatey package icon | `dist/chocolatey/gitwig.nuspec` → `iconUrl` | `resources/icon.png` via jsDelivr |
| About popup mark | `src/popups/about.rs` | `logo-mark.svg`, redrawn in box-drawing |
| Empty-state rivet | `src/ui/draw.rs` (`draw_empty_state`) | copper rivet only |
| Brand mark colors in code | `src/ui/style.rs` → `BRAND_VERDIGRIS()`, `BRAND_COPPER()` | `theme.json` |
| Brand TUI theme | `config/themes/gitwig.theme` | `theme.json` |
| Theme shipped on first launch | `src/config.rs` — `include_str!`s the file above | — |
| Theme documentation | `docs/configuration.md` → Themes | — |

`accent`/`warning` map to `accent_verdigris`/`accent_copper`. `danger` (`#b2402e`)
and `success` (`#3c8a6b`) are **derived**, not in `theme.json`: the TUI needs a
conflict red and a committed-green that the brand palette does not define. They
follow the palette's logic — `danger` is a redder sibling of copper, `success` a
deeper verdigris.

Regenerate the app icon after editing `app-icon.svg`:

```sh
qlmanage -t -s 1024 -o /tmp branding/app-icon.svg
magick /tmp/app-icon.svg.png -alpha on -fuzz 12% -transparent white PNG32:resources/icon.png
```

ImageMagick alone will **not** work — its built-in SVG renderer silently drops the
mark's paths and emits a bare plate. The `qlmanage` step routes through WebKit;
the `-transparent white` step removes the background QuickLook bakes in.
