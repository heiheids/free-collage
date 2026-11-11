# Free Collage · Technical Notes

> Project: Free Collage / 自由拼图  
> Version: 1.0.3  
> Platform: Windows 10/11 x64  
> License: GPL-3.0-only  
> Source: https://github.com/heiheids/free-collage

This document describes the architecture and implementation for developers reviewing or extending the source. User-facing documentation is in [README](../README.md).

## 1. Goals and constraints

Free Collage is a native Windows collage and annotation application designed around these constraints:

- Start immediately on an unlimited transparent canvas, with no startup size or file-type prompt.
- Import images at their original pixel dimensions; view zoom does not change layer geometry.
- Support auto-crop and fixed A4/B5/custom pages.
- Keep UI text, canvas text and exported text consistent.
- Ship as a single statically linked executable with no runtime DLLs.
- Avoid Electron/Qt/WebView and third-party UI runtimes.
- Keep the source modular and the direct dependency set small.

The application is Windows-only. That allows direct use of Win32 DPI, input, clipboard, DWM and GDI screen capture without a cross-platform abstraction layer.

## 2. Stack

| Component | Role |
| --- | --- |
| Rust 2021, MSVC target | Application language |
| `windows-sys` | Win32, GDI, DWM, clipboard and dialog bindings |
| `tiny-skia` | Anti-aliased 2D rasterization for UI, canvas and export |
| `ab_glyph` | TrueType/OpenType outline parsing and grayscale glyph rasterization |
| `image` | PNG, JPEG, BMP and GIF (first frame) decoding |
| `webp` / bundled `libwebp` | WebP decoding and encoding |

Text editing uses a native `EDIT` child window overlay for IME support. All other UI is rendered by the application.

## 3. Layout

```text
src/
  main.rs                  Process entry point
  app/
    mod.rs                 App state, paint entry, window commands, modals
    actions.rs             Open, paste, export, canvas and layer actions
    input.rs               Mouse, keyboard, drag, hit testing
    text.rs                Text layer creation and native EDIT editor
  model/
    document.rs            Document, page, content bounds
    layer.rs               Layer, LayerKind, image/text/shape/mosaic data
    canvas.rs              CanvasMode, A4/B5/DPI, pixel sizes
    history.rs             Undo stack (60 steps)
    geometry.rs            Point, Rect, transforms
    style.rs               Rgba, color target, palette
  render/
    mod.rs                 Renderer/Painter, layout, hits, panel cache
    doc.rs                 Document renderer, page view, image scaling, checker
    export.rs              Flattened export, quality, transparency
    overlays.rs            Popups and save/about/close-confirm modals
    icons.rs               Icon paths, rounded rects, shadow/stroke helpers
    theme.rs               Theme colors
  platform/
    window.rs              Main window, WndProc, DPI, hit testing, message loop
    capture.rs             Capture bar, full-screen overlay, window picker, GDI capture
    gdi.rs                 Font engine, DIB presentation, GDI helpers
    clipboard.rs           CF_DIB/PNG/HDROP reading
    dialogs.rs             Native open/save dialogs
    system.rs              Paths, system locale, window corners
  codec.rs                 Image decode/encode
  config.rs                settings.cfg
  i18n.rs                  Language loading
  state.rs                 Runtime state, popups, view, text edit state
  tools.rs                 Tool enum and drag state
  ui.rs                    Layout, Rect, HitId
build.rs                   Icon resource compilation, language/icon copy
assets/                    Icon, resource script, screenshot
languages/                 External zh-CN/en-US language files
tools/                     Development helper scripts
```

## 4. Window and lifecycle

`main.rs` calls `platform::window::run()`. The main window is a borderless `WS_POPUP` window with custom DPI awareness:

