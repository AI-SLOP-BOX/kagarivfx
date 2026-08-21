use clap::{Parser, Subcommand};
use aftereffects_oss::core::timeline::{Project, Composition};
use aftereffects_oss::core::software_renderer::render_frame_to_pixels;

#[derive(Parser)]
#[command(name = "aevfx")]
#[command(about = "AE VFX - Headless VFX rendering engine CLI", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a composition to PNG sequence or MP4
    Render {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Composition name or index (default: first)
        #[arg(short, long)]
        comp: Option<String>,

        /// Output directory (for PNG sequence) or file path (for MP4)
        #[arg(short, long, default_value = "./output")]
        output: String,

        /// Output format: png, mp4, gif
        #[arg(short, long, default_value = "png")]
        format: String,

        /// Start frame (inclusive)
        #[arg(long, default_value = "0")]
        from: u32,

        /// End frame (inclusive, default: last frame)
        #[arg(long)]
        to: Option<u32>,

        /// Output width (overrides composition size)
        #[arg(long)]
        width: Option<u32>,

        /// Output height (overrides composition size)
        #[arg(long)]
        height: Option<u32>,

        /// Exposure value for color grading (-5.0 to 5.0)
        #[arg(long, default_value = "0.0")]
        exposure: f32,

        /// LUT mode: 0=none, 1=linear, 2=ACES
        #[arg(long, default_value = "0")]
        lut: u32,

        /// Number of parallel render threads
        #[arg(short, long, default_value = "4")]
        threads: usize,
    },

    /// List available effects and their parameters
    Effects,

    /// Show project composition info
    Info {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
    },

    /// Validate a project file
    Validate {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
    },

    /// Render a single frame to PNG (for testing)
    Frame {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Frame number to render
        #[arg(short, long, default_value = "0")]
        frame: u32,

        /// Output PNG path
        #[arg(short, long, default_value = "./frame.png")]
        output: String,

        /// Output width
        #[arg(long)]
        width: Option<u32>,

        /// Output height
        #[arg(long)]
        height: Option<u32>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Render { project, comp, output, format, from, to, width, height, exposure, lut, threads: _ } => {
            cmd_render(RenderArgs {
                project_path: project,
                comp_ref: comp,
                output,
                format,
                from,
                to,
                width,
                height,
                exposure,
                lut,
            })?;
        }
        Commands::Effects => {
            cmd_effects();
        }
        Commands::Info { project } => {
            cmd_info(&project)?;
        }
        Commands::Validate { project } => {
            cmd_validate(&project)?;
        }
        Commands::Frame { project, frame, output, width, height } => {
            cmd_frame(&project, frame, &output, width, height)?;
        }
    }

    Ok(())
}

fn load_project(path: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read project file '{}': {}", path, e))?;
    // Schema-migrated load: handles versioned wrappers and legacy files,
    // and sanitizes broken parent-child links on the way in.
    let project = aftereffects_oss::core::project_migration::load_project_migrated(&json)
        .map_err(|e| format!("Failed to parse project JSON: {}", e))?;
    Ok(project)
}

fn find_comp<'a>(project: &'a Project, comp_ref: Option<&str>) -> Result<&'a Composition, Box<dyn std::error::Error>> {
    match comp_ref {
        None => {
            if project.compositions.is_empty() {
                return Err("Project has no compositions".into());
            }
            Ok(project.compositions.first().unwrap())
        }
        Some(name) => {
            if let Ok(idx) = name.parse::<usize>() {
                project.compositions.get(idx)
                    .ok_or_else(|| format!("Composition index {} out of range (project has {})", idx, project.compositions.len()).into())
            } else {
                project.compositions.iter().find(|c| c.name == name)
                    .ok_or_else(|| format!("Composition '{}' not found", name).into())
            }
        }
    }
}

/// Bundled render parameters shared by all export backends.
struct RenderSpec<'a> {
    output: &'a str,
    from: u32,
    to: u32,
    w: u32,
    h: u32,
    exposure: f32,
    lut: u32,
}

