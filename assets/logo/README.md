# Faerry Brand Assets

This folder holds the canonical Faerry logo, mark, monogram, and social
preview assets. Use them consistently across the README, docs, in-app
landing surfaces, store listings, and the desktop bundle. Do not recolor,
add gradients, shadows, outlines, or other effects.

## Brand Colors

| Token   | Hex       | Use                            |
| ------- | --------- | ------------------------------ |
| Ink     | `#10201f` | Primary text, mark, wordmark   |
| Seafoam | `#24b8a3` | Accent, monogram fill, keyline |
| Surface | `#fbfcf8` | Light backgrounds, on-surface lockup |

## Asset Map

| File                                          | Use it for                                                  |
| --------------------------------------------- | ----------------------------------------------------------- |
| `faerry-lockup-horizontal.svg`                | README header, documentation titles, marketing hero.        |
| `faerry-lockup-horizontal-on-surface.svg`     | Light/surface backgrounds where the dark lockup would clash.|
| `faerry-mark.svg`                             | Compact docs sidebar, app landing pages, mid-size icon use. |
| `faerry-monogram.svg`                         | Very small UI or favicon-style icon-only use.               |
| `faerry-wordmark.svg`                         | Wordmark-only placement where the mark is already present.  |
| `faerry-social-preview.png`                   | Open Graph / Twitter card, GitHub social preview.           |
| `faerry-social-preview.svg`                   | Editable source for the social preview artwork.             |
| `faerry-logo-sheet.png` / `.svg`              | Full asset overview, hand-off reference.                    |

Raster fallbacks for the same artwork also live in this folder at higher
resolutions (`*-2080.png`, `*-1600.png`, `*-512.png`). Prefer the SVG
whenever a target supports it, and fall back to the PNG only where raster
is required.

## Application Icon

The desktop app and installer icons live in `assets/icon/` and the full
Tauri icon set lives in `src-tauri/icons/`. The Tauri bundle config in
`src-tauri/tauri.conf.json` references that set, and the window title plus
`productName` are set to `Faerry`.

## Usage Rules

- Do not stretch, skew, or rotate any of the marks.
- Keep the clear-space around each asset roughly equal to the height of
  the monogram glyph.
- On dark backgrounds, use the `on-surface` lockup or the white monogram
  fallback rather than inverting colors by hand.
- When a target needs a single color, use the Seafoam monogram on a light
  background and the white monogram on dark surfaces.
