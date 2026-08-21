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
        Commands::Render { project, comp, output, format, from, to, width, height, exposure, lut, threads } => {
            cmd_render(&project, comp.as_deref(), &output, &format, from, to, width, height, exposure, lut, threads)?;
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
    let project: Project = serde_json::from_str(&json)
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

fn cmd_render(
    project_path: &str, comp_ref: Option<&str>, output: &str, format: &str,
    from: u32, to: Option<u32>, width: Option<u32>, height: Option<u32>,
    exposure: f32, lut: u32, threads: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = find_comp(&project, comp_ref)?;

    let render_w = width.unwrap_or(comp.width);
    let render_h = height.unwrap_or(comp.height);
    let end_frame = to.unwrap_or(comp.duration_frames.saturating_sub(1));

    eprintln!("Rendering composition: {}", comp.name);
    eprintln!("  Size: {}x{}", render_w, render_h);
    eprintln!("  Frames: {}..={} ({} frames)", from, end_frame, end_frame.saturating_sub(from) + 1);
    eprintln!("  Format: {}", format);

    if format == "mp4" {
        render_to_mp4(&project, comp, output, from, end_frame, render_w, render_h, exposure, lut, threads)?;
    } else if format == "gif" {
        render_to_gif(&project, comp, output, from, end_frame, render_w, render_h, exposure, lut, threads)?;
    } else {
        render_to_png_sequence(&project, comp, output, from, end_frame, render_w, render_h, exposure, lut, threads)?;
    }

    Ok(())
}

fn render_to_png_sequence(
    _project: &Project, comp: &Composition, output_dir: &str,
    from: u32, to: u32, w: u32, h: u32,
    exposure: f32, lut: u32, _threads: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;

    let total = to.saturating_sub(from) + 1;
    for frame in from..=to {
        let pixels = render_frame_to_pixels(comp, frame, w, h, exposure, lut);
        let path = format!("{}/frame_{:05}.png", output_dir, frame);
        write_png(&path, &pixels, w, h)?;

        let progress = frame.saturating_sub(from) + 1;
        if progress % 10 == 0 || progress == total {
            eprint!("\r  Rendered {}/{} frames", progress, total);
        }
    }
    eprintln!("\n  Done! Output: {}/", output_dir);
    Ok(())
}

fn render_to_mp4(
    _project: &Project, comp: &Composition, output_path: &str,
    from: u32, to: u32, w: u32, h: u32,
    exposure: f32, lut: u32, _threads: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use aftereffects_oss::core::ffmpeg_export::{is_ffmpeg_available, ExportConfig, start_export_cancelable};
    use std::sync::{Arc, atomic::AtomicBool};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let config = ExportConfig {
        output_path: output_path.to_string(),
        width: w,
        height: h,
        fps: comp.fps,
        total_frames: to.saturating_sub(from) + 1,
    };

    let comp_clone = comp.clone();
    let start_export = move || {
        let _ = start_export_cancelable(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            render_frame_to_pixels(&comp_clone, actual_frame, w, h, exposure, lut)
        });
    };

    std::thread::spawn(start_export);

    let _total = to.saturating_sub(from) + 1;
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

fn render_to_gif(
    _project: &Project, comp: &Composition, output_path: &str,
    from: u32, to: u32, w: u32, h: u32,
    exposure: f32, lut: u32, _threads: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use aftereffects_oss::core::ffmpeg_export::{is_ffmpeg_available, ExportConfig, start_gif_export};
    use std::sync::{Arc, atomic::AtomicBool};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let config = ExportConfig {
        output_path: output_path.to_string(),
        width: w,
        height: h,
        fps: comp.fps,
        total_frames: to.saturating_sub(from) + 1,
    };

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