- The default size is derived from the monitor work area; the minimum is about 940×680.
- `WM_NCCALCSIZE` removes the system non-client area; the client area is the whole drawn window.
- `WM_NCHITTEST` provides eight resize edges and the title-bar drag region.
- **Interactive header areas** (canvas chip, capture, language, about, minimize/maximize/close) must return `HTCLIENT`; otherwise a real mouse click is treated as a title-bar drag. The capture button was initially missing from this list, which made real clicks appear to do nothing. `layout.screenshot` is now included.
- `WM_DPICHANGED` updates DPI and layout; `GetDpiForWindow` is used for queries.
- `WM_CLOSE` is intercepted: if the document is dirty, a `CloseConfirm` modal offers Save / Discard / Cancel. A successful export posts an asynchronous close.
- The UI is single-threaded. The capture overlay runs on the same message loop; there is no cross-thread UI access.

## 5. State and model

`App` owns:

- `State`: document, view, tool, selection, hover, popup, modal, toast, text editing, settings.
- `Renderer` / `TextEngine`: renderer and font caches.
- `buffer: Vec<u32>` for BGRA presentation, `layout`, `hits`, and a reusable frame `Pixmap`.
- `hwnd`, DPI, native editor HWND, `force_close`, and platform state.

`Document` contains a `CanvasMode` and a `Vec<Layer>`:

- `Auto`: no fixed page; export crops to visible content.
- `Fixed { width_mm, height_mm, dpi }`: A4/B5/custom; portrait and landscape are represented by swapped dimensions.
- `LayerKind`: Image, Text, Stroke, Shape, Mosaic.
- Layers have an id, rectangle, visibility and z-order.
- `History` stores snapshots with a 60-step limit.

The view is `zoom` + `pan`; `page_screen_rect()` and its inverse map world coordinates to screen coordinates.

## 6. Rendering pipeline

### 6.1 Frame lifecycle

`App::paint(hdc)`:

1. `sync_client_size()` checks client size and DPI.
2. `Renderer::render()` obtains/reuses a `Pixmap`, builds `Layout`, and runs `paint_base()`, `paint_popup()`, `paint_toast()` and `paint_modal()`.
3. `App.layout` and `App.hits` are updated.
4. `gdi::present()` converts RGBA to BGRA in a `Vec<u32>` and presents via `StretchDIBits`.
5. The Pixmap is returned to the renderer for reuse, avoiding a 7 MB allocation every frame.

`Frame` carries the Pixmap, layout and hit list. Painting and hit testing share the same layout.

### 6.2 Layout and hit testing

`ui.rs::Layout` computes geometry from physical client size and DPI:

- Large layout when width ≥ 1400 and height ≥ 980; `ui_boost = 1.5`.
- Compact layout otherwise; the canvas chip uses a short label in compact mode.
- Every interactive region is pushed into `hits: Vec<Hit>`; `hit_at()` returns the topmost hit.
- Hover/pressed state affects paint and cursor only, not layout.

### 6.3 Base UI

`paint_base()` draws header, toolbar, canvas, bottom style bar and selection overlay.

- The header contains the title, canvas chip, capture button, language, about and window controls. All header text except the app name uses one size.
- The toolbar has two columns: actions and drawing tools.
- The bottom bar contains color, font, outline, brush, mosaic, zoom, fit and actual-size controls.

### 6.4 Canvas and page rendering

- Auto pages use a transparent checkerboard; fixed pages are white.
- `render_page_view()` renders only the **visible page intersection**, not the whole 300 DPI page. It computes the intersection of `page_screen_rect` and the canvas viewport and creates a viewport-sized Pixmap.
- The page view is copied with `blit_opaque` because its background is opaque; no SourceOver blend is needed.
- Image layers are sampled by destination pixel size instead of using the generic tiny-skia transform, which would traverse the whole source image.
- The checkerboard is filled by direct pixel writes rather than thousands of tiny-skia paths.
- Selection borders and eight handles are drawn directly into the frame, avoiding a canvas-sized overlay allocation on every frame.

### 6.5 Text

`platform::gdi::TextEngine`:

