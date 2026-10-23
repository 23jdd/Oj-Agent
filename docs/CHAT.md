# Chat workflow / 对话工作流

[文档索引 / Documentation index](README.md)

The desktop is a conversation, not an algorithm configuration form. Set a provider, model and API key in **模型设置**, then send normal messages. Replies render Markdown headings, emphasis, lists, quotes, fenced code, tables and links. Code blocks are selectable and scroll when long. Use **Ctrl+Enter** to send; **停止** cancels model work and sandbox execution. Provider, model, endpoint and API key are saved to a local settings file; keys never enter sessions.

Conversations run independently: while A waits for an AI reply, switch to B or start a new conversation and send another request. A continues in the background; returning shows its own waiting state or completed reply. History marks running conversations with `…`. Each conversation permits one active reply or export, with its own cancellation token, status and errors. **停止** stops only the selected conversation. Deleting a conversation cancels its worker without interrupting the others; closing the application cancels all workers. Late events cannot overwrite another conversation or a newer request.

A 等待 AI 回复时，可直接切换到 B 或新建对话继续发送，无需先停止 A。各会话可同时回复；切换时分别保留草稿、附件和状态，后台回复不会显示到其他会话。停止和删除只影响当前会话。未发送的草稿与附件选择保留在当前应用进程中，重启后不会恢复。动画页返回后仍保留草稿，输入框和删除按钮保持可见。

Unsent drafts and attachment selections are kept per conversation for the current application run. Pending file-picker results return to the originating conversation even if another is selected. Returning from an animation preserves the current draft and keeps the composer and delete action within the window, including after resizing.

输入 `@`（位于单词开头）或点击“@ 添加文件”打开系统文件选择器。附件在添加时读取为快照，显示名称，可在发送前移除。每条消息最多 8 个 UTF-8 文本文件，每个不超过 100 KB；文件后续修改不会悄悄改变已发送内容。发送消息会将这些内容提供给所选模型，并保存到本机会话数据库中。应用不会扫描或自动上传其他文件。

AI may answer without animation, verify code without creating animation, or create animation when useful. An animation appears as `[打开动画](oj-animation:UUID)` in a message. This is an application-local identifier: clicking opens the native animation page, never a browser, localhost page or web service. The router checks that the artifact belongs to the current conversation. The page supports case selection, play/pause, seek, steps, speed, GIF and JSON export, and return to chat. Animation and message records persist together.

Large animations scroll horizontally and vertically at readable natural sizes. Text blocks expand and wrap fully; dense structures expand the canvas instead of shrinking labels and nodes. Playback controls stay visible while scrolling. 动画画布支持横向和纵向滚动；图形、标签保持正常尺寸，长说明完整换行，后续面板自动让出空间。暂停时滚动也会刷新，播放和返回按钮保持可见。

## Token usage / Token 用量

The footer shows input, output and total tokens for the selected conversation,
saved across application restarts. Counts come from each completed model response
through Rig, including tool rounds, JSON repairs and responses in later failed or
cancelled turns. Switching conversations switches the displayed totals too.
Cache hit/write and reasoning counters appear when reported; they are breakdowns
of input/output and are not added to the total again.

会话底部显示输入、输出及累计 Token；接口提供时另列缓存和推理用量。
未提供的字段显示“未上报”，部分请求缺失用量时显示 `≥` 已知小计，
真实的零值仍显示 `0`。不按字符数估算，也不补算旧会话的历史请求。
在收到接口用量前取消或发生网络错误，可能无法记录该请求已消耗的 Token。
这些数字是接口上报的对话用量，不是账单金额。

## Missing final answers / 无正文回复

The app distinguishes an empty answer, reasoning without a final answer, output
budget exhaustion, provider filtering and interrupted responses. Empty,
reasoning-only or truncated responses use the configured recovery allowance,
within the shared request retry limit. Defaults permit one recovery within three
total requests per completion. A truncated response uses the configured recovery
output cap (default 16384 rather than 8192). DeepSeek recovery requests by default
disable thinking for that request; other providers receive no
DeepSeek-specific options. The status line explains recovery, and **停止** can
cancel it. Provider filtering or interrupted output is surfaced without automatic
recovery. Partial output is never published as a completed answer.

