# 自由拼图 · 技术说明

> 项目：Free Collage / 自由拼图  
> 版本：1.0.1  
> 平台：Windows 10/11 x64  
> 许可证：GPL-3.0-only  
> 代码：https://github.com/heiheids/free-collage

本文描述程序的结构、关键数据流和实现取舍，面向希望阅读、修改或审查源码的开发者。用户使用说明见 [README](../README.md)。

## 1. 目标与约束

自由拼图的目标是一个**启动即用、单 EXE、无额外运行时 DLL** 的 Windows 拼图 / 标注工具：

- 启动不询问图片类型或画布尺寸，直接进入透明自由画布。
- 图片按原始像素尺寸落图，视图缩放不改变图层尺寸。
- 支持自动裁切和 A4/B5/自定义页面。
- 界面、文字和导出保持一致。
- 截图工具与主程序在同一个进程、同一个 UI 线程中完成。
- 不依赖 Electron、Qt、WebView 或第三方 UI 运行时。
- 发布版使用静态 CRT、LTO、体积优化、符号剥离和 `panic=abort`。

程序只面向 Windows；不使用跨平台窗口抽象。这样可以直接使用 Win32 的 DPI、输入、剪贴板、DWM 和 GDI 截图能力，同时把依赖保持在五项直接依赖。

## 2. 技术栈

| 组件 | 用途 |
| --- | --- |
| Rust 2021 | 主语言；MSVC target |
| `windows-sys` | 原生 Win32 / GDI / DWM / 剪贴板 / 文件对话框绑定 |
| `tiny-skia` | 抗锯齿 2D 光栅化、UI 与文档渲染、导出 |
| `ab_glyph` | 系统 TTF/TTC 字体轮廓解析与灰度字形栅格化 |
| `image` | PNG / JPEG / BMP / GIF（首帧）解码 |
| `webp` / bundled `libwebp` | WebP 解码和编码 |

文本编辑使用一个原生 `EDIT` 子窗口叠加在画布上，以支持中文 IME；其余界面全部由 tiny-skia 自绘。

## 3. 目录结构

```text
src/
  main.rs                  进程入口
  app/
    mod.rs                 App 状态、绘制入口、窗口命令、弹窗
    actions.rs             打开、粘贴、导出、画布、图层动作
    input.rs               鼠标、键盘、拖动、命中
    text.rs                文字层创建与原生 EDIT 编辑器
  model/
    document.rs            Document、页面、内容边界
    layer.rs               Layer、LayerKind、文字/形状/图片
    canvas.rs              CanvasMode、A4/B5/DPI、像素尺寸
    history.rs             撤销栈（最多 60 步）
    geometry.rs            Point / Rect / 变换
    style.rs               Rgba、颜色目标、调色板
  render/
    mod.rs                 Renderer / Painter、布局、命中、面板缓存
    doc.rs                 文档渲染、页面视图、图片缩放、棋盘格
    export.rs              扁平导出、质量、透明背景
    overlays.rs            弹窗、模态框、保存/关于/关闭确认
    icons.rs               图标路径、圆角矩形、阴影/描边辅助
    theme.rs               颜色与主题
  platform/
    window.rs              主窗口、WndProc、DPI、命中、消息循环
    capture.rs             截图小条、全屏 overlay、窗口选择、GDI 截图
    gdi.rs                 字体引擎、DIB 呈现、GDI 辅助
    clipboard.rs           CF_DIB/PNG/HDROP 读取
    dialogs.rs             原生打开/保存对话框
    system.rs              文件路径、系统语言、窗口圆角
  codec.rs                 图片解码/编码
  config.rs                settings.cfg 读写
  i18n.rs                  语言文件加载
  state.rs                 运行状态、弹窗、视图、文字编辑状态
  tools.rs                 工具枚举与拖动状态
  ui.rs                    Layout、Rect、HitId
build.rs                   图标资源编译、语言/图标复制
assets/
  app.ico                  EXE 图标
  app.rc                   版本资源
  screenshot.png           README 截图
languages/
  zh-CN.lang
  en-US.lang
tools/                     开发辅助脚本（备份、截图、点击、图标生成）
```

模块按“模型 / 渲染 / 平台 / 应用”分离；没有数千行的单文件。

