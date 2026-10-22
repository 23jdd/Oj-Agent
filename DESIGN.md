# DESIGN.md

## Current UI direction (2026-10-22)

The user's Neobrutalism redesign request supersedes the ink-wash visual guidance
below. The reference is https://github.com/neobrutalism/neobrutalism. Its default
palette and button treatment guide the native Slint components; React is not used.

- Use white surfaces, a light neutral workspace, black 2px borders, yellow
  `#ffdb33` primary actions and short, unblurred 4px offset shadows on buttons.
- Use square corners, bold modern headings and readable Chinese application text.
- Define colors, geometry and spacing in `ui/theme.slint`. Reuse the controls in
  `ui/components.slint`, including keyboard focus, hover, pressed and disabled states.
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
  See `docs/STYLE.md`.
- Verify minimize/restore, live resizing, playback, seeking, input and model
  settings on a native window before considering UI work complete.

## Previous ink-wash reference

The remainder is retained as the previous design reference; conflicting visual
rules no longer apply to the current Neobrutalism UI.

# Ink Wash Visual Style

## 1. Style Definition

The visual style of this project is:

**Modern Ink Wash**

It combines modern minimal interface design with the visual atmosphere of traditional Chinese ink painting.

The design should feel:

- calm
- poetic
- restrained
- natural
- elegant
- quiet
- breathable
- refined

The style should reference Chinese ink painting indirectly through:

- color
- texture
- contrast
- whitespace
- brush rhythm
- natural forms

Do not imitate traditional painting literally.

The final result should still feel contemporary.

---

## 2. Core Principle

The guiding principle is:

> Modern form, ink-wash spirit.

The design should not feel ancient, theatrical, or decorative-heavy.

Traditional influence should appear subtly.

Prefer atmosphere over ornament.

---

## 3. Visual Keywords

Primary keywords:

- ink
- paper
- mist
- stone
- bamboo
- seal
- brush
- silence
- emptiness
- balance
- natural rhythm

Emotional qualities:

- calm
- elegant
- restrained
- warm
- contemplative
- timeless

---

## 4. Color Philosophy

The palette should be based on natural, desaturated tones.

Primary visual colors:

- warm paper white
- ink black
- charcoal gray
- stone gray
- muted brown
- seal red
- bamboo green
- mist blue-gray

Avoid highly saturated colors.

Color should feel slightly aged, organic, and soft.

---

## 5. Base Palette

### Paper

```text
Paper 50   #FCFAF5
Paper 100  #F8F4EB
Paper 200  #F1EBDF
Paper 300  #E7DED0
```

Preferred background:

```text
#F8F4EB
```

---

### Ink

```text
Ink 950   #181715
Ink 900   #252320
Ink 800   #34312D
Ink 700   #4A453F
Ink 600   #625C54
Ink 500   #797168
Ink 400   #978E84
Ink 300   #B6ADA2
Ink 200   #D0C8BE
Ink 100   #E4DED6
```

---

### Seal Red

```text
Seal 700   #7F302E
Seal 600   #923936
Seal 500   #A94440
Seal 400   #BC5A52
Seal 300   #D18A82
```

Seal red should be used sparingly.

It should feel like a traditional red seal.

---

### Bamboo Green

```text
Bamboo 700   #536654
Bamboo 600   #637664
Bamboo 500   #748876
Bamboo 400   #93A395
Bamboo 300   #B4C0B5
```

---

### Mist

```text
Mist 700   #56686A
Mist 600   #65797B
Mist 500   #758B8D
Mist 400   #98A8A9
Mist 300   #B9C5C5
```

---

## 6. Contrast

Contrast should be soft but readable.

Avoid:

- pure black on pure white
- extremely harsh borders
- strong neon contrast

Prefer:

```text
warm paper
+
soft black ink
```

instead of:

```text
#FFFFFF
+
#000000
```

Primary text should remain clearly readable.

---

## 7. Whitespace

Whitespace is a core part of the style.

Whitespace should feel intentional.

Use empty space to create:

- calm
- hierarchy
- rhythm
- focus

Do not fill every empty area with decoration.

In ink painting, empty space is part of the composition.

The same principle applies here.

---

## 8. Composition

Composition should feel asymmetrical but balanced.

Prefer:

- visually weighted empty space
- subtle imbalance
- organic rhythm
- deliberate negative space

Avoid rigid visual symmetry everywhere.

