# OJ Agent

[中文](README.zh-CN.md)

A native Rust desktop workspace for discussing code with AI, checking algorithms and exploring their execution as animations.

## Preview

![OJ Agent animation preview](docs/preview.gif)

## Quick start

Requires **Rust 1.97.1+** and your platform's C/C++ build tools.

```sh
cargo run --locked
```

1. Open **模型设置** (Model settings), choose OpenAI, DeepSeek or a compatible provider, and enter the model ID and API key. Compatible providers require an endpoint; leave it empty for a provider's default address.
2. Save, then send a message with **发送** or **Ctrl+Enter**. Type `@` or click **@ 添加文件** to attach UTF-8 text/source files.
3. AI can reply in Markdown, verify code or generate an animation when useful. Click an animation link to open playback **inside the app**, then choose **返回对话** to return.
4. Switch conversations while replies run in the background. **停止** cancels only the selected conversation; **清除提示** dismisses its error banner without stopping work or deleting records.

For a release build, run `cargo build --release --locked` and launch the executable in `target/release/`.

## Features

- Persistent conversations with Markdown, selectable code blocks, tables and attachment snapshots.
- Concurrent AI replies with separate drafts, attachments, status and cancellation per conversation.
- Per-conversation input, output and total Token usage, including tool, repair and recovery requests. Missing provider counters are labeled rather than estimated.
- Optional code verification and application-local animation links; no algorithm selectors or sample forms.
- Normal, boundary and adversarial cases with separate playback timelines, seeking, stepping, speed control and GIF/JSON export.
- Structured tree, graph, linked-list, grid and pointer visuals compiled from verified execution traces.
- Frameless Slint UI with Skia rendering, palette presets, installed/local fonts and saved appearance settings.
- Configurable timeouts, retries, output budgets, reasoning strength and agent rounds.

## Model and appearance settings

**模型设置** saves connection details and request behavior. Changes apply to subsequent turns; a running reply keeps its configuration. Invalid values cannot be saved, and Cancel discards the draft.

| Request setting | Default |
| --- | --- |
| Timeout per request | 90 seconds |
| Additional retries | 2; set 0 for a single attempt |
| Initial retry delay | 500 ms, doubling up to 30 seconds |
| Output / recovery output limit | 8192 / 16384 Token |
| Recovery requests without a final answer | 1, within the shared retry allowance |
| Agent rounds per message | 8 |
| Reasoning | Provider default |
| Disable thinking during DeepSeek recovery | On |

Limits and reasoning options must be supported by the selected model. **恢复请求默认值** resets the request draft while keeping connection details. See [request configuration](docs/CHAT.md#request-settings--请求配置) for ranges and JSON fields.

**外观** (Appearance) offers color presets, local TTF/OTF/TTC import, font sizes and geometry controls, with a draft preview before saving. See [appearance configuration](docs/STYLE.md).

## Verification scope

The chat tool executes JavaScript in a bounded QuickJS VM on Windows, Linux and macOS, without Docker or host filesystem, network and process APIs. Each case has time, memory and stack limits.

AI-translated Python/C++/Rust algorithm checks are labeled as translations; they do not execute the original program. Passing the supplied cases does not prove correctness for every input. See [tools and verification](docs/CHAT.md#tools-and-verification).

## Local data

Conversations, attachment snapshots, usage, tool reports and animations are stored in SQLite. Messages and selected attachments are sent to the configured model provider.

| Setting | Purpose |
| --- | --- |
| `OJ_DATA_DIR` | Profile directory containing `sessions.sqlite3`, `model.json` and `style.json` |
| `OJ_MODEL_CONFIG` | Override the model settings file |
| `OJ_STYLE_CONFIG` | Override the appearance settings file |

The default profile uses the operating system's application data directory. Model settings contain the API key in plain text: keep `model.json` private. Clearing an error only dismisses its display; a later failure can show a new message.

## Development

```sh
cargo fmt --check
cargo check --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run `cargo run --locked -- tool` to list CLI tools. See [usage and CLI examples](docs/USAGE.md).

## Documentation

[Documentation index](docs/README.md) · [Usage](docs/USAGE.md) · [Chat and verification](docs/CHAT.md) · [Appearance](docs/STYLE.md) · [Rendering](docs/RENDERER.md)

Contributors: [Agent instructions](AGENTS.md) · [Git workflow](GIT.md) · [License](LICENSE)
