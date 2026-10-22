use oj_agent::{
    animation::{self, timeline::Timeline},
    ir::*,
    renderer::{RasterSize, Renderer, SkiaRenderer, layout},
    trace, visualization,
};

fn fixture(algorithm: Algorithm, values: Vec<i64>, target: i64) -> AnimationIr {
    let input = ProblemInput {
        algorithm,
        values,
        target,
        ..Default::default()
    };
    let (a, t) = trace::simulate(&input).unwrap();
    let v = visualization::plan(&a, &t).unwrap();
    animation::compile(&a, &t, &v).unwrap()
}

#[test]
fn preview_raster_matches_monitor_pixels_and_has_bounded_memory() {
    for (dpi, expected) in [(1.25, (1000, 750)), (1.5, (1200, 900)), (2., (1600, 1200))] {
        let size = RasterSize::fit((960, 720), (800., 600.), dpi).unwrap();
        assert_eq!((size.width, size.height), expected);
    }
    assert!(RasterSize::fit((960, 720), (0., 600.), 1.).is_none());
    assert!(RasterSize::fit((960, 720), (f32::NAN, 600.), 1.).is_none());
    assert!(RasterSize::fit((0, 720), (800., 600.), 1.).is_none());
    let size = RasterSize::fit((960, 720), (100_000., 100_000.), 3.).unwrap();
    assert!(size.width <= 4096 && size.height <= 4096);
    assert!(u64::from(size.width) * u64::from(size.height) * 4 <= 32 * 1024 * 1024);
    let ir = fixture(Algorithm::TwoSum, vec![2, 7, 11, 15], 9);
    let scene = SkiaRenderer.prepare(&ir).unwrap();
    let size = RasterSize::new(1440, 1080).unwrap();
    let first = scene.frame_at_size(0.24, size).unwrap();
    assert_eq!((first.width, first.height), (1440, 1080));
    scene.frame_at_size(1., size).unwrap();
    assert_eq!(first.rgba, scene.frame_at_size(0.24, size).unwrap().rgba);
    assert!(
        scene
            .frame_at_size(
                0.,
                RasterSize {
                    width: 0,
                    height: 720
                }
            )
            .is_err()
    );
}

#[test]
fn duration_produces_continuous_frames_and_stable_holds() {
    let ir = fixture(Algorithm::TwoSum, vec![2, 7, 11, 15], 9);
    let scene = SkiaRenderer.prepare(&ir).unwrap();
    assert_ne!(scene.frame_key(0.08), scene.frame_key(0.24));
    assert_eq!(scene.frame_key(0.4), scene.frame_key(0.6));
    let first = scene.frame(0.08).unwrap();
    let middle = scene.frame(0.24).unwrap();
    assert_ne!(first.rgba, middle.rgba);
    let rewind = scene.frame(0.08).unwrap();
    assert_eq!(first.rgba, rewind.rgba);
    assert!(scene.frame(f64::NAN).is_err());
}

#[test]
fn swap_tracks_preserve_duplicate_identity_and_settle_on_steps() {
    let ir = fixture(Algorithm::BubbleSort, vec![2, 1, 2], 0);
    let timeline = Timeline::new(&ir).unwrap();
    let action = ir
        .timeline
        .iter()
        .find(|a| a.action == Action::UpdateValue && a.values == [1, 2, 2])
        .unwrap();
    let track = timeline
        .objects
        .iter()
        .find(|t| t.object.id == "values_view")
        .unwrap();
    let midway = track.sample(action.at + action.duration / 2.);
    assert_eq!(midway.origins.unwrap(), [Some(1), Some(0), Some(2)]);
    assert!((midway.value_progress - 0.5).abs() < 0.0001);
    let cells = layout::cells(midway.object, midway.values.len());
    let a = layout::motion(cells[0], cells[1], 0.5);
    let b = layout::motion(cells[1], cells[0], 0.5);
    assert!(
        a.y + a.height < b.y,
        "swapping tiles must remain separately readable"
    );
    let step = ir
        .step_times
        .iter()
        .copied()
        .find(|t| *t >= action.at + action.duration - 0.00001)
        .unwrap();
    assert_eq!(track.sample(step).value_progress, 1.);
}