The layout should feel composed rather than mechanically centered.

---

## 9. Typography

Typography should be elegant and understated.

Primary interface typography should remain modern and highly readable.

Suitable directions:

- modern sans-serif
- clean humanist sans-serif
- restrained serif for selected headings

Chinese typography may use:

- Source Han Sans
- Source Han Serif
- Noto Sans CJK
- Noto Serif CJK

---

## 10. Calligraphy

Calligraphy fonts are decorative elements only.

Use them for:

- branding
- short titles
- visual signatures
- artistic labels

Do not use calligraphy fonts for:

- body text
- long paragraphs
- controls
- menus
- data
- small text

Calligraphy should function like artwork, not ordinary typography.

---

## 11. Typography Weight

Prefer:

- Regular
- Medium
- Semibold

Avoid excessive bold typography.

Strong weight should only appear where hierarchy requires it.

The visual tone should feel light rather than heavy.

---

## 12. Borders

Borders should feel subtle.

Prefer:

- thin strokes
- warm gray
- low contrast

Borders should resemble light ink lines rather than strong geometric outlines.

Avoid:

- thick black borders
- heavy boxes
- borders around every element

---

## 13. Corner Radius

Corners should be restrained.

Recommended visual character:

- mostly subtle rounding
- sometimes near-square

Typical radius range:

```text
2px
4px
6px
8px
```

Avoid very large rounded corners.

The style should not resemble a playful mobile UI.

---

## 14. Shapes

Prefer simple geometric shapes softened by subtle organic details.

Suitable shapes:

- rectangle
- rounded rectangle
- circle
- thin line
- irregular brush stroke

Organic shapes should be decorative, not structural.

---

## 15. Shadows

Shadows should be extremely subtle.

Use them only when depth is necessary.

Preferred shadow character:

- soft
- diffuse
- low-opacity
- neutral or warm

Avoid:

- strong floating shadows
- dramatic elevation
- glowing shadows

Ink Wash style should feel mostly flat.

---

## 16. Surface Style

Surfaces should feel similar to:

- paper
- parchment
- stone
- matte ceramic

Avoid surfaces that feel:

- glossy
- plastic
- metallic
- glass-like

The style should feel tactile but understated.

---

## 17. Texture

Texture is optional.

Suitable texture references:

- Xuan paper
- handmade paper
- dry brush
- subtle ink bleed
- stone grain

Texture must remain very low contrast.

Good texture should almost disappear when not actively noticed.

---

## 18. Paper Texture

Paper texture should be:

- soft
- subtle
- irregular
- low frequency

Avoid obvious repeating texture patterns.

Do not use heavy paper fibers behind small text.

When uncertain, use a plain warm background instead.

---

## 19. Ink Texture

Ink texture may appear as:

- soft edges
- subtle bleed
- dry-brush detail
- diluted gray washes

Do not use realistic ink effects on every object.

Ink effects should mainly appear in decorative areas.

---

## 20. Brush Strokes

Brush strokes are accent elements.

Suitable uses:

- short dividers
- heading underline
- visual highlight
- decorative frame fragment
- illustration detail

Avoid using brush strokes everywhere.

A few strokes are stronger than many.

---

## 21. Ink Wash

Ink wash gradients should feel organic.

Suitable transitions:

```text
Ink Gray → Transparent
```

or

```text
Mist Gray → Paper
```

Edges should feel soft and irregular.

Avoid digital-looking linear gradients.

---

## 22. Gradient Rules

Conventional colorful gradients should generally be avoided.

Avoid:

- blue-purple gradient
- orange-pink gradient
- neon gradients
- rainbow gradients

Allowed:

- subtle tonal ink wash
- mist-like opacity fade
- monochrome wash

---

## 23. Decorative Motifs

Suitable motifs include:

- mountains
- mist
- bamboo
- stone
- plum blossom
- water
- clouds
- seal marks

Use motifs sparingly.

They should support atmosphere, not dominate the composition.

---

## 24. Seal Motif

Seal red is one of the signature accents of this style.

Seal shapes may be:

- square
- rectangular
- slightly irregular

Use them for:

- branding
- small labels
- signatures
- key visual accents

Avoid placing seals everywhere.

One seal can establish the style.

---

## 25. Illustration Style

Illustrations should use:

- monochrome ink
- muted colors
- large empty areas
- simplified forms
- subtle brush textures

