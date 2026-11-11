# Changelog

## [1.0.3] - 2026-11-11

### Fixed

- Crop mode now uses the normal arrow cursor instead of the image-move/crosshair cursor, so the pointer no longer obscures the exact crop position.

## [1.0.2] - 2026-11-11

### Added

- Left-toolbar **Crop / 裁切** button below Snap, enabled only when an image layer is selected.
- Crop mode draws a dimmed area with a blue dashed selection; releasing opens a confirmation dialog.
- Confirming keeps the selected pixels, crops the rest, preserves the image's world position/scale and records undo history. Cancelling or Esc exits crop mode unchanged.
- `Layer::crop_image` pixel-crop implementation and a unit test.

## [1.0.1] - 2026-11-10

### Added

- Left-toolbar **Snap / 磁吸** toggle below Save, with a U-shaped magnet icon.
- Image edge snapping while moving: nearest vertical and horizontal guides are applied independently, so both axes can align at once.
- Snap targets include other visible image edges/centres and fixed-page edges/centre; threshold is 8 screen pixels.
- `src/model/snap.rs` unit tests for simultaneous X/Y, threshold and page snapping.

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
