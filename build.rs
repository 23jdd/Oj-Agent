fn main() {
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_debug_info(cfg!(debug_assertions)),
    )
    .expect("compile desktop UI");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_icon();
    }
}

fn embed_windows_icon() {
    const SOURCE: &str = "ui/icons/app-icon.png";
    println!("cargo:rerun-if-changed={SOURCE}");
    let png = std::fs::read(SOURCE).expect("read application icon");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "app icon must be PNG");
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert!((1..=256).contains(&width) && (1..=256).contains(&height));

    // ICO supports a PNG payload. Keep one source asset shared with the Slint
    // window; Windows scales its 256px image for Explorer and shortcut icons.
    // A zero width/height byte represents 256 pixels in the ICO directory.
    let mut ico = vec![
        0,
        0,
        1,
        0,
        1,
        0,
        width as u8,
        height as u8,
        0,
        0,
        1,
        0,
        32,
        0,
    ];
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22_u32.to_le_bytes());
    ico.extend_from_slice(&png);
    let path =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"))
            .join("app-icon.ico");
    std::fs::write(&path, ico).expect("write Windows application icon");
    winresource::WindowsResource::new()
        .set_icon(path.to_str().expect("Windows icon resource path"))
        .set("ProductName", "OJ Agent")
        .set("OriginalFilename", "oj-agent.exe")
        .compile_for("oj-agent")
        .expect("embed Windows application icon");
}