- Loads system TTF/TTC fonts: Microsoft YaHei, SimSun, SimHei, KaiTi, FangSong, Segoe UI, Arial, Times New Roman, Consolas.
- Parses outlines with `ab_glyph` and rasterizes grayscale coverage masks.
- Masks are cached by `(text, pixel_size, weight, family)` with a cap of about 600 entries; measurements are cached separately.
- `blit_mask()` composites the grayscale mask into the target Pixmap.
- Outlines are produced with 16 directional mask offsets; UI and export share the same path.
- Text editing uses a native `EDIT` child window for IME; committing creates a normal text layer.

### 6.6 Performance

Optimizations measured in Release:

- Cached toolbar/bottom rounded panel backgrounds; each frame copies them with `memcpy`.
- Removed large runtime shadows; borders remain. This removed tens of milliseconds per frame from path fills.
- Interactive images use destination-driven nearest-neighbour sampling; small targets and export use destination-driven bilinear sampling.
- Direct checkerboard pixel writes.
- Direct selection overlay rendering.
- Reused frame Pixmap.
- Batched RGBA→BGRA conversion in `u32` units.
- Mouse movement only invalidates when dragging, hover changes, brush ring or slider drag; ordinary movement no longer repaints the entire window.

Typical development machine, 1650×1075 Release: about 13 ms/frame for an empty canvas, about 12 ms/frame for Auto + a 1000×700 image, and about 9.5 ms/frame for A4 + image. Actual timings depend on CPU, layer count and zoom.

## 7. Input and tools

`app/input.rs` handles:

- Left button: hit, select, drag, resize, draw, text editing.
- Right button: layer menu, or pan over empty canvas.
- Middle button/space: pan.
- Wheel: zoom around cursor.
- Double-click text: edit again.
- Keyboard: tool shortcuts, undo, delete, fit, 100%, Tab layer cycling, Esc cancel.

`tools.rs` defines the tool enum and drag state (`Pan`, `Move`, `Resize`, `Stroke`, `ShapeCreate`, `TextSelect`, etc.). Drag updates mutate state and repaint; history is committed on release.

### Magnetic snapping

**Snap** is an independent toggle below Save in the left toolbar, not a drawing mode. When enabled it affects only **image layer moves**:

- The threshold is 8 screen pixels, converted with `8 / zoom`, so it feels consistent at every zoom level.
- Horizontally, the moving image's left/centre/right edges are compared with other images' and fixed-page left/centre/right edges. Vertically the same applies to top/centre/bottom.
- X and Y choose their nearest edge independently, so a vertical and a horizontal edge can snap at once. Image centres and the fixed page centre are also targets.
- Targets are other visible image layers plus fixed-page edges/centre; an auto-crop canvas has no page target.
- Only image layers move with snapping; text and shapes do not.
- The pure calculation lives in `src/model/snap.rs` with tests for simultaneous X/Y snapping, out-of-threshold targets and page snapping.

### Image cropping

- **Crop** sits below Snap in the left toolbar and is enabled only when the selected layer is an image; otherwise it is greyed out and explains that an image must be selected.
- Entering crop mode forces the normal arrow cursor so the image-move cursor does not obscure the selection; the next left-button drag is handled by crop logic. Both endpoints are converted to document coordinates and clamped to the image rectangle.
- Outside the selection is dimmed and the boundary is drawn as a blue dashed rectangle. On release, a selection larger than 2 screen pixels opens `Modal::CropConfirm`.
- Confirming clones the layer, maps the selection to source-pixel bounds, copies those pixels into a new `Pixmap`, and recalculates the layer's world rectangle from the cropped pixel ratio, preserving its position and display scale.
- History is committed only after a successful crop. Cancel, `Esc` or closing the confirmation clears crop state without modifying the image.
- Only image layers can be cropped; a selection covering the whole image is a no-op.

## 8. Canvas and export

- Image layers retain their source Pixmap; the layer rectangle is independent of view zoom.
- Auto-crop uses the union of visible layer content bounds; transparent pixels do not contribute.
- Fixed pages render at page pixel size; content outside the page is clipped on export.
- Export calls `render_document(..., smooth=true)`, using high-quality destination-driven bilinear sampling. Screen interaction uses nearest-neighbour for large targets to reduce latency.
- `export.rs` supports PNG/JPEG/WebP/BMP; JPEG is filled white, PNG/WebP preserve alpha.

