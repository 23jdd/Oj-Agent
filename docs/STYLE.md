# Appearance configuration / 外观配置

Open **外观** in the desktop header. Start with a **经典黄 / 薄荷绿 / 浅紫** palette, choose an installed font from the dropdown or click **本地文件…**, and adjust sizes with **− / +**. Color fields offer clickable swatches and custom `#RRGGBB` input. The sample previews the draft immediately. Click **保存并应用** to update the UI, paused or playing previews, and subsequent GIF exports without restarting. **取消** discards edits; **恢复默认** resets the draft and takes effect only after saving. Palette presets change colors only, preserving fonts and dimensions.

在顶部打开“外观”，可一键选择配色、下拉选择系统字体、点击“本地文件…”导入字体，或用加减按钮调整字号。下方示例会即时预览，点击“保存并应用”后才会改变实际界面。取消保留原外观；恢复默认后仍需保存。预览和 GIF 导出共用相同配置。

Local **TTF / OTF / TTC** fonts (up to 32 MiB each; TTC uses the first face for animation) need no system installation. UI, code, and animation can use separate files. Saving copies each font into a `fonts/` directory beside `style.json`, using a content hash to avoid duplicate copies. The original file can then be moved. Fonts register locally in Slint, and Skia loads the selected animation file directly for previews and exports. **改用系统** clears the file selection; choose an installed family from the dropdown. Choose a font with Chinese glyphs when displaying Chinese animation text. Invalid or missing files produce an error; the saved configuration is retained. On startup, an unusable local font causes the application to use default appearance and display the error.

本地字体不需要安装到系统。保存时会复制到配置文件旁的 `fonts/` 目录，因此原文件移走后仍能使用。界面、代码和动画可分别设置字体；点击“改用系统”并选择系统字体即可移除本地字体选择。用于中文动画时，请选择包含中文字形的字体。

The dialog displays the exact `style.json` path. Windows normally uses `%LOCALAPPDATA%\oj-agent\OjAgent\data\style.json`; Linux uses the application data directory under `$XDG_DATA_HOME` or `~/.local/share`. The application starts with defaults when the file is absent. Invalid files are preserved, an error is shown, and defaults are used until a valid configuration is saved. External file edits load on the next launch. Saving uses atomic replacement.

Use [the full example](../config/style.example.json) as a starting point. Fields may be omitted to use defaults. Unknown keys, unsupported versions, malformed colors, non-finite numbers, and out-of-range dimensions are rejected. The example uses Windows font names; Linux defaults to Noto Sans CJK SC and monospace.

| Group | Settings | Range |
| --- | --- | --- |
| Fonts | UI, code and animation font families; optional `ui_file`, `code_file`, `animation_file` | Installed family or local TTF / OTF / TTC file |
| UI text | Body, labels, headings | 12–18 / 12–16 / 18–28 logical px |
| Animation text | Size multiplier, minimum pixel size, line height | 0.8–1.6 / 12–18 physical px / 1.2–1.8 |
| Colors | Background, surface, text, borders, highlights, hover, pressed, focus, disabled, success, error | `#RRGGBB` |
| Geometry | Border width, hard-shadow offset, corner radius, control height | 1–3 / 0–6 / 0–8 / 36–48 logical px |

The same semantic colors drive Slint and Skia. Animation fills use `background`, the canvas uses `surface`, and active objects use `accent`. Animation borders and shadows use `border`. Geometry changes affect shared controls and animation tiles; circular graph nodes retain their shape. Dense numeric values still fit their allocated cells, and long animation text wraps or truncates to preserve layout. Choose text and background colors with sufficient contrast; the default style remains Neobrutalism.

Set `OJ_STYLE_CONFIG` to use another desktop preferences file, for example a portable or test configuration. The path shown in the editor reflects this override.

For manual JSON edits and CLI rendering, font file paths may be absolute or relative to the configuration file. A non-empty file path takes precedence over the corresponding family name. For example, `"animation_file": "fonts/MyFont.ttf"` uses a local animation font; empty file strings use family names. Existing configurations without these fields continue to work.

CLI rendering accepts an explicit configuration; omitting it uses built-in defaults:

```sh
cargo run -- render output/animation-v1.json output/styled.png --style config/style.example.json --time 0.4
cargo run -- render output/animation-v1.json output/styled.gif --style config/style.example.json
```

Preferences are presentation settings, stored separately from session/animation IR. Applying a style invalidates preview pixels, rejects old in-flight frames and reuses the animation timeline. GIF creates its palette from the selected colors once per export, preserving flat theme colors exactly.
