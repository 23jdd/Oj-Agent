# UI design / 界面设计

[文档索引 / Documentation index](../README.md)

## Current UI direction (2026-10-22)

The current application follows Neobrutalism. Previous ink-wash guidance is
archived and does not override these rules. The reference is
https://github.com/neobrutalism/neobrutalism. Its default palette and button
treatment guide the native Slint components; React is not used.

- Use white surfaces, a light neutral workspace, black 2px borders, yellow
  `#ffdb33` primary actions and short, unblurred 4px offset shadows on buttons.
- Use square corners, bold modern headings and readable Chinese application text.
- Define colors, geometry and spacing in [ui/theme.slint](../../ui/theme.slint). Reuse the controls in
  [ui/components.slint](../../ui/components.slint), including keyboard focus, hover, pressed and disabled states.
- The primary workspace is a conversation with Markdown messages, a composer and
  `@` file attachments. Do not show algorithm-type selectors or sample forms.
  AI decides whether to produce an animation. Registered animation links navigate
  to an application-local page with case selection and playback, then back to chat.
  No browser, HTTP link or web server is involved in animation navigation.
  History can be collapsed; settings and message content scroll at compact sizes.
- Conversation navigation and new conversations remain enabled during AI replies.
  Busy controls, cancellation, drafts, attachments, errors and worker events belong
  to individual conversations. Switching conversations does not stop background
  replies; show their running state in history without changing the visible chat.
- Show the selected conversation's saved token totals in a wrapping footer using
  semantic typography and colors. Keep the composer and history actions visible
  at compact window sizes; background usage must not replace the selected totals.
- Error banners share a scrollable notice with a keyboard-accessible clear action.
  Dismissing a conversation's notice updates its view state without cancelling
  jobs or deleting persisted errors. Model and appearance editors use the same notice.
- Model settings must not shrink or clip the main layout. Avoid large decorative
  graphics, gradients, glow and continuous UI animations.
  The model editor scrolls its connection and request-policy fields, keeping
  validation errors and Save/Cancel visible. Edits remain drafts until saved;
  changing settings does not alter requests already in progress.
- The desktop window is frameless. Its header uses Slint's native `WindowMoveArea`,
  double-click to maximize/restore, and reusable icon controls for minimize,
  maximize/restore and close. Keep the 8px native resize region clear of controls.
  Model settings leave the window controls accessible.
- Keep the Skia animation palette consistent with the UI. Slint uses its Skia
  window renderer. Preview frames are painted by skia-safe on a worker directly
  into shared premultiplied RGBA storage, cached by timeline state, and displayed
  through Slint's image interface.
  Render the preview at the viewport's physical pixel size and present it without
  upscaling. Debounce resize redraws by 120ms and refresh paused frames too.
  Use the shared font-family token and keep small labels readable.
- The **外观** editor configures shared font, color and geometry tokens. Keep
  editable values in a draft until validated and saved; retain the default
  Neobrutalism appearance. Offer coordinated palette presets, installed-font
  dropdowns, local font-file import, bounded numeric steppers and clickable color
  swatches. Show a draft sample without restyling the editor before save.
  See [appearance settings](../STYLE.md).
- Verify minimize/restore, live resizing, playback, seeking, input and model
  settings on a native window before considering UI work complete.

## Shared implementation rules

These reusable rules were consolidated from AGENTS. Read them before editing UI.

### Core UI principles

Prioritize readability, consistency, clear hierarchy, desktop usability and functional space.
Use the active theme and shared controls; decoration must not compete with the main task.

### Before Editing UI

Before creating or modifying UI:

1. Read `docs/reference/DESIGN.md`.
2. Inspect nearby existing UI components.
3. Reuse existing components whenever possible.
4. Reuse existing design tokens.
5. Preserve the existing visual hierarchy.
6. Do not redesign unrelated UI.

Do not introduce a new visual pattern for a single feature.

If a new pattern is necessary, make it reusable.

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

### Dialogs

Dialogs should:

- have a clear title
- contain only relevant content
- use consistent internal spacing
- place actions consistently
- clearly distinguish primary and secondary actions

Avoid oversized dialogs.

Avoid decorative backgrounds inside ordinary dialogs.

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

### Accessibility

Maintain sufficient contrast.

Do not use very light gray text on paper-colored backgrounds for important information.

Focused controls must remain identifiable.

Disabled states must remain distinguishable.

Important information must not be communicated through color alone.

### Final Review

Before completing a UI task, verify:

- UI follows `docs/reference/DESIGN.md`.
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
- Decoration remains functional and follows the active Neobrutalism direction.
- Main functionality remains visually dominant.
- The result looks like part of the same application.

If the new UI looks more decorative than functional, simplify it.

## Historical reference

[Archived ink-wash design and contributor notes](../archive/INK_WASH_DESIGN.md) are retained for history only.
