use slint_build::{CompilerConfiguration, EmbedResourcesKind};

fn main() {
    // Embed fonts + icons into the binary. The default references the build
    // machine's absolute paths, which breaks when the binary runs elsewhere.
    let cfg = CompilerConfiguration::new().embed_resources(EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/app.slint", cfg).expect("slint build failed");
}
