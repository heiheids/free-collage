# Changelog

## [1.0.0] - 2026-11-10

首个公开版本 / First public release.

### Added

- Native Win32 borderless UI with DPI-aware 1.5× and compact layouts.
- Unlimited transparent canvas, auto-crop export, A4/B5 portrait/landscape and custom pages.
- PNG/JPEG/WebP/GIF(first frame)/BMP import; PNG/JPEG/WebP/BMP export.
- Layer move/scale, layer up/down, 60-step undo.
- Nine system fonts with text outline colour/width.
- Brush, mosaic, eraser, arrow, border, rectangle and ellipse tools.
- Window and rectangular screen capture with a floating capture bar and highlighted window target.
- External Chinese/English language files.
- Unsaved-change close confirmation.
- GPL-3.0-only licensing and third-party notices.

### Performance

- Cached rounded panel backgrounds.
- Destination-driven image scaling.
- Direct checkerboard and selection rendering.
- Reused frame Pixmap and batched BGRA presentation.
- Invalidation only when the visual state changes.
