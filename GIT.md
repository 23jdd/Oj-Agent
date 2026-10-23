# Git Workflow Guidelines

This document defines the Git conventions for this repository.

The goal is to keep the repository history:

- clean
- readable
- easy to review
- easy to revert
- easy to understand

All contributors and coding agents should follow these rules.

---

## 1. General Rules

Before making changes:

```bash
git status
```

Understand the current working tree before editing files.

Do not overwrite unrelated user changes.

Do not delete or revert files that are unrelated to the current task.

Avoid modifying generated files unless explicitly required.

---

## 2. Keep Changes Focused

Each commit should represent one logical change.

Good examples:

```text
Add theme color tokens
Fix hover state for buttons
Refactor toolbar layout
Add dark theme support
Fix parser handling of escaped strings
```

Bad examples:

```text
update stuff
fix
changes
misc
final
test
```

Do not combine unrelated changes into one commit.

For example, avoid committing:

```text
UI redesign
+
parser fix
+
dependency update
+
README changes
```

as a single commit.

---

## 3. Commit Message Format

Use the following format:

```text
<type>: <description>
```

Example:

```text
feat: add ink wash theme
fix: correct button hover state
refactor: simplify theme tokens
docs: add design guidelines
test: add parser edge cases
perf: reduce render allocations
style: format UI components
chore: update dependencies
```

---

## 4. Commit Types

Allowed commit types:

### `feat`

New functionality.

```text
feat: add dark theme
```

---

### `fix`

Bug fix.

```text
fix: prevent dialog from opening twice
```

---

### `refactor`

Code restructuring without changing behavior.

```text
refactor: extract reusable button component
```

---

### `perf`

Performance improvements.

```text
perf: cache rendered text
```

---

### `docs`

Documentation changes.

```text
docs: add DESIGN.md
```

---

### `test`

Adding or updating tests.

```text
test: add string parser edge cases
```

---

### `style`

Formatting or purely stylistic code changes.

```text
style: format slint files
```

Do not use `style` for visual UI design changes.

A visual UI change is normally:

```text
feat:
```

or:

```text
fix:
```

---

### `chore`

Maintenance work.

Examples:

```text
chore: update dependencies
chore: configure CI
```

---

## 5. Commit Message Rules

Commit descriptions should:

- use imperative language
- be short
- describe the actual change
- avoid unnecessary punctuation

Prefer:

```text
feat: add reusable icon button
```

Avoid:

```text
feat: added reusable icon button.
```

Prefer:

```text
fix: handle empty input
```

Avoid:

```text
fixed bug
```

---

## 6. Commit Scope

Optional scopes may be used when useful.

Format:

```text
<type>(<scope>): <description>
```

Examples:

```text
feat(ui): add ink wash color palette
fix(parser): handle escaped quotes
perf(render): reduce temporary allocations
docs(design): document typography rules
refactor(theme): centralize spacing tokens
```

Scopes should be short and meaningful.

Do not create overly specific scopes.

Bad:

```text
fix(src-ui-components-buttons-primary-button): fix hover
```

Good:

```text
fix(ui): correct button hover state
```

---

## 7. Branch Naming

Use lowercase branch names.

Recommended patterns:

```text
feature/<name>
fix/<name>
refactor/<name>
docs/<name>
experiment/<name>
```

Examples:

```text
feature/ink-wash-theme
feature/dark-theme

fix/button-hover
fix/parser-escape

refactor/theme-system

docs/design-guidelines
```

Use hyphens between words.

Avoid:

```text
MyBranch
new_stuff
test123
aaa
```

---

## 8. Working Tree Safety

Before editing:

```bash
git status
```

If unrelated changes already exist:

- preserve them
- do not reset them
- do not overwrite them
- do not include them in your commit

Never run destructive commands automatically.

Avoid commands such as:

```bash
git reset --hard
git clean -fd
git checkout -- .
git restore .
```

unless explicitly requested.

---

## 9. Staging

Stage only files related to the current change.

Prefer:

```bash
git add src/ui/button.slint
git add ui/theme.slint
```

instead of blindly using:

```bash
git add .
```

when unrelated changes exist.

Before committing:

```bash
git diff --cached
```

Review staged changes.

---

## 10. Diff Review

Before finishing a task, inspect:

```bash
git diff
```

and when staged:

```bash
git diff --cached
```

Check for:

- accidental edits
- debug code
- temporary files
- unrelated formatting changes
- generated files
- secrets
- large unintended changes

