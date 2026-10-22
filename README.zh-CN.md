# OJ Agent

[English](README.md)

OJ Agent 是一个 Rust 桌面对话工作区，用于讨论代码、验证算法，并打开可回放的动画。

![动画预览](docs/preview.gif)

## 使用

需要 Rust 1.97.1+ 和平台 C/C++ 构建工具。

```sh
cargo run
```

在“模型设置”中配置 OpenAI、DeepSeek 或兼容接口，然后直接对话。输入 `@` 添加文本或源文件。AI 判断是否需要动画；点击消息中的链接，在**应用内**打开动画页面。

## 主要功能

- 持久化对话，支持 Markdown、代码块、表格和附件快照。
- AI 按需验证代码、生成动画，无需选择算法类型或填写样例表单。
- 跨平台 JavaScript 沙箱，限制时间、内存和栈，无主机文件、网络和进程接口，不使用 Docker。
- 验证一般、边界及特殊情况，各情况对应独立动画时间线。
- Slint 界面、Skia 播放、GIF/JSON 导出，以及可保存的模型与外观设置。
- Rig 模型接口、校验后的 IR、SQLite 产物、取消和有限工具轮次。

跨平台沙箱执行 JavaScript；对 Python/C++/Rust 的 AI 转写验证会明确标注，不会声称执行了原程序。提供的测试通过不代表对所有输入证明正确。

## 文档

[对话与沙箱](docs/CHAT.md) · [使用](docs/USAGE.md) · [外观](docs/STYLE.md) · [渲染器](docs/RENDERER.md) · [架构](ARCHITECTURE.md) · [IR](IR.md) · [工具](TOOL.md) · [会话](SESSIONS.md) · [错误](EXCEPTON.md) · [设计](DESIGN.md) · [许可证](LICENSE.md)

```sh
cargo fmt --check
cargo check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
