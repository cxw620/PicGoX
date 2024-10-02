use slint_build::{compile_with_config, CompilerConfiguration};

fn main() {
    // Build tray icon resources on Windows
    #[cfg(windows)]
    embed_resource::compile("ui/img/tray.rc", embed_resource::NONE);

    // Compile slint
    compile_with_config(
        "ui/main.slint",
        CompilerConfiguration::new().with_style("material".to_string()),
    )
    .unwrap();
}
