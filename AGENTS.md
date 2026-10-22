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
- anyhow: application-level error handling; follow `EXCEPTON.md` for error classification and recovery.

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

All UI implementation and modification MUST follow `DESIGN.md`.

The user's 2026-10-22 Neobrutalism redesign request replaces the ink-wash visual
direction below. Follow the current-direction section at the top of `DESIGN.md`
and https://github.com/neobrutalism/neobrutalism for the active UI. Continue to
apply the reusable-component, layout, accessibility and review rules below.

The project uses a **modern Chinese ink-wash inspired design language**.

The design is not intended to imitate an ancient Chinese interface literally.

The goal is:

> Modern desktop application structure with the quiet, restrained, natural feeling of Chinese ink painting.

Usability and readability always take priority over decoration.

---

### Core UI Principles

Every UI change should be:

- calm
- restrained
- readable
- consistent
- spacious
- functional
- desktop-oriented

Prefer subtle traditional Chinese visual cues instead of obvious decorative styling.

The interface should feel like:

- Xuan paper
- ink
- mist
- brush rhythm
- stone
- bamboo
- seal red

without becoming an illustration or poster.

---

### Before Editing UI

Before creating or modifying UI:

1. Read `DESIGN.md`.
2. Inspect nearby existing UI components.
3. Reuse existing components whenever possible.
4. Reuse existing design tokens.
5. Preserve the existing visual hierarchy.
6. Do not redesign unrelated UI.

Do not introduce a new visual pattern for a single feature.

If a new pattern is necessary, make it reusable.

---

### Slint Component Rules

Prefer reusable Slint components.

Examples:

- `AppButton`
- `IconButton`
- `ToolButton`
- `TextField`
- `NumberField`
- `CheckBox`
- `Panel`
- `Section`
- `Sidebar`
- `Toolbar`
- `StatusBar`
- `Dialog`
- `Tab`
- `PropertyRow`
- `Divider`

Do not repeatedly implement identical styling in different files.

Bad:

```slint
Rectangle {
    background: #f5f0e8;
    border-color: #d4ccc0;
    border-width: 1px;
    border-radius: 6px;
}
```

copied into many components.

Prefer reusable components or theme tokens.

---

### Design Tokens

Colors, spacing, control sizes, radii, and typography SHOULD come from centralized design tokens.

Prefer:

```slint
global Theme {
    // colors
    in property <color> background;
    in property <color> surface;
    in property <color> text-primary;
    in property <color> text-secondary;
    in property <color> border;
    in property <color> accent;

    // spacing
    in property <length> spacing-xs;
    in property <length> spacing-sm;
    in property <length> spacing-md;
    in property <length> spacing-lg;

    // geometry
    in property <length> radius-sm;
    in property <length> radius-md;
    in property <length> control-height;
}
```

Do not hard-code arbitrary visual values throughout the project.

---

### Layout Rules

Use Slint layout components whenever possible:

- `VerticalLayout`
- `HorizontalLayout`
- `GridLayout`

Avoid absolute positioning for normal UI.

Absolute positioning is acceptable for:

- canvas overlays
- CAD overlays
- floating tool controls
- drag handles
- visual annotations
- coordinate-based content

Normal forms and panels should use layouts.

---

### Spacing

Use the project spacing scale.

Preferred values:

- 4px
- 8px
- 12px
- 16px
- 24px
- 32px

Do not casually introduce values such as:

- 7px
- 13px
- 19px
- 27px

unless required for precise visual alignment.

---

### Interaction States

Interactive components should define appropriate states.

When applicable, include:

- normal
- hover
- pressed
- selected
- focused
- disabled

Do not rely on color alone when the interaction state would otherwise be unclear.

Active tools and selected items must be immediately recognizable.

---

### Animation

Animations must be subtle and functional.

Prefer short transitions.

Typical durations:

- hover: 100–140ms
- pressed: 80–120ms
- selection: 120–160ms
- panel transition: 160–220ms
- dialog transition: 160–220ms

Avoid:

- bounce
- elastic motion
- strong scaling
- continuous decorative animation
- unnecessary movement

For precision-oriented interfaces such as CAD or editors, avoid animation that can interfere with pointer accuracy.

---

### Typography

Normal application text must prioritize readability.