---

## 11. Do Not Commit Secrets

Never commit:

```text
.env
API keys
access tokens
passwords
private keys
credentials
database passwords
personal secrets
```

Environment values should use files such as:

```text
.env.example
```

Example:

```env
API_KEY=
DATABASE_URL=
```

Do not include real values.

---

## 12. Generated Files

Generated files should generally not be manually modified.

If a file is generated from another source:

```text
source
   ↓
generator
   ↓
generated file
```

modify the source or generator when possible.

Do not commit generated build artifacts unless the project explicitly requires them.

Typical ignored directories include:

```text
target/
build/
dist/
node_modules/
```

---

## 13. Formatting

Before committing code, run the relevant formatter.

Rust:

```bash
cargo fmt
```

Go:

```bash
gofmt
```

JavaScript / TypeScript:

```bash
prettier
```

Use the formatter already configured by the project.

Do not reformat unrelated files.

---

## 14. Tests

Run relevant tests before committing.

Rust:

```bash
cargo test
```

Go:

```bash
go test ./...
```

Only run tests relevant to the project.

If the complete test suite is expensive, run focused tests first.

---

## 15. Rust Checks

For Rust projects, prefer running:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy
```

When appropriate.

Do not introduce new warnings.

---

## 16. Small Commits

Prefer several understandable commits over one huge commit.

Example:

```text
feat(theme): add ink wash color tokens

feat(ui): apply theme tokens to buttons

feat(ui): add themed input component
```

is often better than:

```text
feat: redesign entire application
```

---

## 17. Refactoring

Do not mix large refactoring with feature development unless necessary.

Prefer:

```text
refactor: extract theme system
```

then:

```text
feat: add dark theme
```

This makes review and rollback easier.

---

## 18. Dependency Changes

When adding a dependency:

- verify that it is necessary
- prefer existing dependencies when possible
- avoid adding large libraries for trivial functionality

Dependency updates should be explicit.

Example:

```text
chore: update slint dependency
```

Do not silently update unrelated packages.

---

## 19. Reverts

If a change must be reverted, preserve history.

Prefer:

```bash
git revert <commit>
```

instead of rewriting shared history.

---

## 20. History Rewriting

Do not rewrite published/shared history unless explicitly requested.

Avoid:

```bash
git push --force
```

Prefer:

```bash
git push --force-with-lease
```

only when rewriting history is intentionally required.

---

## 21. Merge Conflicts

When resolving conflicts:

1. Understand both sides.
2. Preserve intended behavior.
3. Do not blindly accept one side.
4. Run tests after resolution.
5. Review the resulting diff.

Do not automatically choose:

```text
ours
```

or:

```text
theirs
```

without understanding the conflict.

---

## 22. Pull Requests

A pull request should describe:

### What

What changed?

### Why

Why was this change necessary?

### How

What approach was used?

### Testing

How was the change verified?

Example:

```text
## What

Adds the initial Modern Ink visual theme.

## Why

The project needs a consistent visual language.

## Changes

- Added paper and ink color tokens
- Added seal-red accent
- Added spacing tokens
- Updated button styling

## Testing

- cargo check
- cargo test
- manually verified light theme
```

---

## 23. Pull Request Size

Prefer small and focused pull requests.

Avoid giant PRs containing:

```text
new architecture
+
UI redesign
+
dependency migration
+
large refactor
+
new features
```

Break them into logical steps.

---

## 24. Agent Rules

When a coding agent works on this repository:

The agent must:

1. Inspect `git status` before editing.
2. Preserve existing user changes.
3. Modify only files relevant to the task.
4. Avoid destructive Git operations.
5. Review the final diff.
6. Run relevant formatting and tests.
7. Follow `AGENTS.md`.
8. Follow [the current UI design rules](docs/reference/DESIGN.md) for visual changes.
9. Follow this document for Git operations.

The agent must not create commits unless explicitly asked to commit.

The agent must not push changes unless explicitly asked to push.

---

## 25. Final Checklist

Before considering work complete:

```text
[ ] git status inspected
[ ] unrelated changes preserved
[ ] diff reviewed
[ ] no debug code remains
[ ] no secrets added
[ ] formatting completed
[ ] relevant tests passed
[ ] commit scope is focused
[ ] commit message is meaningful
[ ] documentation updated if necessary
```

---

## Core Principle

Git history should explain the evolution of the project.

A good commit should answer:

> What changed, and why?

without requiring someone to inspect every modified line.
