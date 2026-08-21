//! Compile-time WGSL validation: catches shader syntax/type errors in CI
//! instead of at GUI startup (wgpu compiles lazily at runtime).

const SHADER: &str = include_str!("../src/core/shader.wgsl");

#[test]
fn shader_wgsl_parses_and_validates() {
    let module = naga::front::wgsl::parse_str(SHADER)
        .expect("shader.wgsl must parse as valid WGSL");

    // Full type validation (entry point signatures, uniform layouts, etc.)
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    let info = validator
        .validate(&module)
        .expect("shader.wgsl must pass naga type validation");

    // Sanity: the entry points we bind must exist
    let ep_names: Vec<&str> = module
        .entry_points
        .iter()
        .map(|ep| ep.name.as_str())
        .collect();
    assert!(ep_names.contains(&"vs_main"), "vs_main entry point missing");
    assert!(ep_names.contains(&"fs_main"), "fs_main entry point missing");
    let _ = info;
}
