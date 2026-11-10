# 构建与发布 / Build & Release

本文说明如何在 Windows 上从源码构建自由拼图，以及如何生成发布包和 GitHub Release。  
This document explains how to build Free Collage from source on Windows and how to produce release packages.

## 1. 环境要求 / Requirements

- Windows 10/11 x64
- Rust stable，MSVC toolchain / Rust stable with the MSVC toolchain
  - 本项目以 Rust 1.99 验证 / tested with Rust 1.99
  - `rustup default stable-x86_64-pc-windows-msvc`
- Visual Studio 2022 Build Tools（或完整 VS）
  - 工作负载：**使用 C++ 的桌面开发** / **Desktop development with C++**
  - 需要 MSVC `link.exe`、C/C++ 编译器（libwebp 的 C 源码需要）
- Windows 10/11 SDK
  - 需要 `rc.exe` 编译 `assets/app.rc`，嵌入图标和版本信息
  - 没有 `rc.exe` 也能构建，但 EXE 不含图标/版本资源
- Git（获取源码）

## 2. 获取源码 / Clone

```powershell
git clone https://github.com/heiheids/free-collage.git
cd free-collage
```

## 3. 测试与构建 / Test and build

```powershell
cargo test --locked
cargo build --release --locked
```

产物 / Output:

```text
target/release/PuzzleStudio.exe
target/release/languages/
target/release/assets/app.ico
```

`build.rs` 会自动：

- 复制 `languages/*.lang` 到产物目录；
- 复制 `assets/app.ico`；
- 查找 `rc.exe`，编译 `assets/app.rc` 并嵌入图标/版本资源。

发布包中的 EXE 重命名为 `自由拼图.exe`。

### 为什么使用 `--locked`

`Cargo.lock` 固定了完整依赖树，保证可复现构建和许可证审查一致。修改依赖后应先提交新的 `Cargo.lock`。

### 静态 CRT

`.cargo/config.toml` 对 `x86_64-pc-windows-msvc` 设置：

```toml
rustflags = ["-C", "target-feature=+crt-static"]
```

因此发布 EXE 不依赖 VC++ Redistributable，也没有额外 DLL。WebP 的 libwebp 源码也静态编译进 EXE。

### Release profile

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## 4. 打包 / Packaging

建议目录：

```text
自由拼图_v1.0.1_win64/
  自由拼图.exe
  assets/
    app.ico
    screenshot.png
  languages/
    zh-CN.lang
    en-US.lang
  docs/
    TECHNICAL.zh-CN.md
    TECHNICAL.en.md
    BUILD.md
    RELEASE_NOTES.md
  README.md
  LICENSE
  COPYRIGHT
  THIRD_PARTY_NOTICES.txt
  SHA256.txt
```

PowerShell 示例：

```powershell
$name = "自由拼图_v1.0.1_win64"
$dist = "dist\$name"
New-Item -ItemType Directory -Force $dist | Out-Null
Copy-Item target/release/PuzzleStudio.exe "$dist\自由拼图.exe"
Copy-Item assets, languages, docs -Destination $dist -Recurse -Force
Copy-Item README.md, LICENSE, COPYRIGHT, THIRD_PARTY_NOTICES.txt -Destination $dist
$hash = (Get-FileHash "$dist\自由拼图.exe" -Algorithm SHA256).Hash.ToLower()
"$hash  自由拼图.exe" | Set-Content "$dist\SHA256.txt" -Encoding utf8
Compress-Archive -Path $dist -DestinationPath "dist\$name.zip" -Force
```

发布包不应包含：

- `settings.cfg`（运行后生成）
- `target/`
- 工程备份
- 调试日志、`state-dump.txt`

## 5. 版本与标签 / Version and tag

版本在以下位置保持一致：

- `Cargo.toml` 的 `version`
- `assets/app.rc` 的 `PRODUCTVERSION` / `FileVersion` / `ProductVersion`
- Release 标签，例如 `v1.0.1`
- README / Release Notes

```powershell
git tag v1.0.1
git push origin v1.0.1
```

## 6. GitHub Actions

仓库包含 [`.github/workflows/build.yml`](../.github/workflows/build.yml)：

- 在 `windows-latest` 上 checkout、安装 stable Rust；
- 执行 `cargo build --release --locked`；
- 上传 `PuzzleStudio.exe`、语言文件、资源、README 和许可证为构建产物。

工作流只做构建，不自动创建 Release；Release 由维护者在本地构建、校验后手动发布。

查看运行：

```powershell
gh run list --repo heiheids/free-collage
gh run watch --repo heiheids/free-collage
```

## 7. GitHub Release

```powershell
gh release create v1.0.1 `
  "dist\FreeCollage_v1.0.1_win64.zip" `
  "dist\SHA256SUMS.txt" `
  --repo heiheids/free-collage `
  --title "自由拼图 v1.0.1 / Free Collage v1.0.1" `
  --notes-file docs/RELEASE_NOTES.md
```

GitHub 对 Release 资产名会过滤非 ASCII 字符，因此上传时使用 ASCII 文件名 `FreeCollage_v1.0.1_win64.zip`；ZIP 内的目录和可执行文件仍保留中文名。

Release 必须同时提供：

- 可运行 ZIP；
- EXE 与 ZIP 的 SHA-256；
- 对应源码标签（GitHub 自动提供 tag 源码归档）；
- GPL-3.0 许可证与第三方声明。

GPLv3 要求分发目标代码时提供对应源代码。GitHub 仓库 + 版本标签即对应源码；修改版发布时应保留同样义务。

## 8. 常见问题 / Troubleshooting

### `rc.exe` not found

安装 Windows 10/11 SDK，或把 `rc.exe` 所在目录加入 `PATH`，或设置：

```powershell
$env:RC = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.xxxxx.0\x64\rc.exe"
```

没有 `rc.exe` 时构建仍会成功，但 EXE 没有图标和版本资源。

### `link.exe` not found

安装 Visual Studio Build Tools 的“使用 C++ 的桌面开发”工作负载，并从 **x64 Native Tools Command Prompt** 运行，或使用 `rustup` 的 MSVC toolchain。

### libwebp / C compiler errors

`webp` crate 会编译随包的 libwebp C 源码。需要 MSVC C 编译器（`cl.exe`）；只安装 Rust 不够。

### 测试字体失败

文字测试依赖系统字体（微软雅黑等）。在精简 Windows 容器中可能缺少字体；标准 Windows 10/11 桌面环境应通过。

### 构建成功但界面无图标

确认 `assets/app.ico` 存在，且 `rc.exe` 编译成功；查看 Cargo 输出中的 `cargo:warning`。

## 9. 开发脚本 / Development scripts

`tools/` 包含开发辅助脚本（备份、截图、点击、图标生成）。它们不是运行时依赖，发布包不需要。

## 10. 许可证 / License

构建和分发的代码受 GPL-3.0-only 约束。完整文本见 [LICENSE](../LICENSE)，版权声明见 [COPYRIGHT](../COPYRIGHT)，第三方组件见 [THIRD_PARTY_NOTICES.txt](../THIRD_PARTY_NOTICES.txt)。