#[test]
fn interrupted_highlights_are_continuous_and_zero_duration_is_immediate() {
    let mut ir = fixture(Algorithm::TwoSum, vec![2, 7], 9);
    ir.timeline = vec![
        AnimationAction {
            at: 0.,
            duration: 1.,
            action: Action::Highlight,
            target: "values_view".into(),
            indices: vec![0],
            text: String::new(),
            values: vec![],
            structure: None,
        },
        AnimationAction {
            at: 0.5,
            duration: 0.5,
            action: Action::Highlight,
            target: "values_view".into(),
            indices: vec![1],
            text: String::new(),
            values: vec![],
            structure: None,
        },
        AnimationAction {
            at: 1.5,
            duration: 0.,
            action: Action::Hide,
            target: "values_view".into(),
            indices: vec![],
            text: String::new(),
            values: vec![],
            structure: None,
        },
    ];
    let timeline = Timeline::new(&ir).unwrap();
    let track = timeline
        .objects
        .iter()
        .find(|t| t.object.id == "values_view")
        .unwrap();
    let before = track.sample(0.5 - 0.000001).highlights[0];
    let after = track.sample(0.5).highlights[0];
    assert!((before - after).abs() < 0.00001);
    assert_eq!(track.sample(1.).highlights[..2], [0., 1.]);
    assert_eq!(track.sample(1.5).opacity, 0.);
}

#[test]
fn layouts_keep_all_values_indices_and_nodes_inside_their_region() {
    let ir = fixture(Algorithm::ArrayTraversal, (0..64).collect(), 0);
    let mut object = ir
        .scene
        .objects
        .iter()
        .find(|o| o.id == "values_view")
        .unwrap()
        .clone();
    for kind in ["array", "table", "stack", "queue"] {
        object.kind = kind.into();
        for count in [1, 4, 12, 41, 64] {
            let cells = layout::cells(&object, count);
            assert_eq!(cells.len(), count);
            for cell in cells {
                assert!(cell.x >= object.x && cell.y >= object.y);
                assert!(cell.x + cell.width + 4. <= object.x + object.width);
                assert!(
                    cell.y + cell.height + 22. <= object.y + object.height + 0.001,
                    "{kind}/{count}"
                );
            }
        }
    }
    for kind in ["tree", "graph"] {
        object.kind = kind.into();
        for count in [1, 3, 16, 33, 64] {
            let points = layout::graph_points(&object, count);
            for (x, y) in &points {
                assert!(*x >= object.x + 25. && *x <= object.x + object.width - 25.);
                assert!(*y >= object.y + 25. && *y <= object.y + object.height - 25.);
            }
            for (i, point) in points.iter().enumerate() {
                assert!(!points.iter().skip(i + 1).any(|p| p == point));
            }
        }
    }
}

#[test]
fn prepared_preview_and_export_render_the_same_frame() {
    let ir = fixture(Algorithm::Bfs, vec![4, 2, 6, 1, 3, 5, 7], 0);
    let scene = SkiaRenderer.prepare(&ir).unwrap();
    for time in [0., 0.16, 1.23, ir.duration] {
        assert_eq!(
            scene.frame(time).unwrap().rgba,
            SkiaRenderer.frame(&ir, time).unwrap().rgba
        );
    }
}

#[test]
fn gif_rounding_preserves_total_duration_at_30_fps() {
    let ir = fixture(Algorithm::ArrayTraversal, vec![1], 0);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("timing.gif");
    oj_agent::export::gif(&ir, &path, 30).unwrap();
    let mut reader = gif::DecodeOptions::new()
        .read_info(std::fs::File::open(path).unwrap())
        .unwrap();
    let mut duration = 0u64;
    while let Some(frame) = reader.read_next_frame().unwrap() {
        duration += frame.delay as u64;
    }
    assert!((duration as f64 / 100. - ir.duration).abs() <= 0.011);
}