struct RenderArgs {
    project_path: String,
    comp_ref: Option<String>,
    output: String,
    format: String,
    from: u32,
    to: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    exposure: f32,
    lut: u32,
}

fn cmd_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(&args.project_path)?;
    let comp = find_comp(&project, args.comp_ref.as_deref())?;

    let spec = RenderSpec {
        output: &args.output,
        from: args.from,
        to: args.to.unwrap_or(comp.duration_frames.saturating_sub(1)),
        w: args.width.unwrap_or(comp.width),
        h: args.height.unwrap_or(comp.height),
        exposure: args.exposure,
        lut: args.lut,
    };
    let format = args.format.as_str();

    eprintln!("Rendering composition: {}", comp.name);
    eprintln!("  Size: {}x{}", spec.w, spec.h);
    eprintln!("  Frames: {}..={} ({} frames)", spec.from, spec.to, spec.to.saturating_sub(spec.from) + 1);
    eprintln!("  Format: {}", format);

    if format == "mp4" {
        render_to_mp4(comp, &spec)?;
    } else if format == "gif" {
        render_to_gif(comp, &spec)?;
    } else {
        render_to_png_sequence(comp, &spec)?;
    }

    Ok(())
}

fn render_to_png_sequence(comp: &Composition, spec: &RenderSpec) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;

    let total = spec.to.saturating_sub(spec.from) + 1;
    for frame in spec.from..=spec.to {
        let pixels = render_frame_to_pixels(comp, frame, spec.w, spec.h, spec.exposure, spec.lut);
        let path = format!("{}/frame_{:05}.png", spec.output, frame);
        write_png(&path, &pixels, spec.w, spec.h)?;

        let progress = frame.saturating_sub(spec.from) + 1;
        if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    eprintln!("\n  Done! Output: {}/", spec.output);
    Ok(())
}

fn render_to_mp4(comp: &Composition, spec: &RenderSpec) -> Result<(), Box<dyn std::error::Error>> {
    use aftereffects_oss::core::ffmpeg_export::{is_ffmpeg_available, ExportConfig, start_export_cancelable};
    use std::sync::{Arc, atomic::AtomicBool};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
    };

    let (from, _to, w, h, exposure, lut) = (spec.from, spec.to, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let start_export = move || {
        let _ = start_export_cancelable(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            render_frame_to_pixels(&comp_clone, actual_frame, w, h, exposure, lut)
        });
    };

    std::thread::spawn(start_export);

    while let Ok(event) = rx.recv() {
        match event {
            aftereffects_oss::ExportEvent::Progress(frac, msg) => {
                let pct = (frac * 100.0) as u32;
                eprint!("\r  Encoding: {}% {}", pct, msg);
            }
            aftereffects_oss::ExportEvent::Finished(msg) => {
                eprintln!("\n  {}", msg);
            }
            aftereffects_oss::ExportEvent::Error(msg) => {
                eprintln!("\n  Error: {}", msg);
                return Err(msg.into());
            }
        }
    }

    Ok(())
}

fn render_to_gif(comp: &Composition, spec: &RenderSpec) -> Result<(), Box<dyn std::error::Error>> {
    use aftereffects_oss::core::ffmpeg_export::{is_ffmpeg_available, ExportConfig, start_gif_export};
    use std::sync::{Arc, atomic::AtomicBool};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
    };

    let (from, w, h, exposure, lut) = (spec.from, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let start_export = move || {
        let _ = start_gif_export(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            render_frame_to_pixels(&comp_clone, actual_frame, w, h, exposure, lut)
        });
    };

    std::thread::spawn(start_export);

    while let Ok(event) = rx.recv() {
        match event {
            aftereffects_oss::ExportEvent::Progress(frac, msg) => {
                let pct = (frac * 100.0) as u32;
                eprint!("\r  GIF Encoding: {}% {}", pct, msg);
            }
            aftereffects_oss::ExportEvent::Finished(msg) => {
                eprintln!("\n  {}", msg);
            }
            aftereffects_oss::ExportEvent::Error(msg) => {
                eprintln!("\n  Error: {}", msg);
                return Err(msg.into());
            }
        }
    }

    Ok(())
}

