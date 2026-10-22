# Usage and implementation

## Desktop

Run `cargo run`, open **模型设置**, and configure a provider, model ID and API key. Start a conversation, attach files with `@`, and send with **Ctrl+Enter** or **发送**. The model may answer in Markdown without producing animation. Generated animations appear as registered application-local links; click one to enter its native playback page and use **返回对话** to return. No browser or web server is used. See [chat, attachments and verification](CHAT.md).

The frameless window supports header dragging, double-click maximize/restore, native edge resizing and upper-right window controls. **会话** toggles local history. **外观** edits fonts, colors and geometry; [appearance settings](STYLE.md) persist separately from messages. **模型设置** saves the provider, model, endpoint and API key to a user-private local file; keys never enter sessions.

Generation and verification run off the UI thread. A dedicated Skia worker paints preview frames directly into shared pixels. The image uses the viewport's physical resolution and redraws after 120 ms of stable resizing, including while paused. Playback, steps, seeking, speed and case selection share the prepared timeline with export. See [renderer](RENDERER.md).

## CLI

```sh
cargo run -- tool
cargo run -- tool sandbox.verify --input verify.json
cargo run -- validate output/animation-v1.json
cargo run -- render output/animation-v1.json output/preview.png --style config/style.example.json
```

The legacy `generate` command and built-in algorithm templates remain available for CLI demonstrations and existing IR tests. They are not the desktop input workflow. The previous `code.run` / `code.trace` native execution adapter remains separate from the portable chat VM; its Windows backend is experimental and disabled by default. The chat workflow never falls back to executing non-JavaScript source directly on the host.

## Data

The application data directory contains `sessions.sqlite3`, `model.json` and optional `style.json`. Set `OJ_DATA_DIR` to use an isolated or portable profile. `OJ_STYLE_CONFIG` and `OJ_MODEL_CONFIG` select other settings files. Messages, attachment snapshots, tool requests, verification reports and animation dependencies are stored locally. Old IR artifacts are retained.

## Checks

```sh
cargo fmt --check
cargo check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