DeepSeek 的思考内容与最终正文分开返回，思考可能占满输出预算而没有正文。
默认最多恢复一次，DeepSeek 恢复请求会关闭思考模式；这些行为均可配置，不修改
已保存的模型设置。恢复前后接口返回的用量都会累计；错误记录只保存结束原因、
预算等诊断信息，不保存思考内容。参见
[DeepSeek thinking mode](https://api-docs.deepseek.com/guides/thinking_mode/)。

## Request settings / 请求配置

Open **模型设置** to edit the following saved request policy. The form scrolls at
compact sizes while Save and Cancel stay visible. Drafts apply only after a
successful save, to subsequent turns; in-progress conversations retain their
own configuration snapshot. **恢复请求默认值** resets only the request draft,
leaving provider, model, endpoint and credentials unchanged. Old configuration
files gain defaults automatically.

| 设置 / JSON `request` 字段 | 默认值 | 范围 |
| --- | --- | --- |
| 单次超时 / `timeout_secs` | 90 秒 | 1–1800 |
| 额外重试 / `max_retries` | 2 次 | 0–5 |
| 首次重试间隔 / `retry_delay_ms` | 500 毫秒 | 0–30000 |
| 输出上限 / `max_output_tokens` | 8192 Token | 1–262144 |
| 恢复输出上限 / `recovery_output_tokens` | 16384 Token | 不低于普通上限，最多 262144 |
| 无正文恢复 / `empty_response_retries` | 1 次 | 0–5 |
| 每条消息轮次 / `max_rounds` | 8 轮 | 1–32 |
| 思考强度 / `reasoning` | `provider-default` | `provider-default` / `off` / `low` / `high` / `max` |
| DeepSeek 恢复时关闭思考 / `disable_thinking_on_recovery` | `true` | `true` / `false` |

额外重试包含网络故障、单次超时和无正文恢复：设为 0 时只请求一次，
即使无正文恢复次数大于 0 也不会追加请求。重试间隔逐次翻倍，最多 30 秒，
停止操作也可取消等待。超时按单次请求计算，不是整个对话的总时限。
输出上限及思考强度仍受所选模型支持范围限制。鉴权失败和其他不可重试错误
不会自动重试。Token 用量统计仍以每次接口实际返回的值为准。

## Tools and verification

- `sandbox.verify`: executes a JavaScript `solve(input, emit)` in a fresh [QuickJS VM](https://docs.rs/rquickjs/0.14.0/rquickjs/struct.Runtime.html) for each supplied JSON test case. Compares actual and expected results outside the VM. Supports Windows, Linux and macOS without Docker, WSL, external runtimes or host modules. Each case is limited to 2 seconds, 32 MiB engine memory, a 256 KiB stack and 256 trace steps. There are no host filesystem, networking, process, environment or package-loading APIs. These are language-VM boundaries, not a native OS process sandbox. The portable package has CI jobs for all three platforms; Windows is also exercised locally.
- `animation.create`: accepts only a verification ID actually produced in the current agent turn. Requires passed **normal, boundary and adversarial** cases, at least three distinct inputs, and a valid emitted trace for each. All cases compile into separate timelines. Layouts are `array`, `table`, `stack`, `queue`, `tree`, `graph`, `linked-list` and `grid`; they must match the emitted data. The compiler checks node/edge/pointer references, indices, settled values, structure changes, IR bounds and representative raster frames. Final state includes the observed return value. A failed or missing check produces a tool error rather than an animation link.

Structured observations use `emit({...})` with optional fields:
`nodes` (stable `id`, `label`, numeric `value`, semantic `state`, optional `x`/`y` in 0–1, optional `parent`), `edges` (`from`, `to`, optional `label`/`directed`), `pointers` (`name`, `target` = node ID, value index or `row,col`) and `cells` (`row`, `col`, `value`, optional `label`/`state`). Limits are 64 values, 128 nodes, 256 edges, 32 pointers, 256 cells and 8 KiB per step. Empty containers are valid. The renderer can show tree parent/child relationships, directed graphs, linked-list `next` edges, array/container pointers, edge labels and true row/column grids. Values-only traces remain compatible with the array/table/stack/queue layouts. The structured observation schema is exported as `schemas/observation-v1.schema.json`.

验证的含义是“提供的测试通过”，不是“对所有输入证明正确”。期望输出仍需来自题目、用户或独立推导。各情况的名称和分类由 AI 选择，不要求用户填写表单。动画是代码调用 `emit` 产生的执行观察；时间线和代表帧检查不证明每一帧都经过人工视觉审查。

**Language scope:** the portable VM executes original JavaScript. For Python, C++, Rust and other source files, AI can generate a JavaScript algorithm translation; the tool report and animation page explicitly label that basis. Such a result does **not** validate the original compiler, language runtime or complete source program. The previous native execution adapter remains a separate legacy facility; this chat workflow does not execute arbitrary native programs on the host.

The agent uses Rig through the common OpenAI / DeepSeek / compatible-endpoint interface. Its JSON action protocol permits at most two tool calls per response and a configurable number of rounds per turn (eight by default). Invalid JSON can be repaired within that same bound. Tool requests, actual reports, dependencies and published animation artifacts are persisted. Tool summaries are generated by the application, independently of the AI's final wording. Attachments are treated as untrusted data.

## Portable tool example

Save this request as `verify.json`, then run `cargo run -- tool sandbox.verify --input verify.json`:

```json
{
  "source": "function solve(input, emit) { emit({values: input, explanation: 'Sum input'}); return input.reduce((a,b) => a+b, 0); }",
  "cases": [
    {"name":"normal", "category":"normal", "input":[1,2,3], "expected":6},
    {"name":"empty", "category":"boundary", "input":[], "expected":0},
    {"name":"negative", "category":"adversarial", "input":[-3,3,-2], "expected":-2}
  ]
}
```

## Persistence and review

SQLite stores chat messages, attachment snapshots, tool calls, verification reports and animation bundles. Existing IR/CLI artifacts remain in the database. The UI no longer asks users to select algorithm templates. Use `OJ_DATA_DIR` to choose another profile directory; `OJ_STYLE_CONFIG` and `OJ_MODEL_CONFIG` can override the appearance and model settings files separately. The model settings file contains the API key in plain text and is restricted to the current user on Unix; do not share it or put API keys in attachments or code.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oj-sandbox
cargo run --example chat_fixture
```

`chat_fixture` creates an isolated review profile under `target/chat-review/profile`. Set `OJ_DATA_DIR` to that directory before launching the desktop, then open **对话界面预览** in history. It uses locally executed example code; no model credentials or requests are needed for this review fixture.
