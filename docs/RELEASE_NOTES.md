# 自由拼图 v1.0.2 / Free Collage v1.0.2

本次更新新增图片裁切。  
This release adds image cropping.

## 中文

### 新增：图片裁切

- 左栏“磁吸”下方新增 **裁切** 按钮；只有选中图片图层时才可用，否则按钮置灰。
- 点击后在图片上拖出选区，选区外压暗，边界显示蓝色虚线。
- 松开鼠标弹出确认框；选择“裁切”只保留选区内像素并删除其余部分，选择“取消”则退出裁切且不修改图片。
- 裁切保留图片原位置和显示比例，并支持撤销。
- 仅裁切图片图层；文字、形状等不会受影响。

### 升级

下载 `FreeCollage_v1.0.2_win64.zip`，解压后运行 `自由拼图.exe`。支持 Windows 10/11 x64；原有 `settings.cfg` 可继续使用。

### 许可证

GPL-3.0-only。源码与完整文档：https://github.com/heiheids/free-collage

---

## English

### Added: image cropping

- New **Crop** button below Snap in the left toolbar; it is enabled only when an image layer is selected and is greyed out otherwise.
- Click it, then drag a selection over the image. The outside is dimmed and the boundary is a blue dashed rectangle.
- Releasing the mouse opens a confirmation dialog. **Crop** keeps only the selected pixels and removes the rest; **Cancel** exits without changing the image.
- Cropping preserves the image's position and display scale and can be undone.
- Only image layers are cropped; text and shapes are unaffected.

### Upgrade

Download `FreeCollage_v1.0.2_win64.zip`, extract and run `自由拼图.exe`. Windows 10/11 x64; the existing `settings.cfg` remains compatible.

### License

GPL-3.0-only. Source and full documentation: https://github.com/heiheids/free-collage

---

## Build

```powershell
cargo test --locked
cargo build --release --locked
```

See [docs/BUILD.md](BUILD.md) for toolchain, packaging and release details.
