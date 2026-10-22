use oj_agent::{
    animation, export,
    ir::ProblemInput,
    renderer::SkiaRenderer,
    style::{Rgb, StyleConfig},
    trace, visualization,
};

#[test]
fn preferences_round_trip_partial_defaults_and_failed_saves_preserve_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("style.json");
    let mut style = StyleConfig::load(&path).unwrap();
    style.fonts.text_size = 16.;
    style.colors.accent = Rgb::parse("#12AbEF").unwrap();
    style.geometry.radius = 4.;
    style.save(&path).unwrap();
    assert_eq!(StyleConfig::load(&path).unwrap(), style);
    style.fonts.text_size = f32::NAN;
    assert!(style.save(&path).is_err());
    assert_eq!(StyleConfig::load(&path).unwrap().fonts.text_size, 16.);
    std::fs::write(&path, r##"{"colors":{"accent":"#123456"}}"##).unwrap();
    let partial = StyleConfig::load(&path).unwrap();
    assert_eq!(partial.colors.accent, Rgb([18, 52, 86]));
    assert_eq!(partial.fonts, StyleConfig::default().fonts);
    for invalid in [
        r##"{"colors":{"accent":"#abc"}}"##,
        r#"{"version":2}"#,
        r#"{"fonts":{"text_size":999}}"#,
        r#"{"fonts":{"ui_family":""}}"#,
        r#"{"colours":{}}"#,
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert!(StyleConfig::load(&path).is_err(), "{invalid}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    }
    assert!(Rgb::parse("#汉字").is_err());
    std::fs::write(&path, vec![b' '; 65537]).unwrap();
    assert!(StyleConfig::load(&path).is_err());
}

#[test]
fn custom_font_sizes_and_colors_reach_preview_png_and_gif() {
    let (algorithm, trace) = trace::simulate(&ProblemInput::default()).unwrap();
    let plan = visualization::plan(&algorithm, &trace).unwrap();
    let animation = animation::compile(&algorithm, &trace, &plan).unwrap();
    let plain = SkiaRenderer
        .prepare(&animation)
        .unwrap()
        .frame(0.4)
        .unwrap();
    let mut style = StyleConfig::default();
    style.fonts.animation_scale = 1.4;
    style.fonts.animation_family = "Arial".into();
    let bigger = SkiaRenderer
        .prepare_styled(&animation, &style)
        .unwrap()
        .frame(0.4)
        .unwrap();
    assert_ne!(plain.rgba, bigger.rgba);
    style.colors.surface = Rgb([18, 34, 56]);
    style.colors.background = Rgb([45, 67, 89]);
    style.colors.text_primary = Rgb([250, 240, 230]);
    style.colors.accent = Rgb([78, 210, 190]);
    style.geometry.radius = 4.;
    let scene = SkiaRenderer.prepare_styled(&animation, &style).unwrap();
    let expected = scene.frame(0.).unwrap();
    assert_eq!(&expected.rgba[..4], &[18, 34, 56, 255]);
    assert_eq!(
        plain.rgba,
        SkiaRenderer
            .prepare(&animation)
            .unwrap()
            .frame(0.4)
            .unwrap()
            .rgba
    );
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("style.png");
    export::png_styled(&animation, &png, 0., &style).unwrap();
    let image = skia_safe::images::deferred_from_encoded_data(
        skia_safe::Data::new_copy(&std::fs::read(png).unwrap()),
        None,
    )
    .unwrap();
    let info = skia_safe::ImageInfo::new(
        (expected.width as i32, expected.height as i32),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Premul,
        None,
    );
    let mut rgba = vec![0; expected.rgba.len()];
    assert!(image.read_pixels(
        &info,
        &mut rgba,
        expected.width as usize * 4,
        (0, 0),
        skia_safe::image::CachingHint::Disallow
    ));
    assert_eq!(rgba, expected.rgba);
    // Keep the palette test short while exercising actual GIF encoding and decoding.
    let mut short = animation.clone();
    short.duration = 0.1;
    short
        .timeline
        .retain(|action| action.at + action.duration <= 0.1);
    short.step_times = vec![0.];
    let gif = dir.path().join("style.gif");
    export::gif_cancellable_styled(
        &short,
        &gif,
        10,
        &std::sync::atomic::AtomicBool::new(false),
        &style,
    )
    .unwrap();
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(std::fs::File::open(gif).unwrap())
        .unwrap();
    let frame = decoder.read_next_frame().unwrap().unwrap();
    assert_eq!(&frame.buffer[..4], &[18, 34, 56, 255]);
    assert!(frame.buffer.as_chunks::<4>().0.contains(&[45, 67, 89, 255]));
}