## 4. 启动与窗口

`main.rs` 调用 `platform::window::run()`。主窗口使用无边框 `WS_POPUP` + 自定义 DPI 感知：

- 根据显示器工作区计算默认窗口大小，并限制最小约 940×680。
- `WM_NCCALCSIZE` 去掉系统非客户区，客户区即整个自绘窗口。
- `WM_NCHITTEST` 处理八方向缩放边和标题栏拖动区。
- **标题栏交互区**（画布 chip、截图、语言、关于、最小化/最大化/关闭）必须返回 `HTCLIENT`，否则真实鼠标点击会被当作拖动标题栏。截图按钮曾因此失效，当前已加入 `layout.screenshot`。
- `WM_DPICHANGED` 更新 DPI，重新布局；`GetDpiForWindow` 用于查询。
- 使用 DWM 圆角/系统主题辅助，但主体由自绘完成。
- `WM_CLOSE` 不直接销毁：有未保存改动时进入 `CloseConfirm` 模态，保存成功后再异步 `WM_CLOSE`。

UI 线程是唯一线程。截图 overlay 也运行在同一消息循环中；没有跨线程 UI 访问。

## 5. 状态与模型

`App` 持有：

- `State`：文档、视图、工具、选择、悬浮、弹窗、模态、提示、文字编辑、配置。
- `Renderer` / `TextEngine`：渲染器和字体缓存。
- 缓存：`buffer: Vec<u32>`（BGRA 呈现）、`layout`、`hits`、`frame` Pixmap（复用）。
- `hwnd`、DPI、编辑器 HWND、`force_close` 等平台状态。

`Document` 包含 `CanvasMode` 与 `Vec<Layer>`：

- `CanvasMode::Auto`：不固定页面，导出按可见内容边界裁切。
- `CanvasMode::Fixed { width_mm, height_mm, dpi }`：A4/B5/自定义；支持横竖互换。
- `LayerKind`：Image / Text / Stroke / Shape / Mosaic。
- 每个图层有 id、矩形、可见性、层级顺序。
- `History` 保存快照，限制 60 步；图片、文字、形状、打码、橡皮操作都进入历史。

视图使用 `zoom` + `pan`；屏幕坐标与世界坐标通过 `page_screen_rect()` 和逆变换互转。

## 6. 渲染管线

### 6.1 一帧的生命周期

`App::paint(hdc)`：

1. `sync_client_size()`：检查客户区和 DPI。
2. `Renderer::render()`：创建/复用 `Pixmap`，构造 `Layout`，执行 `paint_base()`、`paint_popup()`、`paint_toast()`、`paint_modal()`。
3. 更新 `App.layout` 与 `hits`。
4. `gdi::present()` 将 RGBA Pixmap 转为 BGRA `Vec<u32>`，用 `StretchDIBits` 呈现。
5. 将 Pixmap 还给 `Renderer` 复用，避免每帧 7MB 分配。

`Frame` 持有 Pixmap、Layout 和命中列表；绘制和命中测试共享同一布局，避免坐标不一致。

### 6.2 UI 布局与命中

`ui.rs::Layout` 根据物理客户区尺寸和 DPI 计算：

- 大窗口条件：宽度 ≥ 1400、高度 ≥ 980；使用 `ui_boost = 1.5`。
- 小窗口使用紧凑尺寸；画布 chip 在紧凑模式只显示短标签。
- 所有可点击区域写入 `hits: Vec<Hit>`，`hit_at()` 按顺序返回最上层命中。
- 悬浮/按下状态只影响绘制和光标，不改变布局。

### 6.3 基础界面

`paint_base()` 顺序：

1. 标题栏。
2. 工具栏。
3. 画布。
4. 底部属性栏。
5. 选择框。

标题栏包括程序名、画布 chip、截图按钮、语言、关于、最小化/最大化/关闭。除程序名外，顶部文字使用统一字号。

工具栏为双列：左列操作，右列绘制工具。底部是颜色、字体、描边、画笔、打码、缩放、适应和实际尺寸。

### 6.4 画布与页面

