# AGENTS.md

## Tech

- Rust: application implementation.
- Slint: desktop UI.
- skia-safe: OJ algorithm visualization and animation rendering.
- Rig: agent framework and model provider integration.
- nanosandbox: isolated execution of OJ programs.
- Serde and serde_json: serialization and deserialization of JSON data.
- JSON Schema: define the structure and constraints of JSON documents.
- jsonschema: validate JSON documents against JSON Schemas.
- SQLite: local session storage.
- anyhow: application-level error handling; follow `docs/reference/ERROR_HANDLING.md` for error classification and recovery.

## Target

- Refactor and update the original project, replacing Go with Rust.
- As part of the migration, remove original project files except Markdown (`*.md`) documentation. Identify legacy files before removal; this cleanup does not apply to newly created Rust source, configuration, or required assets.
- Preserve unrelated user changes and follow `GIT.md` when removing tracked files.

## Rust Module Layout

- Use named module entry files such as `src/renderer.rs`; do not create `mod.rs`.
- Put child modules beside their entry file in the matching directory, for example
  `src/app.rs` with `src/app/preview.rs`.

## Model Providers

- Expose a common model interface backed by Rig for supported providers, including OpenAI and DeepSeek.
- Allow users to select a provider and model and configure the required credentials and endpoint.
- Keep provider-specific implementation details behind the common interface.

## UI Design

All UI implementation and modification MUST follow [the current UI design rules](docs/reference/DESIGN.md).
The active direction is Neobrutalism, as requested on 2026-10-22, following
https://github.com/neobrutalism/neobrutalism. Slint remains the native UI framework.

- Read the design document and inspect nearby components before editing UI.
- Reuse existing Slint components, centralized theme tokens and visual hierarchy.
- Use layouts for ordinary forms and panels; use coordinates only for canvas or overlay content.
- Keep text readable, spacing consistent, interaction states clear and controls accessible.
- Preserve the conversation workflow, application-local animation links and independent sessions.
- Do not redesign unrelated UI or introduce a one-off visual pattern.
- Complete the design document's final review and applicable native-window checks.

Reusable component, layout, typography, interaction and review rules are maintained
in the design document. Previous ink-wash rules are retained only in
[the historical archive](docs/archive/INK_WASH_DESIGN.md), not as current requirements.

## Project Documentation

- Documentation index: [docs/README.md](docs/README.md).
- Git workflow: [GIT.md](GIT.md). All Git operations must follow this document.
- Architecture: [ARCHITECTURE.md](docs/reference/ARCHITECTURE.md).
- Error handling and recovery: [ERROR_HANDLING.md](docs/reference/ERROR_HANDLING.md).
- Intermediate representations: [IR.md](docs/reference/IR.md).
- Tools: [TOOLS.md](docs/reference/TOOLS.md).
- Sessions: [SESSIONS.md](docs/reference/SESSIONS.md).
- UI design: [DESIGN.md](docs/reference/DESIGN.md).

## Acceptance Criteria

For Rust code changes, run the relevant checks from the Rust project or workspace root:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy
```

- All applicable checks must pass without introducing new warnings.
- For documentation-only changes, check links and review the diff; Rust checks are not required.
- If a required check cannot run, report the reason and the remaining verification work.

## README

- Keep the description and usage instructions concise.
- Include a GIF preview demonstrating the application.
- List the main features and links to project documentation.
- Provide English and Chinese versions.