Avoid:

- glossy 3D renders
- cartoon vector characters
- highly saturated illustrations
- dense anime-style imagery

---

## 26. Icon Style

Icons should be:

- simple
- thin
- geometric
- restrained

Preferred characteristics:

- consistent stroke width
- low visual complexity
- minimal fill

Avoid mixing:

- filled icons
- outline icons
- cartoon icons
- hand-painted icons

unless intentionally designed as one coherent system.

---

## 27. Motion

Motion should feel calm.

Suitable motion qualities:

- slow fade
- gentle reveal
- subtle slide
- opacity transition
- soft expansion

Avoid:

- bounce
- elastic spring
- aggressive zoom
- spinning decoration
- energetic motion

---

## 28. Motion Inspiration

Motion may evoke:

- mist appearing
- ink spreading
- paper unfolding
- water settling

Do not literally simulate these effects unless they remain subtle.

The result should still feel like normal modern UI animation.

---

## 29. Animation Timing

Recommended range:

```text
80–150ms
```

for small interactions.

```text
150–300ms
```

for larger transitions.

Avoid long ornamental animations.

---

## 30. Hover Style

Hover should remain subtle.

Possible changes:

- slight darkening
- slight paper-tone shift
- border strength increase
- opacity increase

Avoid:

- glow
- strong scaling
- dramatic color shift

---

## 31. Selected State

Selected states may use:

- seal red
- deeper ink
- subtle tint
- thin accent line

Selection should remain elegant and clear.

---

## 32. Visual Density

The style should feel breathable.

However, breathable does not mean oversized.

Avoid:

- giant spacing everywhere
- oversized controls
- excessive empty vertical gaps

Use whitespace deliberately.

---

## 33. Layering

Visual hierarchy should primarily come from:

1. whitespace
2. typography
3. tone
4. subtle borders
5. restrained accent color

Avoid depending heavily on shadows.

---

## 34. Photography

If photography is used, prefer:

- desaturated imagery
- natural light
- subdued contrast
- simple composition

Photos may be lightly toned toward:

- warm gray
- brown
- muted green

Avoid vivid commercial photography.

---

## 35. Material Character

The overall visual material should feel:

```text
paper
ink
stone
wood
ceramic
```

rather than:

```text
glass
chrome
plastic
neon
```

---

## 36. Light Theme

The default style is light.

Light theme should resemble:

> Ink placed on warm paper.

Use warm off-white backgrounds.

Avoid cold pure-white interfaces.

---

## 37. Dark Theme

Dark theme should resemble:

> Pale paper and seal colors emerging from deep ink.

Suggested background direction:

```text
#1C1A18
#24211E
#2C2824
```

Text should be warm off-white.

Avoid pure black backgrounds.

---

## 38. Style Balance

The design should roughly feel:

```text
80% modern minimalism
20% traditional ink influence
```

Do not reverse this balance.

The traditional style should be felt, not constantly displayed.

---

## 39. Anti-Patterns

Avoid:

- glassmorphism
- cyberpunk
- neon
- glossy surfaces
- excessive gradients
- giant rounded cards
- overly playful UI
- excessive shadows
- bright SaaS palette
- heavy Chinese ornament
- gold borders everywhere
- traditional pattern on every surface
- calligraphy everywhere
- large ink splashes behind text

---

## 40. Important Rule

Do not confuse:

**Chinese style**

with:

**putting traditional motifs everywhere.**

The style should come primarily from:

- restraint
- color
- whitespace
- rhythm
- material
- typography
- atmosphere

not decoration.

---

## 41. Visual Reference Formula

When creating a new visual element, use this formula:

```text
Modern Minimalism
+
Warm Paper
+
Soft Ink
+
Large Negative Space
+
One Restrained Traditional Detail
```

Do not add multiple decorative ideas at once.

---

## 42. Final Visual Test

Before accepting a design, ask:

### Does it feel calm?

If no, reduce color and decoration.

### Does it feel modern?

If no, remove literal traditional ornament.

### Does it feel Eastern without being theatrical?

If yes, the balance is correct.

### Is the decoration more noticeable than the content?

If yes, reduce the decoration.

### Does every visual element have a purpose?

If no, remove it.

---

## Final Principle

The desired result is not:

> an ancient Chinese interface.

The desired result is:

> a modern interface with the visual temperament of Chinese ink painting.
