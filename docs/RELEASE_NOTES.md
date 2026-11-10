# 自由拼图 v1.0.0 / Free Collage v1.0.0

首个公开版本。  
First public release.

## 中文

### 功能

- 原生 Rust + Win32 拼图 / 标注工具，单 EXE，无额外运行时 DLL。
- 启动即进入透明自由画布，支持自动裁切。
- A4 / B5 纵向、横向，自定义毫米尺寸和 36–1200 DPI。
- 打开、粘贴、拖放导入 PNG / JPEG / WebP / GIF（首帧）/ BMP。
- 自由移动、四角等比缩放、滚轮缩放、平移、适应窗口。
- 图层上移/下移，最多 60 步撤销。
- 九种系统字体、字号、文字颜色、边缘色和描边宽度。
- 画笔、打码、橡皮、箭头、边框、矩形、圆形。
- 截图小条：窗口截图（蓝色高亮当前窗口）或矩形选区，完成后自动置入画布。
- PNG / JPEG / WebP / BMP 导出，质量与透明背景控制。
- 中英文外置语言文件。
- 未保存关闭确认。

### 安装

下载 `FreeCollage_v1.0.0_win64.zip`，解压后运行 `自由拼图.exe`。支持 Windows 10/11 x64。

### 许可证

GPL-3.0-only。源码：https://github.com/heiheids/free-collage

---

## English

### Features

- Native Rust + Win32 collage/annotation app; one EXE, no runtime DLLs.
- Starts on an unlimited transparent canvas with auto-crop.
- A4/B5 portrait and landscape, custom millimetres, 36–1200 DPI.
- Import via dialog, paste or drag-and-drop: PNG/JPEG/WebP/GIF (first frame)/BMP.
- Move, corner-scale, wheel zoom, pan, fit.
- Layer up/down and 60-step undo.
- Nine system fonts, size, text colour, outline colour and width.
- Brush, mosaic, eraser, arrow, border, rectangle and ellipse.
- Capture bar: highlighted window capture or rectangular region capture; the result is inserted automatically.
- PNG/JPEG/WebP/BMP export with quality and transparency controls.
- External Chinese/English language files.
- Unsaved-change close confirmation.

### Install

Download `FreeCollage_v1.0.0_win64.zip`, extract and run `自由拼图.exe`. Windows 10/11 x64.

### License

GPL-3.0-only. Source: https://github.com/heiheids/free-collage

---

## Build

```powershell
cargo test --locked
cargo build --release --locked
```

See [docs/BUILD.md](BUILD.md) for toolchain, packaging and release details.
