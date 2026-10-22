# Appearance configuration / 外观配置

Open **外观** in the desktop header. Choose **字体 / 颜色 / 尺寸**, edit the fields, then **保存并应用**. Changes update the UI, paused or playing previews, and subsequent GIF exports without restarting. **取消** discards edits; **恢复默认** resets the draft and takes effect only after saving. Color fields show a swatch and accept `#RRGGBB`.

在顶部打开“外观”，修改字体、颜色或尺寸后点击“保存并应用”，立即生效并保存到本机。取消不会改变当前外观；恢复默认后仍需保存。预览和 GIF 导出共用相同配置。修改字体名称前，请先在系统中安装该字体；缺失字体会回退到系统可用字体。

The dialog displays the exact `style.json` path. Windows normally uses `%LOCALAPPDATA%\oj-agent\OjAgent\data\style.json`; Linux uses the application data directory under `$XDG_DATA_HOME` or `~/.local/share`. The application starts with defaults when the file is absent. Invalid files are preserved, an error is shown, and defaults are used until a valid configuration is saved. External file edits load on the next launch. Saving uses atomic replacement.

Use [the full example](../config/style.example.json) as a starting point. Fields may be omitted to use defaults. Unknown keys, unsupported versions, malformed colors, non-finite numbers, and out-of-range dimensions are rejected. The example uses Windows font names; Linux defaults to Noto Sans CJK SC and monospace.

| Group | Settings | Range |
| --- | --- | --- |
| Fonts | UI, code and animation font families | Installed family name |
| UI text | Body, labels, headings | 12–18 / 12–16 / 18–28 logical px |
| Animation text | Size multiplier, minimum pixel size, line height | 0.8–1.6 / 12–18 physical px / 1.2–1.8 |
| Colors | Background, surface, text, borders, highlights, hover, pressed, focus, disabled, success, error | `#RRGGBB` |
| Geometry | Border width, hard-shadow offset, corner radius, control height | 1–3 / 0–6 / 0–8 / 36–48 logical px |

The same semantic colors drive Slint and Skia. Animation fills use `background`, the canvas uses `surface`, and active objects use `accent`. Animation borders and shadows use `border`. Geometry changes affect shared controls and animation tiles; circular graph nodes retain their shape. Dense numeric values still fit their allocated cells, and long animation text wraps or truncates to preserve layout. Choose text and background colors with sufficient contrast; the default style remains Neobrutalism.

Set `OJ_STYLE_CONFIG` to use another desktop preferences file, for example a portable or test configuration. The path shown in the editor reflects this override.

CLI rendering accepts an explicit configuration; omitting it uses built-in defaults:

```sh
cargo run -- render output/animation-v1.json output/styled.png --style config/style.example.json --time 0.4
cargo run -- render output/animation-v1.json output/styled.gif --style config/style.example.json
```

Preferences are presentation settings, stored separately from session/animation IR. Applying a style invalidates preview pixels, rejects old in-flight frames and reuses the animation timeline. GIF creates its palette from the selected colors once per export, preserving flat theme colors exactly.