## 9. Capture subsystem

`platform/capture.rs` uses two top-level windows, separate from the main window.

### Capture bar

- The main window is minimized with `ShowWindow(SW_MINIMIZE)`; a topmost borderless bar is created.
- The bar is a white rectangular window with a 2 px grey border and light buttons. It has only a right-side close button.
- “Capture window” and “Rectangle” enter the corresponding mode; the right `×` cancels and restores the main window.
- The bar is GDI-painted at 480×64. It fills the actual client area and uses four solid edge strips, avoiding white bottom/right edges.

### Full-screen overlay

- Before entering a mode, the app captures the whole virtual desktop with GDI `BitBlt` into an in-memory DIB, then hides the bar.
- The overlay covers the virtual screen and displays the desktop snapshot, not the overlay itself.
- Rectangle mode drags a selection and crops the snapshot on release.
- Window mode:
  - On mouse move, it walks top-level windows in Z-order, skips the overlay/bar/minimized main window, and finds the real window under the cursor.
  - It draws a 4 px blue highlight border on the overlay.
  - On click, the highlighted window becomes the target. The overlay is hidden, the target is brought forward, and its window rectangle is captured with a screen `BitBlt`.
- On completion, the image is placed on the clipboard as CF_DIB, the main window is restored, and `WM_PASTE` lets the normal paste path add the layer.
- Closing/cancelling does not modify the clipboard or document.

No additional crate is used; the subsystem builds on `windows-sys` and GDI.

## 10. Configuration and localization

- `settings.cfg` lives next to the executable and stores language, canvas, colors, font, size, brush, mosaic, export format/quality/transparency and last directory.
- `languages/*.lang` are UTF-8 `key=value` files with `#` comments; zh-CN and en-US must have the same key set.
- The app selects a language from the system locale at startup; the top-right menu can follow the system or select a language explicitly.
- Language files are external; adding a language does not require a rebuild.

## 11. Build and resources

- `build.rs` copies `languages/` and `assets/app.ico` into the output directory.
- It locates `rc.exe` via `RC`, `PATH` or Windows Kits, compiles `assets/app.rc`, and embeds icon/version resources.
- If `rc.exe` is missing it emits a warning; the EXE still builds, without icon/version metadata.
- `.cargo/config.toml` enables `+crt-static` for `x86_64-pc-windows-msvc`.
- Release profile: `opt-level = "s"`, LTO, one codegen unit, strip and `panic = "abort"`.
- All dependencies are statically linked; no extra DLLs.

## 12. Tests

```powershell
cargo test --locked
```

Tests cover A4/B5/landscape pixel sizes, geometry union/intersection, content bounds and transparent cropping, scaled image drawing, PNG/WebP transparency, JPEG white fill, BMP round-trip, text export, system font loading, CJK grayscale masks, font family/outline rendering, language parsing and intro length.

## 13. Limitations

- Windows only; there is no cross-platform backend.
- Single UI thread; very large canvases or many high-resolution layers can still increase frame time.
- Animated WebP is not supported; GIF uses the first frame.
- Export is flattened; no layer project and no Redo.
- Fonts come from the OS; missing fonts fall back to Microsoft YaHei.
- Windows with DRM/protected content may not be capturable with GDI.
- Window capture uses a screen BitBlt; occluded content follows what is actually on screen.

## 14. Security and privacy

- No network requests, telemetry or automatic uploads.
- Settings, exports and backups stay on the local filesystem.
- Capture data is written only to the local clipboard and read by the main process; it does not leave the machine.
- Source is GPL-3.0-only; third-party licenses are in `THIRD_PARTY_NOTICES.txt`.

## 15. License

GPL-3.0-only. Full text: [LICENSE](../LICENSE). Copyright: [COPYRIGHT](../COPYRIGHT).
