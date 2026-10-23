# Skia animation renderer

[文档索引 / Documentation index](README.md)

This renderer applies ideas studied in [23jdd/animation](https://github.com/23jdd/animation/tree/d845bfad88bd9794bf1270542e510ed0557c5795): absolute-time evaluation, renderer-independent object state, cached rendering resources, and a shared preview/export timeline. The primary references are its [architecture](https://github.com/23jdd/animation/blob/d845bfad88bd9794bf1270542e510ed0557c5795/Architecture.md), [timeline](https://github.com/23jdd/animation/blob/d845bfad88bd9794bf1270542e510ed0557c5795/timeline/timeline.go) and [renderer interface](https://github.com/23jdd/animation/blob/d845bfad88bd9794bf1270542e510ed0557c5795/renderer/renderer.go). The implementation here uses Rust, skia-safe and the existing OJ IR.

## Rendering path

`AnimationIr → compiled property tracks → sampled scene → Skia → shared RGBA pixels → Slint`

- `src/animation/timeline.rs` validates once and indexes keyframes. Binary search finds each property's state at an absolute time; backwards seeks produce the same pixels. Visibility and highlights interpolate continuously, including interrupted highlights.
- `src/animation/easing.rs` supplies monotone smoothstep easing. OJ steps transition over 320 ms, then hold; stepping pauses on a completed state.
- `src/renderer.rs` prepares a scene once, caches typefaces, measures text and draws antialiased vector shapes directly into caller-owned pixels. Swaps preserve value identity, including duplicates, and use separate arcs. Text fades out before the next explanation fades in.
- `src/renderer/layout.rs` centers short arrays, wraps long collections, budgets space for node/index labels, lays out explicit tree parent/child edges, follows linked-list `next` chains, uses normalized positions for graphs, and places `rows × cols` grid cells. Pointer labels, edge weights/labels and semantic node states (`active`, `visited`/`done`, `error`, `hidden`, `empty`) are rendered from the same sampled structure track. Legacy complete-binary-tree layouts remain the fallback when a tree has no explicit edges.
- `src/app/preview.rs` owns a render thread, one replaceable pending request and a 32 MiB frame cache. An extra working buffer (at most 32 MiB) rotates with evicted frames so fullscreen does not immediately rewrite the buffer still owned by Slint and trigger a whole-frame copy. Transition frames have distinct keys; settled frames are reused. Seek epochs reject obsolete frames. The UI timer targets 60 updates per second; rasterization runs off the UI thread.
- Completed frames wake the UI event loop immediately instead of waiting for the next playback timer. When several results are ready, only the newest preview is presented. Camera movement keeps in-flight pixels anchored at their original scene coordinates; seek, style, case and resolution changes still reject obsolete generations.

The animation canvas scrolls horizontally and vertically at its natural logical size. Text wraps in full; scene preparation reserves the largest labels, collections and text blocks across all steps, expands their panels and moves later panels down. Node radii and fonts do not shrink to squeeze dense content into the original scene. Explicit hidden states and visibility actions still follow the timeline. Playback, seeking, export and return controls remain outside the canvas scroll area.

`src/renderer/presentation.rs` prepares stable readable geometry; `src/renderer/viewport.rs` defines a camera independent of canvas extent. The worker renders only the visible camera region at monitor scale, with pixel-aligned offsets and a 4096px / 32 MiB frame budget. Exceptionally large physical windows reduce raster density while retaining the full logical viewport. Scrolling replaces camera-cache entries and redraws even when paused; it does not discard every in-flight frame. Resizing retains the last image until geometry is stable for 120 ms. Skia text uses grayscale antialiasing, normal hinting and baseline snapping; Windows prefers Microsoft YaHei UI.

Fullscreen keeps the same native text density while cropping raster allocation to the canvas intersection; empty window area beyond the scene uses the matching UI surface color. Off-screen objects and labels are culled conservatively. `src/renderer/text.rs` reuses fonts, wrapping and measured lines in a cache bounded to 512 entries and 1 MiB of source text. `src/renderer/raster.rs` caches settled graph-node circle/shadow/border sprites at their physical scale and fractional pixel position, with a separate 32 MiB/256-entry LRU limit. Changing highlights remain vector-rendered. DPI, fill color and pixel phase distinguish sprite entries; each prepared scene owns its style-specific caches. Exports retain vector evaluation. The default development profile optimizes the application, `i-slint-core` and `i-slint-renderer-skia`: Slint 1.18.1 copies image data and scans every pixel's alpha while preparing a new Skia image, so leaving that dependency unoptimized makes large fullscreen uploads particularly expensive. Other dependency profiles remain unchanged.

GIF, PNG and video use the same prepared scene evaluator and include the full expanded canvas, fitted into the source resolution (bounded to 1920×1080 for large source canvases). Desktop scrolling keeps normal text and shape sizes; a fixed-size exported image may reduce them. GIF defaults to 30 fps and distributes centisecond rounding across frames. A shared theme palette and color lookup avoid per-frame general-purpose quantization. Export remains streaming and cancellable.

## Verification

```sh
cargo test --test rendering
cargo run --example render_bench
cargo run --example preview_bench
```

The benchmark warms fonts and samples 180 absolute times at 960×720 for Two Sum, Bubble Sort, BFS and Fibonacci DP. It includes frame allocation and CPU rasterization, and excludes GPU upload, window presentation and display refresh. Its output measures local draw time, not a guaranteed desktop frame rate.

`preview_bench` samples native-scale simple and 128-node graph scenes in 960×600, 2560×1440 and 3840×2160 windows. It reports the actual cropped raster size and CPU median/p95 including allocation. Pass `-- --uncropped` to measure the full viewport buffer. These measurements do not include texture upload or window presentation and cannot establish a display frame rate.

Tests cover intermediate frames, repeatable seeks, duplicate-value swaps, interrupted highlights, zero-duration actions, layout bounds, matching camera/cropped native pixels at multiple DPI scales, paused scrolling, dense/deep structures, complete long text, large logical canvases, cache invalidation and GIF timing. Text sizes use the configured animation scale and minimum size in logical pixels. `cargo run --example render_hidpi` writes compact and 150% resolution PNGs under `target/font-review/` for visual inspection. `cargo run --example chat_fixture -- --large` creates an isolated dense-graph and long-text profile at `target/animation-scroll-review/profile`; launch the desktop with `OJ_DATA_DIR` set to that directory for native review.
