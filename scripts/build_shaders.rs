pub fn main() {
    let project_base_path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_base_path = std::path::Path::new(&project_base_path);

    let relative_shader_files = [
        ("shader/texture/texture.vert", "shader/texture/vert.spv"),
        ("shader/texture/texture.frag", "shader/texture/frag.spv"),
    ];

    let absolute_shader_files = relative_shader_files
        .iter()
        .map(|(src, dst)| (project_base_path.join(src), project_base_path.join(dst)))
        .collect::<Vec<_>>();

    for (src, dst) in absolute_shader_files.iter() {
        println!("cargo:rerun-if-changed={}", src.to_str().unwrap());
        std::process::Command::new("glslc")
            .arg(src)
            .arg("-o")
            .arg(dst)
            .status()
            .expect("Failed to compile shader");
    }
}