Do not use decorative calligraphy fonts for:

- body text
- forms
- menus
- toolbars
- property panels
- numeric values

Calligraphy-inspired typography may only be used sparingly for:

- welcome screens
- decorative headings
- branding

UI text should remain modern and readable.

---

### Color Rules

Use semantic theme colors.

Do not scatter raw color literals throughout UI files.

Prefer tokens such as:

- `background`
- `surface`
- `surface-hover`
- `surface-active`
- `border`
- `border-strong`
- `text-primary`
- `text-secondary`
- `text-disabled`
- `accent`
- `accent-hover`
- `danger`
- `warning`
- `success`

Accent colors should be used sparingly.

Seal red must not dominate the application.

---

### Water-Ink Decoration

Ink-wash decoration is optional and must remain subtle.

Allowed:

- faint paper texture
- subtle ink gradient in large empty areas
- small seal-red mark
- restrained brush-line divider
- low-opacity landscape silhouette
- soft mist-like decorative background

Do not place decoration behind dense readable content.

Decorative elements must never reduce:

- contrast
- readability
- pointer precision
- information density

---

### Desktop UI Rules

This is a desktop application.

Prefer:

- compact controls
- structured panels
- clear toolbars
- visible state
- medium-to-high information density
- keyboard-friendly interaction

Avoid mobile-oriented styling such as:

- huge touch targets
- giant rounded cards
- oversized headings
- excessive vertical spacing
- pill-shaped everything

---

### Tool / Editor Interfaces

For tool-oriented screens:

- Main content or canvas receives the most visual space.
- Side panels remain compact.
- Toolbars should be visually quiet.
- Selected tools must be obvious.
- Numeric values should align consistently.
- Properties should be grouped logically.
- Do not overuse cards.

Prefer flat panel hierarchy over dashboard-style card grids.

---

### Dialogs

Dialogs should:

- have a clear title
- contain only relevant content
- use consistent internal spacing
- place actions consistently
- clearly distinguish primary and secondary actions

Avoid oversized dialogs.

Avoid decorative backgrounds inside ordinary dialogs.

---

### Icons

Use a consistent icon family.

Prefer simple:

- outline icons
- geometric icons
- restrained stroke icons

Do not use:

- emoji as application icons
- random mixed icon styles
- cartoon icons
- highly ornamental Chinese symbols for ordinary actions

Traditional motifs may be used only where semantically appropriate.

---

### Forbidden Styles

Do NOT introduce the following unless explicitly requested:

- glassmorphism
- neon UI
- cyberpunk styling
- heavy gradients
- strong glow
- excessive shadows
- large pill-shaped controls
- giant SaaS-style cards
- colorful dashboard blocks
- overly playful illustrations
- dense ornamental Chinese borders
- large ink splashes behind content

---

### Do Not Over-Theme

The application is not a poster.

The application is not a traditional painting viewer.

The application is not a historical-game UI.

Ink-wash aesthetics should influence:

- color
- rhythm
- texture
- whitespace
- contrast
- small decorative details

not replace modern interaction patterns.

---

### Accessibility

Maintain sufficient contrast.

Do not use very light gray text on paper-colored backgrounds for important information.

Focused controls must remain identifiable.

Disabled states must remain distinguishable.

Important information must not be communicated through color alone.

---

### Final Review

Before completing a UI task, verify:

- UI follows `DESIGN.md`.
- Existing components were reused where possible.
- No unnecessary new visual pattern was introduced.
- Spacing follows the design scale.
- Colors use semantic theme values.
- Typography remains readable.
- Hover states are present where needed.
- Pressed states are present where needed.
- Selected states are clear.
- Disabled states are clear.
- Alignment is consistent.
- Decorative ink elements remain subtle.
- Main functionality remains visually dominant.
- The result looks like part of the same application.

If the new UI looks more decorative than functional, simplify it.

## Project Documentation

- Git workflow: [GIT.md](GIT.md). All Git operations must follow this document.
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md).
- Error handling and recovery: [EXCEPTON.md](EXCEPTON.md).
- Intermediate representations: [IR.md](IR.md).
- Tools: [TOOL.md](TOOL.md).
- Sessions: [SESSIONS.md](SESSIONS.md).
- UI design: [DESIGN.md](DESIGN.md).

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
