# Skia animation renderer

This renderer applies ideas studied in [23jdd/animation](https://github.com/23jdd/animation/tree/d845bfad88bd9794bf1270542e510ed0557c5795): absolute-time evaluation, renderer-independent object state, cached rendering resources, and a shared preview/export timeline. The primary references are its [architecture](https://github.com/23jdd/animation/blob/d845bfad88bd9794bf1270542e510ed0557c5795/Architecture.md), [timeline](https://github.com/23jdd/animation/blob/d845bfad88bd9794bf1270542e510ed0557c5795/timeline/timeline.go) and [renderer interface](https://github.com/23jdd/animation/blob/d845bfad88bd9794bf1270542e510ed0557c5795/renderer/renderer.go). The implementation here uses Rust, skia-safe and the existing OJ IR.

## Rendering path

`AnimationIr → compiled property tracks → sampled scene → Skia → shared RGBA pixels → Slint`

- `src/animation/timeline.rs` validates once and indexes keyframes. Binary search finds each property's state at an absolute time; backwards seeks produce the same pixels. Visibility and highlights interpolate continuously, including interrupted highlights.
- `src/animation/easing.rs` supplies monotone smoothstep easing. OJ steps transition over 320 ms, then hold; stepping pauses on a completed state.
- `src/renderer.rs` prepares a scene once, caches typefaces, measures text and draws antialiased vector shapes directly into caller-owned pixels. Swaps preserve value identity, including duplicates, and use separate arcs. Text fades out before the next explanation fades in.
- `src/renderer/layout.rs` centers short arrays, wraps long collections, budgets space for node/index labels, lays out explicit tree parent/child edges, follows linked-list `next` chains, uses normalized positions for graphs, and places `rows × cols` grid cells. Pointer labels, edge weights/labels and semantic node states (`active`, `visited`/`done`, `error`, `hidden`, `empty`) are rendered from the same sampled structure track. Legacy complete-binary-tree layouts remain the fallback when a tree has no explicit edges.
- `src/app/preview.rs` owns a render thread, one replaceable pending request and a 32 MiB frame cache. Transition frames have distinct keys; settled frames are reused. Seek epochs reject obsolete frames. The UI timer targets 60 updates per second; rasterization runs off the UI thread.

Preview raster size follows the viewport's physical pixels (logical size × monitor scale), with a 4096px / 32 MiB frame limit. The image is presented at native pixel size with pixel-aligned placement. Resizing temporarily retains the image; after 120 ms of stable geometry, the worker redraws at the new resolution, including while paused. Resolution changes invalidate the frame cache and presentation epoch. Skia text uses grayscale antialiasing, normal hinting and baseline snapping; Windows prefers Microsoft YaHei UI.

GIF, PNG and video use the same prepared scene evaluator. GIF defaults to 30 fps and distributes centisecond rounding across frames. A shared theme palette and color lookup avoid per-frame general-purpose quantization. Export remains streaming and cancellable.

## Verification

```sh
cargo test --test rendering
cargo run --example render_bench
```

The benchmark warms fonts and samples 180 absolute times at 960×720 for Two Sum, Bubble Sort, BFS and Fibonacci DP. It includes frame allocation and CPU rasterization, and excludes GPU upload, window presentation and display refresh. Its output measures local draw time, not a guaranteed desktop frame rate.

Tests cover intermediate frames, repeatable seeks, duplicate-value swaps, interrupted highlights, zero-duration actions, layout bounds, matching preview/export pixels, DPI sizing, resolution cache invalidation and GIF timing. Small preview text uses a 13 physical-pixel floor (12 for centered labels); dense values still fit their tile bounds. `cargo run --example render_hidpi` writes compact and 150% resolution PNGs under `target/font-review/` for visual inspection.
