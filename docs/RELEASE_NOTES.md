# 自由拼图 v1.0.1 / Free Collage v1.0.1

本次更新新增磁吸对齐。  
This release adds magnetic edge snapping.

## 中文

### 新增：磁吸

- 左栏“保存”下方新增 **磁吸** 开关，图标为 U 形磁铁。
- 打开后，拖动图片靠近其他图片或固定页面边缘时会自动吸附。
- 水平方向（左/中/右）和垂直方向（上/中/下）分别寻找最近的边，两个方向可以同时对齐。
- 吸附阈值固定为屏幕 8px，缩放画布后手感一致。
- 目标包括其他可见图片的边缘/中心和固定页面的边缘/中心；自动裁切画布没有页面目标。
- 仅作用于图片移动；文字、形状等不会被吸附。

### 升级

下载 `FreeCollage_v1.0.1_win64.zip`，解压后运行 `自由拼图.exe`。支持 Windows 10/11 x64；原有 `settings.cfg` 可继续使用。

### 许可证

GPL-3.0-only。源码与完整文档：https://github.com/heiheids/free-collage

---

## English

### Added: magnetic snapping

- New **Snap** toggle below Save in the left toolbar, with a U-shaped magnet icon.
- While moving an image, nearby image edges or fixed-page edges pull into alignment automatically.
- Horizontal (left/centre/right) and vertical (top/centre/bottom) guides are chosen independently, so both axes can snap at once.
- The threshold is 8 screen pixels, so it feels consistent at any canvas zoom.
- Targets include visible image edges/centres and fixed-page edges/centre; auto-crop canvases have no page target.
- Applies to image moves only; text and shapes are not snapped.

### Upgrade

Download `FreeCollage_v1.0.1_win64.zip`, extract and run `自由拼图.exe`. Windows 10/11 x64; the existing `settings.cfg` remains compatible.

### License

GPL-3.0-only. Source and full documentation: https://github.com/heiheids/free-collage

---

## Build

```powershell
cargo test --locked
cargo build --release --locked
```

See [docs/BUILD.md](BUILD.md) for toolchain, packaging and release details.
