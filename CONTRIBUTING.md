# 贡献指南 / Contributing

感谢你愿意改进自由拼图。  
Thank you for helping improve Free Collage.

## 许可证 / License

本项目以 **GPL-3.0-only** 发布。提交 Pull Request 即表示你同意以相同许可证贡献代码，且确认你有权提交这些代码。项目不使用 CLA。  
This project is licensed under **GPL-3.0-only**. By submitting a pull request you agree to license your contribution under the same terms and confirm you have the right to submit it. There is no CLA.

## 报告问题 / Bug reports

请提供：

- Windows 版本、系统语言、DPI 缩放；
- 应用版本（关于窗口或 Release 标签）；
- 复现步骤、期望结果、实际结果；
- 相关截图或 `F12` 生成的 `state-dump.txt`（请先检查其中是否有隐私内容）；
- 如果是构建问题，提供 `rustc -Vv`、`cargo -V` 和完整错误。

Please include Windows version, display scaling, app version, reproduction steps, expected/actual behavior, and relevant screenshots. Remove private data before attaching diagnostics.

## 开发环境 / Development

```powershell
cargo test --locked
cargo build --release --locked
```

要求 Windows 10/11 x64、Rust stable MSVC、Visual Studio Build Tools（C++ 桌面开发）和 Windows SDK。详见 [docs/BUILD.md](docs/BUILD.md)。

## 代码约定 / Code conventions

- 保持模块拆分，不要把大量逻辑堆进单个文件。
- UI、模型、平台代码分层：`model/` 不依赖 Win32；`platform/` 不包含业务规则。
- 新功能尽量补充单元测试；至少运行 `cargo test --locked`。
- 遵循现有命名、错误处理和中文/英文语言键约定。
- 新增界面文字必须同时更新 `languages/zh-CN.lang` 和 `languages/en-US.lang`，并保持键集合一致。
- 不提交 `target/`、`dist/`、工程备份、`settings.cfg`、日志或本机路径。
- 不引入不必要的依赖；优先使用现有五项依赖和 Win32 API。

## Pull Request

- 说明改动目的、用户可见行为和测试方式。
- 保持提交聚焦；格式化/重排不要和功能改动混在一起。
- UI 改动请附截图（含 100% 和 125% DPI，如适用）。
- 涉及 GDI/截图/窗口命中的改动，请说明真实鼠标验证结果，而不仅是消息模拟。

## 安全 / Security

不要在 Issue/PR 中粘贴 token、密钥、个人路径或未公开的隐私截图。安全问题请通过 GitHub 私下联系维护者。