fn cmd_effects() {
    println!("Available Effects:");
    println!("==================");
    let effects = vec![
        ("GaussianBlur", "Fast box blur (CPU)"),
        ("ColorTint", "Color tint overlay"),
        ("DropShadow", "Drop shadow with blur"),
        ("ChromaticAberration", "RGB channel split (CPU)"),
        ("Vignette", "Darkened edges (CPU)"),
        ("Levels", "Input/output levels + gamma (CPU)"),
        ("HueSaturation", "HSL adjustment (CPU)"),
        ("Glow", "Bloom / glow effect"),
        ("MotionBlur", "Shutter-based motion blur (CPU)"),
        ("MeshWarp", "4-corner mesh warp (CPU)"),
        ("ColorGradeLUT", "LUT color grading (CPU)"),
        ("ColorSpaceConvert", "Color space transform (CPU)"),
        ("FilmGrain", "Film grain noise (CPU)"),
        ("FractalNoise", "Procedural noise (fBm/turb/ridge) (CPU)"),
        ("Curves", "Per-channel bezier tone curve (CPU)"),
        ("DisplacementMap", "Layer-based displacement warp (CPU)"),
        ("CompoundBlur", "Variable blur with intensity map (CPU)"),
        ("Minimax", "Dilate/erode matte (CPU)"),
        ("ShiftChannels", "RGBA channel swap/remap (CPU)"),
        ("Twirl", "Twirl distortion (CPU)"),
        ("Bulge", "Bulge distortion (CPU)"),
        ("Posterize", "Reduce color levels (CPU)"),
        ("Invert", "Invert colors (CPU)"),
        ("Offset", "Pixel offset with wrap (CPU)"),
        ("DirectionalBlur", "Directional blur (CPU)"),
        ("RadialBlur", "Radial blur (CPU)"),
        ("Sharpen", "Unsharp mask (CPU)"),
        ("Threshold", "Binary threshold (CPU)"),
        ("LinearWipe", "Linear wipe transition (CPU)"),
        ("SimpleChoker", "Matte choker (CPU)"),
        ("ChromaKey", "Green screen key (CPU)"),
        ("Spherize", "Sphere distortion (CPU)"),
        ("TurbulentDisplace", "Turbulence displacement (CPU)"),
        ("Colorama", "Color cycle (CPU)"),
    ];
    for (name, desc) in &effects {
        println!("  {:<24} {}", name, desc);
    }
    println!("\nTotal: {} effects", effects.len());
}

fn cmd_info(project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;

    println!("Project: {} compositions", project.compositions.len());
    println!("Assets: {}", project.assets.len());
    println!();

    for (i, comp) in project.compositions.iter().enumerate() {
        println!("Composition {}: \"{}\"", i, comp.name);
        println!("  Size: {}x{}", comp.width, comp.height);
        println!("  FPS: {}", comp.fps);
        println!("  Duration: {} frames ({:.2}s)", comp.duration_frames, comp.duration_frames as f32 / comp.fps as f32);
        println!("  Layers: {}", comp.layers.len());

        for layer in comp.layers.iter() {
            let type_name = match &layer.layer_type {
                aftereffects_oss::core::timeline::LayerType::Solid { .. } => "Solid",
                aftereffects_oss::core::timeline::LayerType::Text { .. } => "Text",
                aftereffects_oss::core::timeline::LayerType::Image { .. } => "Image",
                aftereffects_oss::core::timeline::LayerType::Shape { .. } => "Shape",
                aftereffects_oss::core::timeline::LayerType::Null => "Null",
                aftereffects_oss::core::timeline::LayerType::PreComp { .. } => "PreComp",
                aftereffects_oss::core::timeline::LayerType::Audio { .. } => "Audio",
                aftereffects_oss::core::timeline::LayerType::AdjustmentLayer => "Adjustment",
                aftereffects_oss::core::timeline::LayerType::Particle { .. } => "Particle",
            };
            let vis = if layer.visible { "●" } else { "○" };
            println!("    {} [{}] {} ({} effects)", vis, type_name, layer.name, layer.effects.len());
        }
        println!();
    }

    Ok(())
}

