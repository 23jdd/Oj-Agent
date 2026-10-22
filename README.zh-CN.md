# OJ Agent

[English](README.md)

Rust 原生桌面对话工作区，用于与 AI 讨论代码、检查算法，并通过动画观察执行过程。
## 预览
![OJ Agent 动画预览](docs/preview.gif)

## 快速开始

需要 **Rust 1.97.1+** 和所在平台的 C/C++ 构建工具。

```sh
cargo run --locked
```

1. 打开 **模型设置**，选择 OpenAI、DeepSeek 或兼容接口，填写模型名称和 API Key。兼容接口需要填写地址；使用服务商默认地址时留空。
2. 保存后输入消息，点击 **发送** 或按 **Ctrl+Enter**。输入 `@` 或点击 **@ 添加文件**，添加 UTF-8 文本或源文件。
3. AI 按需回复 Markdown、验证代码或生成动画。点击动画链接，在**应用内**进入播放页，点击 **返回对话** 回到会话。
4. AI 回复期间可切换或新建会话。**停止**只取消当前会话；**清除提示**只关闭错误栏，不停止任务或删除记录。

发布构建使用 `cargo build --release --locked`，完成后运行 `target/release/` 中的可执行文件。

## 主要功能

- 持久化对话，支持 Markdown、可选择的代码块、表格和附件快照。
- 多个会话并行回复，分别保留草稿、附件、状态和停止操作。
- 按会话统计输入、输出及累计 Token，包含工具、回答修复和恢复请求；接口未上报的用量会明确标记，不进行估算。
- AI 按需验证代码、生成应用内动画链接，无需选择算法或填写样例表单。
- 一般、边界及特殊情况各有独立时间线，支持播放、定位、单步、倍速及 GIF/JSON 导出。
- 由真实验证执行记录编译的树、图、链表、二维表格与指针可视化。
- Slint 无边框界面与 Skia 渲染，支持配色预设、系统或本地字体，以及外观配置保存。
- 可配置超时、重试、输出预算、思考强度和 Agent 轮次。

## 模型与外观设置

**模型设置**可保存连接信息和请求策略。保存后用于后续消息，进行中的回复保留原配置；无效值不能保存，取消会放弃草稿。

| 请求设置 | 默认值 |
| --- | --- |
| 单次请求超时 | 90 秒 |
| 额外重试次数 | 2 次；设为 0 时只请求一次 |
| 首次重试间隔 | 500 毫秒，逐次翻倍，最多 30 秒 |
| 普通 / 恢复输出上限 | 8192 / 16384 Token |
| 无正文恢复 | 1 次，占用额外重试次数 |
| 每条消息最大轮次 | 8 轮 |
| 思考强度 | 跟随模型默认 |
| DeepSeek 恢复时关闭思考 | 开启 |

Token 上限和思考选项仍须在所选模型支持范围内。**恢复请求默认值**只重置请求草稿，保留连接信息。数值范围及 JSON 字段见[请求配置](docs/CHAT.md#request-settings--请求配置)。

**外观**提供配色预设、本地 TTF/OTF/TTC 字体导入、字号和尺寸设置，保存前可查看草稿预览。详见[外观配置](docs/STYLE.md)。

## 验证范围

对话工具通过有界 QuickJS VM 执行 JavaScript，支持 Windows、Linux 和 macOS，不使用 Docker，也不提供主机文件、网络和进程接口。每个测试限制执行时间、内存和栈。

对 AI 转写的 Python/C++/Rust 算法会明确标注验证依据，不会声称执行了原程序。提供的测试通过不代表对所有输入证明正确。详见[工具与验证](docs/CHAT.md#tools-and-verification)。

## 本地数据

对话、附件快照、Token 用量、工具报告和动画保存在 SQLite 中。发送的消息及所选附件会提供给配置的模型服务商。

| 设置 | 用途 |
| --- | --- |
| `OJ_DATA_DIR` | 指定配置目录，包含 `sessions.sqlite3`、`model.json` 和 `style.json` |
| `OJ_MODEL_CONFIG` | 单独指定模型配置文件 |
| `OJ_STYLE_CONFIG` | 单独指定外观配置文件 |

默认目录位于系统应用数据目录。模型配置以明文保存 API Key，请妥善保管 `model.json`。清除错误提示只关闭显示；再次失败时会出现新的提示。

## 开发检查

```sh
cargo fmt --check
cargo check --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

执行 `cargo run --locked -- tool` 可查看 CLI 工具。详见[使用与命令行示例](docs/USAGE.md)。

## 文档

[对话与沙箱](docs/CHAT.md) · [使用](docs/USAGE.md) · [外观](docs/STYLE.md) · [渲染器](docs/RENDERER.md) · [架构](ARCHITECTURE.md) · [IR](IR.md) · [工具](TOOL.md) · [会话](SESSIONS.md) · [错误](EXCEPTON.md) · [设计](DESIGN.md) · [许可证](LICENSE)
