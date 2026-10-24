# Usage and implementation

[文档索引 / Documentation index](README.md)

## Desktop

Run `cargo run`, open **模型设置**, and configure a provider, model ID and API key. Start a conversation, attach files with `@`, and send with **Ctrl+Enter** or **发送**. The model may answer in Markdown without producing animation. Generated animations appear as registered application-local links; click one to enter its native playback page and use **返回对话** to return. No browser or web server is used. See [chat, attachments and verification](CHAT.md).

The frameless window supports header dragging, double-click maximize/restore, native edge resizing and upper-right window controls. **会话** toggles local history. **外观** edits fonts, colors and geometry; [appearance settings](STYLE.md) persist separately from messages. **模型设置** saves the provider, model, endpoint and API key to a user-private local file; keys never enter sessions.

**清除提示** dismisses the selected conversation's error banner, including when
returning to it after switching conversations. It does not stop requests, delete
error records or clear errors in another conversation. New failures can show a
new notice. Model and appearance validation notices have their own clear action;
clearing them does not save or discard the edited values.

Generation and verification run off the UI thread. A dedicated Skia worker paints preview frames directly into shared pixels. The image uses the viewport's physical resolution and redraws after 120 ms of stable resizing, including while paused. Playback, steps, seeking, speed and case selection share the prepared timeline with export. See [renderer](RENDERER.md).

## Windows install and shortcuts

From the project directory, install or replace the executable:

```sh
cargo install --path . --locked --force
```

Close the installed app before updating it. Point a Windows shortcut directly to
`%USERPROFILE%\.cargo\bin\oj-agent.exe`, with no arguments. If you set `CARGO_HOME`
or an install root, use that directory's `bin/oj-agent.exe` instead. The executable
uses the Windows GUI subsystem in both debug and release builds, so ordinary
shortcut launches do not open a terminal. Startup failures show an error dialog.
Existing shortcuts targeting the installed executable continue to work after updating.

The Windows executable embeds the same OJ icon as the application window; no
separate icon file is required after installation. If an existing shortcut keeps
the generic icon after updating, open its **Properties → Change Icon**, select the
installed `oj-agent.exe`, and choose the OJ icon, or recreate that shortcut.

Windows 下更新前先关闭已安装的应用，再执行上述命令。快捷方式直接指向安装后的
`oj-agent.exe`，不要通过 `cmd.exe`、PowerShell 或批处理文件启动；无参数启动只显示
应用窗口。若仍弹出终端，请检查快捷方式是否指向旧版本或终端包装脚本。

EXE 已内嵌 OJ 图标，无需另外复制图片。如果旧快捷方式仍显示默认图标，在
**属性 → 更改图标** 中选择更新后的 `oj-agent.exe`，或重新创建快捷方式。

CLI arguments attach to an existing parent console and preserve redirected output;
they never allocate a new terminal. When scripting, wait for the GUI-subsystem
process to finish: use `oj-agent.exe --help | Out-Host` in PowerShell, or
`start /wait "" oj-agent.exe --help` in Command Prompt.

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
