fn main() {
    slint_build::compile_with_config(
        "ui/app.slint",
        slint_build::CompilerConfiguration::new().with_debug_info(cfg!(debug_assertions)),
    )
    .expect("compile desktop UI");
}
