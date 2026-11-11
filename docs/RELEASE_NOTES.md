# 自由拼图 v1.0.3 / Free Collage v1.0.3

本次更新修复裁切模式的光标。  
This release fixes the cursor in crop mode.

## 中文

### 修复

- 进入裁切模式后，鼠标指针改为普通箭头；不再显示移动图片的四向箭头或绘制工具十字光标。
- 拖出裁切虚线与确认弹窗期间均保持普通箭头，便于看清选区位置。

### 升级

下载 `FreeCollage_v1.0.3_win64.zip`，解压后运行 `自由拼图.exe`。支持 Windows 10/11 x64；原有 `settings.cfg` 可继续使用。

### 许可证

GPL-3.0-only。源码与完整文档：https://github.com/heiheids/free-collage

---

## English

### Fixed

- Crop mode now uses the normal arrow cursor instead of the image-move four-way cursor or the drawing crosshair.
- The normal arrow remains while dragging the dashed crop area and while the confirmation dialog is open, making the selection position easier to see.

### Upgrade

Download `FreeCollage_v1.0.3_win64.zip`, extract and run `自由拼图.exe`. Windows 10/11 x64; the existing `settings.cfg` remains compatible.

### License

GPL-3.0-only. Source and full documentation: https://github.com/heiheids/free-collage

---

## Build

```powershell
cargo test --locked
cargo build --release --locked
```

See [docs/BUILD.md](BUILD.md) for toolchain, packaging and release details.