- `Auto` 页面使用透明棋盘格；固定页面使用白色页面。
- `render_page_view()` 只渲染当前**可见页面区域**，不是整张 300 DPI 页面：它计算 `page_screen_rect` 与画布视口的交集，创建视口大小的 Pixmap。
- 页面渲染完成后直接 `blit_opaque` 到帧缓冲；页面背景不透明，不需要 SourceOver 混合。
- 图片层按目标像素尺寸采样，避免 tiny-skia 通用变换遍历整张源图。
- 棋盘格直接写像素，不逐格创建 tiny-skia path。
- 选择框和八个手柄直接画到帧缓冲，避免每帧创建整块画布大小的 overlay。

### 6.5 文字

`platform::gdi::TextEngine`：

- 从系统字体目录加载 TTF/TTC，支持微软雅黑、宋体、黑体、楷体、仿宋、Segoe UI、Arial、Times New Roman、Consolas。
- 用 `ab_glyph` 解析轮廓并光栅化为灰度覆盖 mask。
- mask 按 `(text, pixel_size, weight, family)` 缓存，默认上限约 600 条；测量结果单独缓存。
- `blit_mask()` 把灰度 mask 合成到 Pixmap。
- 描边通过 16 方向偏移 mask 叠加实现；文字和导出使用同一条路径。
- 文字编辑使用原生 `EDIT` 子窗口以支持 IME；提交后转为普通文字层。

### 6.6 渲染性能

关键优化（均在 Release 下测量）：

- 工具栏/底栏圆角面板背景缓存为 Pixmap，每帧直接内存拷贝。
- 移除大面积运行期阴影；保留描边，避免每帧数十毫秒的路径填充。
- 图片交互使用目标尺寸最近邻采样；小目标或导出使用目标尺寸双线性采样。
- 棋盘格直接写像素。
- 选择框直接绘制。
- 帧 Pixmap 复用。
- 帧缓冲 RGBA→BGRA 使用 `u32` 批量转换。
- 鼠标移动只在拖拽、悬停变化、画笔环或滑杆拖动时触发重绘；普通移动不再全窗口重绘。

典型开发机（1650×1075、Release）：空画布约 13ms/帧，Auto + 1000×700 图片约 12ms/帧，A4 + 图片约 9.5ms/帧。实际数值取决于 CPU、图层数量和缩放比例。

## 7. 输入与工具

`app/input.rs` 处理：

- 左键：命中、选择、拖动、缩放、绘制、文字编辑。
- 右键：图层菜单、平移（画布空白）。
- 中键/空格：平移。
- 滚轮：以光标为中心缩放。
- 双击文字：重新编辑。
- 键盘：工具快捷键、撤销、删除、适应、100%、Tab 切换图层、Esc 取消。

`tools.rs` 定义工具枚举和拖动状态（`Drag::Pan`、`Move`、`Resize`、`Stroke`、`ShapeCreate`、`TextSelect` 等）。拖动更新只修改状态并重绘，提交时才写入历史。

### 磁吸吸附

“磁吸”是左栏 Save 下方的独立开关，不是绘制模式。开启后只影响图片图层的**移动**：

- 阈值固定为屏幕 8px，按 `8 / zoom` 换算成文档坐标，因此不同缩放级别手感一致。
- 水平方向比较移动图片的左、中、右三条边与目标图片/固定页面的左、中、右三条边；垂直方向同理。
- 两个方向独立选择最近边，所以可以同时吸附一条竖边和一条横边；目标图片中心和固定页面中心也参与。
- 目标为其他可见图片图层，以及固定画布的页面边缘/中心；自动裁切画布没有页面目标。
- 只移动图片图层；文字、形状等不会被磁吸。
- 纯计算在 `src/model/snap.rs`，有同时吸附、超出阈值、页面吸附的单元测试。

## 8. 画布与导出

- 图片层保留原始 Pixmap；图层矩形独立于视图缩放。
- 自动裁切使用所有可见图层的内容边界；透明区域不参与边界。
- 固定页面按页面像素尺寸渲染，超出内容被页面裁切。
- 导出使用 `render_document(..., smooth=true)`，图片使用高质量双线性采样；屏幕交互对较大目标使用最近邻以降低延迟。
- `export.rs` 支持 PNG/JPEG/WebP/BMP，JPEG 自动填白，PNG/WebP 保留透明。

## 9. 截图子系统