fn cmd_validate(project_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    if project.compositions.is_empty() {
        errors.push("Project has no compositions".to_string());
    }

    for comp in project.compositions.iter() {
        if comp.width == 0 || comp.height == 0 {
            errors.push(format!("Composition '{}': invalid dimensions {}x{}", comp.name, comp.width, comp.height));
        }
        if comp.width > aftereffects_oss::core::software_renderer::MAX_RENDER_DIMENSION
            || comp.height > aftereffects_oss::core::software_renderer::MAX_RENDER_DIMENSION
        {
            errors.push(format!(
                "Composition '{}': dimensions {}x{} exceed render limit {}",
                comp.name,
                comp.width,
                comp.height,
                aftereffects_oss::core::software_renderer::MAX_RENDER_DIMENSION
            ));
        }
        if comp.layers.len() > 10_000 {
            warnings.push(format!(
                "Composition '{}': very high layer count ({}), rendering may be slow",
                comp.name,
                comp.layers.len()
            ));
        }
        if comp.fps == 0 {
            errors.push(format!("Composition '{}': FPS is 0", comp.name));
        }
        if comp.duration_frames == 0 {
            warnings.push(format!("Composition '{}': duration is 0 frames", comp.name));
        }

        for layer in comp.layers.iter() {
            if layer.in_frame >= layer.out_frame {
                warnings.push(format!("Composition '{}' layer '{}': in_frame >= out_frame ({} >= {})", comp.name, layer.name, layer.in_frame, layer.out_frame));
            }

            if let Some(ref parent_id) = layer.parent_id {
                if !comp.layers.iter().any(|l| &l.id == parent_id) {
                    errors.push(format!("Composition '{}' layer '{}': parent '{}' not found", comp.name, layer.name, parent_id));
                }
            }

            for effect in &layer.effects {
                if effect.name.is_empty() {
                    warnings.push(format!("Composition '{}' layer '{}': effect has empty name", comp.name, layer.name));
                }
            }
        }

    }

    // ── Project-wide pre-comp graph analysis (references + cycle detection) ──
    {
        use std::collections::{HashMap, HashSet};
        // Collect every composition in the project (top-level + nested sub-comps)
        let mut all_comps: HashMap<&str, &aftereffects_oss::core::timeline::Composition> = HashMap::new();
        fn collect<'a>(comp: &'a aftereffects_oss::core::timeline::Composition, all: &mut HashMap<&'a str, &'a aftereffects_oss::core::timeline::Composition>) {
            if all.insert(comp.id.as_str(), comp).is_some() {
                return;
            }
            for sub in &comp.sub_compositions {
                collect(sub, all);
            }
        }
        for comp in project.compositions.iter() {
            collect(comp, &mut all_comps);
        }

        // Build edges: comp id -> referenced pre-comp ids
        let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
        for (id, comp) in &all_comps {
            let mut refs = Vec::new();
            for layer in &comp.layers {
                if let aftereffects_oss::core::timeline::LayerType::PreComp { comp_id } = &layer.layer_type {
                    if !all_comps.contains_key(comp_id.as_str()) {
                        errors.push(format!(
                            "Composition '{}': pre-comp '{}' referenced but not found",
                            comp.name, comp_id
                        ));
                    } else {
                        refs.push(comp_id.as_str());
                    }
                }
            }
            edges.insert(*id, refs);
        }

        // DFS cycle detection across all comps
        fn dfs(
            node: &str,
            edges: &HashMap<&str, Vec<&str>>,
            visiting: &mut Vec<String>,
            visited: &mut HashSet<String>,
            reported: &mut HashSet<String>,
            errors: &mut Vec<String>,
        ) {
            if reported.contains(node) {
                return;
            }
            if let Some(pos) = visiting.iter().position(|v| v == node) {
                let cycle_path: Vec<String> = visiting[pos..].to_vec();
                errors.push(format!("Pre-comp cycle detected: {}", cycle_path.join(" -> ")));
                reported.insert(node.to_string());
                return;
            }
            if visited.contains(node) {
                return;
            }
            visiting.push(node.to_string());
            if let Some(refs) = edges.get(node) {
                for r in refs.clone() {
                    dfs(r, edges, visiting, visited, reported, errors);
                }
            }
            visiting.pop();
            visited.insert(node.to_string());
        }
        let keys: Vec<&str> = edges.keys().copied().collect();
        let mut visiting = Vec::new();
        let mut visited = HashSet::new();
        let mut reported = HashSet::new();
        for k in keys {
            dfs(k, &edges, &mut visiting, &mut visited, &mut reported, &mut errors);
        }

    }

    // ── Per-composition layer checks ──
    for comp in project.compositions.iter() {
        // Expression script sanity
        for layer in &comp.layers {
            let exprs = [
                ("position", &layer.transform.position_expression),
                ("rotation", &layer.transform.rotation_expression),
                ("scale", &layer.transform.scale_expression),
                ("opacity", &layer.transform.opacity_expression),
            ];
            for (prop, expr) in exprs {
                if let Some(aftereffects_oss::core::timeline::Expression::Raw(script)) = expr {
                    if script.len() > 10_000 {
                        errors.push(format!(
                            "Composition '{}' layer '{}': {} expression too long ({} bytes)",
                            comp.name, layer.name, prop, script.len()
                        ));
                    }
                }
            }
        }

        // Extreme property values (NaN / out-of-range)
        for layer in &comp.layers {
            let opacity = layer.transform.opacity.evaluate(0);
            if !opacity.is_finite() || !(-1.0..=1e6).contains(&opacity) {
                warnings.push(format!(
                    "Composition '{}' layer '{}': suspicious opacity value {}",
                    comp.name, layer.name, opacity
                ));
            }
            let scale = layer.transform.scale.evaluate(0);
            if !scale.iter().all(|v| v.is_finite()) || scale.iter().any(|v| v.abs() > 1e6) {
                warnings.push(format!(
                    "Composition '{}' layer '{}': suspicious scale value {:?}",
                    comp.name, layer.name, scale
                ));
            }
        }

        // Check for parent cycles
        for layer in &comp.layers {
            if let Some(ref parent_id) = layer.parent_id {
                let mut visited = std::collections::HashSet::new();
                visited.insert(&layer.id);
                let mut current = parent_id;
                loop {
                    if visited.contains(current) {
                        errors.push(format!("Composition '{}': parent cycle detected involving '{}'", comp.name, current));
                        break;
                    }
                    visited.insert(current);
                    match comp.layers.iter().find(|l| &l.id == current) {
                        Some(parent) => {
                            if let Some(ref next_parent) = parent.parent_id {
                                current = next_parent;
                            } else {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("✓ Project is valid");
    } else {
        for w in &warnings {
            println!("⚠ Warning: {}", w);
        }
        for e in &errors {
            println!("✗ Error: {}", e);
        }
        println!("\n{} warnings, {} errors", warnings.len(), errors.len());
        if !errors.is_empty() {
            std::process::exit(1);
        }
    }

    Ok(())
}

fn cmd_frame(
    project_path: &str, frame: u32, output: &str,
    width: Option<u32>, height: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = project.compositions.first()
        .ok_or("Project has no compositions")?;

    let w = width.unwrap_or(comp.width);
    let h = height.unwrap_or(comp.height);

    eprintln!("Rendering frame {} from \"{}\" ({}x{})", frame, comp.name, w, h);

    let pixels = render_frame_to_pixels(comp, frame, w, h, 0.0, 0);
    write_png(output, &pixels, w, h)?;

    eprintln!("Saved: {}", output);
    Ok(())
}

fn write_png(path: &str, rgba: &[u8], width: u32, height: u32) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let w = &mut std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}