`platform/capture.rs` 独立于主窗口，使用两个顶层窗口：

### 截图小条

- 主窗口 `ShowWindow(SW_MINIMIZE)` 后，创建置顶、无边框的小条。
- 小条为白色直角窗口、2px 灰色边框、浅色按钮；仅右侧关闭按钮，左侧无关闭。
- 点击“截图窗口”或“矩形选择”进入对应模式；右侧 `×` 取消并恢复主窗口。
- GDI 自绘，尺寸 480×64；按实际客户区填充，边框使用贴边实心条，避免底/右白边。

### 全屏 overlay

- 进入模式前先 GDI `BitBlt` 捕获整个虚拟桌面到内存 DIB，并隐藏小条。
- overlay 覆盖虚拟屏幕，绘制桌面快照，而不是直接截取 overlay 自身。
- 矩形选择：鼠标拖动矩形，松开后从桌面快照裁剪。
- 窗口选择：
  - 鼠标移动时按 Z 序枚举顶层窗口，跳过 overlay/小条/最小化的主窗口，找到光标下的真实窗口。
  - 在 overlay 上绘制 4px 蓝色高亮边框。
  - 点击时以高亮窗口为目标；隐藏 overlay，将目标窗口前置，再用屏幕 `BitBlt` 捕获其窗口矩形。
- 截图完成后将图像写入剪贴板（CF_DIB），恢复主窗口并发送 `WM_PASTE`，由主程序走正常粘贴流程加入图层。
- 关闭/取消不修改剪贴板和画布。

该实现不需要新增 crate，全部基于 `windows-sys` + GDI。

## 10. 配置与多语言

- `settings.cfg` 位于 EXE 同目录，纯文本 `key=value`；保存语言、画布、颜色、字体、字号、画笔、打码、导出格式/质量/透明和上次目录。
- `languages/*.lang` 为 UTF-8 文本，每行 `key=value`，`#` 注释；中英文键集合必须一致。
- 启动时按系统语言选择；右上角语言菜单可切换“跟随系统”或指定语言。
- 语言文件外置，新增语言不需要重新编译。

## 11. 构建与资源

- `build.rs` 在构建时复制 `languages/` 和 `assets/app.ico` 到产物目录。
- 查找 `rc.exe`（`RC` 环境变量、PATH、Windows Kits），编译 `assets/app.rc`，嵌入图标和版本资源。
- 找不到 `rc.exe` 时给出 warning，仍能生成 EXE，但没有图标/版本资源。
- `.cargo/config.toml` 对 `x86_64-pc-windows-msvc` 使用 `+crt-static`。
- Release profile：`opt-level = "s"`、LTO、单 codegen unit、strip、`panic = "abort"`。
- 依赖静态链接；没有额外 DLL。

## 12. 测试

```powershell
cargo test --locked
```

测试覆盖：

- A4/B5/横竖像素尺寸
- 几何 union/intersect
- 文档内容边界与透明裁切
- 图片缩放绘制
- PNG/WebP 透明、JPEG 白底、BMP 往返
- 文字导出
- 字体加载、CJK 灰度 mask、字体族/描边
- 语言文件解析和简介长度

## 13. 已知限制

- 仅 Windows；没有跨平台后端。
- 单个 UI 线程；超大画布或大量高分辨率图层仍可能增加帧时间。
- 动画 WebP 不支持；GIF 仅首帧。
- 导出为扁平图片，不保存图层工程；无 Redo。
- 字体来自系统，缺失时回退微软雅黑。
- 受 DRM/保护内容限制的窗口可能无法 GDI 截取。
- 截图使用屏幕 BitBlt，窗口被其他窗口遮挡时以屏幕实际内容为准。

## 14. 安全与隐私

- 程序不包含网络请求、遥测或自动上传。
- 设置、导出和备份均在本机文件系统。
- 截图数据只写入本机剪贴板，并由主程序读取；不离开进程/本机。
- 项目源代码以 GPL-3.0-only 发布；第三方组件许可见 `THIRD_PARTY_NOTICES.txt`。

## 15. 许可证

GPL-3.0-only。完整文本见 [LICENSE](../LICENSE)，版权声明见 [COPYRIGHT](../COPYRIGHT)。
