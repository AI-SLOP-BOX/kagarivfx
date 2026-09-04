use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::{Composition, Project};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "kagari")]
#[command(about = "Kagari VFX — Headless compositing & motion-graphics engine", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Png,
    Mp4,
    Gif,
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
        #[arg(short, long, default_value = "png", value_enum)]
        format: OutputFormat,

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

        /// Live binding values such as audio.bass=0.75; may be repeated
        #[arg(long = "binding", value_parser = parse_binding_value)]
        bindings: Vec<(String, f64)>,
    },

    /// List available effects and their parameters
    Effects {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Show project composition info
    Info {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
    },

    /// Show unified audio, tempo, and cross-domain binding information
    ProductionInfo {
        /// Path to a unified production JSON file
        #[arg(short, long)]
        project: String,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },

    /// Evaluate cross-domain automation bindings from source=value pairs
    Bindings {
        /// Path to a unified production JSON file
        #[arg(short, long)]
        project: String,
        /// Source values such as audio.bass=0.75; may be repeated
        #[arg(long = "value", value_parser = parse_binding_value)]
        values: Vec<(String, f64)>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Write resolved targets as opacity keyframes at this frame
        #[arg(long)]
        apply_frame: Option<u32>,
        /// Evaluate saved automation curves at this video frame
        #[arg(long)]
        frame: Option<u32>,
        /// Composition name or zero-based index used with --frame
        #[arg(short, long)]
        comp: Option<String>,
    },

    /// Add Logic Pro-style sample-position automation to a production file
    AddSampleBinding {
        /// Path to a unified production JSON file
        #[arg(short, long)]
        project: String,
        /// Audio/DAW source endpoint
        #[arg(long)]
        source: String,
        /// VFX target endpoint
        #[arg(long)]
        target: String,
        /// Repeated absolute sample points in `sample=value` form
        #[arg(long = "sample-point", value_parser = parse_sample_point)]
        points: Vec<(i64, f64)>,
        /// Source automation minimum
        #[arg(long)]
        input_min: f64,
        /// Source automation maximum
        #[arg(long)]
        input_max: f64,
        /// VFX target minimum
        #[arg(long)]
        output_min: f64,
        /// VFX target maximum
        #[arg(long)]
        output_max: f64,
    },

    /// Validate a project file
    Validate {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,
    },

    /// Render and analyze a frame for exposure clipping and focus quality
    Qc {
        #[arg(short, long)]
        project: String,
        /// Composition name or index (default: first)
        #[arg(short, long)]
        comp: Option<String>,
        /// Analyze one frame; overrides --from and --to
        #[arg(short, long)]
        frame: Option<u32>,
        /// First frame for batch analysis
        #[arg(long, default_value = "0")]
        from: u32,
        /// Last frame for batch analysis (default: composition end)
        #[arg(long)]
        to: Option<u32>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Optional report output path; stdout is used when omitted
        #[arg(short, long)]
        output: Option<String>,
        /// Return a failing exit status when any QC warning is found
        #[arg(long)]
        fail_on_warnings: bool,
    },

    /// Render a single frame to PNG (for testing)
    /// Run an automation script (Rhai) against a project
    Script {
        /// Path to project JSON file (created/overwritten by save_project)
        #[arg(short, long)]
        project: String,

        /// Path to .rhai script file
        #[arg(short, long)]
        file: String,
    },
    /// Render a single frame to PNG (convenience shorthand)
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

    /// Export the project to Lottie / Bodymovin JSON (transforms + keyframes; effects not included)
    Lottie {
        /// Path to project JSON file
        #[arg(short, long)]
        project: String,

        /// Output JSON path
        #[arg(short, long, default_value = "./lottie_export.json")]
        output: String,
    },

    /// Track points through a PNG frame sequence using markerless optical flow
    Mocap {
        /// Directory containing frame_*.png files
        #[arg(long)]
        frames: String,
        /// Initial point as x,y; may be repeated
        #[arg(long = "point", value_parser = parse_point)]
        points: Vec<[f32; 2]>,
        /// Output JSON path
        #[arg(short, long, default_value = "./mocap.json")]
        output: String,
        /// Block matching radius
        #[arg(long, default_value = "2")]
        block_radius: i32,
        /// Search radius in pixels
        #[arg(long, default_value = "16")]
        search_radius: i32,
        /// Discard samples below this confidence
        #[arg(long, default_value = "0.0")]
        min_confidence: f32,
        /// Estimate a 17-joint humanoid pose instead of tracking explicit points
        #[arg(long)]
        pose: bool,
        /// Output frame rate used for BVH frame time
        #[arg(long, default_value = "30.0")]
        fps: f32,
        /// Maximum automatically detected features used for pose estimation
        #[arg(long, default_value = "64")]
        max_features: usize,
        /// Minimum spacing between automatically detected features
        #[arg(long, default_value = "12")]
        feature_spacing: u32,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Render {
            project,
            comp,
            output,
            format,
            from,
            to,
            width,
            height,
            exposure,
            lut,
            bindings,
        } => {
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
                bindings,
            })?;
        }
        Commands::Effects { json } => {
            cmd_effects(json);
        }
        Commands::Info { project } => {
            cmd_info(&project)?;
        }
        Commands::ProductionInfo { project, json } => {
            cmd_production_info(&project, json)?;
        }
        Commands::Bindings {
            project,
            values,
            json,
            apply_frame,
            frame,
            comp,
        } => {
            cmd_bindings(&project, &values, json, apply_frame, frame, comp.as_deref())?;
        }
        Commands::AddSampleBinding {
            project,
            source,
            target,
            points,
            input_min,
            input_max,
            output_min,
            output_max,
        } => {
            cmd_add_sample_binding(
                &project,
                &source,
                &target,
                points,
                (input_min, input_max),
                (output_min, output_max),
            )?;
        }
        Commands::Validate { project } => {
            cmd_validate(&project)?;
        }
        Commands::Qc {
            project,
            comp,
            frame,
            from,
            to,
            json,
            output,
            fail_on_warnings,
        } => {
            cmd_qc(QcArgs {
                project_path: project,
                comp_ref: comp,
                frame,
                from,
                to,
                json,
                output,
                fail_on_warnings,
            })?;
        }
        Commands::Script { project, file } => {
            let project_meta = std::fs::metadata(&project)
                .map_err(|e| std::io::Error::new(e.kind(), format!("cannot read project: {}", e)))?;
            if project_meta.len() > 50 * 1024 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "project file too large ({} MB, limit 50 MB)",
                        project_meta.len() / (1024 * 1024)
                    ),
                ).into());
            }
            let json = std::fs::read_to_string(&project)?;
            let mut production =
                kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
                    .ok();
            let mut proj = if let Some(document) = production.as_mut() {
                document.project().clone()
            } else {
                load_project(&project)?
            };
            // Tolerate wrapped ProjectFile format too
            let script_meta = std::fs::metadata(&file)
                .map_err(|e| std::io::Error::new(e.kind(), format!("cannot read script: {}", e)))?;
            if script_meta.len() > 1024 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "script file too large ({} KB, limit 1 MB)",
                        script_meta.len() / 1024
                    ),
                ).into());
            }
            let source = std::fs::read_to_string(&file)?;
            let logs = kagari_vfx::automation::run_script(&mut proj, &source)?;
            for l in logs {
                println!("{l}");
            }
            if let Some(document) = production.as_mut() {
                *document.project_mut() = proj;
                document
                    .save_atomic(&project)
                    .map_err(std::io::Error::other)?;
            } else {
                let out = serde_json::to_string_pretty(&proj)?;
                let parent = std::path::Path::new(&project)
                    .parent()
                    .unwrap_or(std::path::Path::new("."));
                let tmp = parent.join(format!(
                    ".kagari_save_{}",
                    std::process::id()
                ));
                std::fs::write(&tmp, &out)?;
                std::fs::rename(&tmp, &project)?;
            }
            println!("project saved → {}", project);
        }
        Commands::Frame {
            project,
            frame,
            output,
            width,
            height,
        } => {
            cmd_frame(&project, frame, &output, width, height)?;
        }
        Commands::Lottie { project, output } => {
            cmd_lottie(&project, &output)?;
        }
        Commands::Mocap {
            frames,
            points,
            output,
            block_radius,
            search_radius,
            min_confidence,
            pose,
            fps,
            max_features,
            feature_spacing,
        } => {
            cmd_mocap(
                &frames,
                &points,
                &output,
                block_radius,
                search_radius,
                min_confidence,
                pose,
                fps,
                max_features,
                feature_spacing,
            )?;
        }
    }

    Ok(())
}

fn parse_point(value: &str) -> Result<[f32; 2], String> {
    let mut parts = value.split(',');
    let x = parts
        .next()
        .ok_or("point must be x,y")?
        .parse::<f32>()
        .map_err(|_| "invalid x")?;
    let y = parts
        .next()
        .ok_or("point must be x,y")?
        .parse::<f32>()
        .map_err(|_| "invalid y")?;
    if parts.next().is_some() || !x.is_finite() || !y.is_finite() {
        return Err("point must contain two finite numbers".into());
    }
    Ok([x, y])
}

fn parse_binding_value(value: &str) -> Result<(String, f64), String> {
    let (source, raw) = value
        .split_once('=')
        .ok_or("binding value must use source=value")?;
    if source.trim().is_empty() || source != source.trim() {
        return Err("binding source must be non-empty and trimmed".into());
    }
    let number = raw
        .parse::<f64>()
        .map_err(|_| "binding value is not a number")?;
    if !number.is_finite() {
        return Err("binding value must be finite".into());
    }
    Ok((source.to_string(), number))
}

fn parse_sample_point(value: &str) -> Result<(i64, f64), String> {
    let (sample, raw) = value
        .split_once('=')
        .ok_or("sample point must use sample=value")?;
    let sample = sample
        .parse::<i64>()
        .map_err(|_| "sample position is not an integer")?;
    let value = raw
        .parse::<f64>()
        .map_err(|_| "sample point value is not a number")?;
    if !value.is_finite() {
        return Err("sample point value must be finite".into());
    }
    Ok((sample, value))
}

fn cmd_add_sample_binding(
    project_path: &str,
    source: &str,
    target: &str,
    points: Vec<(i64, f64)>,
    input_range: (f64, f64),
    output_range: (f64, f64),
) -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(project_path)?;
    let mut document =
        kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
            .map_err(|error| format!("Not a valid production document: {error}"))?;
    document
        .add_sample_automation_binding(source, target, points, input_range, output_range)
        .map_err(std::io::Error::other)?;
    document
        .save_atomic(project_path)
        .map_err(std::io::Error::other)?;
    println!("sample automation binding added: {source} -> {target}");
    Ok(())
}

fn cmd_bindings(
    project_path: &str,
    values: &[(String, f64)],
    json_output: bool,
    apply_frame: Option<u32>,
    frame: Option<u32>,
    comp_ref: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(project_path)?;
    let mut document =
        kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
            .map_err(|error| format!("Not a valid production document: {error}"))?;
    let sources = values
        .iter()
        .cloned()
        .collect::<std::collections::HashMap<_, _>>();
    let mut targets = document.evaluate_bindings(&sources);
    if let Some(frame) = frame {
        let composition = if let Some(comp_ref) = comp_ref {
            if let Ok(index) = comp_ref.parse::<usize>() {
                document.project().compositions.get(index)
            } else {
                document
                    .project()
                    .compositions
                    .iter()
                    .find(|composition| composition.name == comp_ref)
            }
        } else {
            document.project().compositions.first()
        }
        .ok_or("production document has no matching composition")?;
        let rate = kagari_vfx::core::unified_time::FrameRate::new(composition.fps.max(1), 1)
            .ok_or("composition has an invalid frame rate")?;
        let time = kagari_vfx::core::unified_time::Time::from_frame(frame as i64, rate);
        targets.extend(document.evaluate_bindings_at_time(time));
    }
    if let Some(frame) = apply_frame {
        let applied = document.apply_binding_targets_at_frame(&targets, frame);
        document
            .save_atomic(project_path)
            .map_err(std::io::Error::other)?;
        eprintln!("applied {applied} binding target(s) at frame {frame}");
    }
    if json_output {
        println!("{}", serde_json::to_string_pretty(&targets)?);
    } else {
        for (target, value) in targets {
            println!("{target}={value}");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_mocap(
    directory: &str,
    points: &[[f32; 2]],
    output: &str,
    block_radius: i32,
    search_radius: i32,
    min_confidence: f32,
    pose: bool,
    fps: f32,
    max_features: usize,
    feature_spacing: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::optical_flow_timewarp::{
        estimate_markerless_pose, filter_pose_frames_by_quality, markerless_pose_to_bvh,
        markerless_pose_to_csv, markerless_pose_to_json, name_and_connect_markerless_pose_track,
        track_markerless_motion,
    };
    let mut paths = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() || (!pose && points.is_empty()) {
        return Err(if pose {
            "frames are required"
        } else {
            "frames and at least one --point are required"
        }
        .into());
    }
    let mut decoded = Vec::with_capacity(paths.len());
    let mut dimensions = None;
    for path in &paths {
        // Peek at the PNG header for dimensions before full decode
        let (w, h) = if path.extension().and_then(|e| e.to_str()) == Some("png") {
            let file = std::fs::File::open(path)?;
            let mut buf = std::io::BufReader::new(file);
            let mut decoder = png::Decoder::new(&mut buf);
            let info = decoder
                .read_header_info()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            (info.width, info.height)
        } else {
            // For non-PNG files we must decode to learn dimensions
            let img = image::open(path)?;
            (img.width(), img.height())
        };
        const MAX_DIM: u32 = 16384;
        if w > MAX_DIM || h > MAX_DIM {
            return Err(format!(
                "Frame {:?} dimensions {}x{} exceed {}x{} limit",
                path, w, h, MAX_DIM, MAX_DIM
            )
            .into());
        }
        let image = image::open(path)?.to_rgba8();
        let current = (image.width(), image.height());
        if dimensions.is_some_and(|expected| expected != current) {
            return Err("all mocap frames must have identical dimensions".into());
        }
        dimensions = Some(current);
        decoded.push(image.into_raw());
    }
    let (width, height) = dimensions.ok_or("no valid frames")?;
    let refs = decoded.iter().map(Vec::as_slice).collect::<Vec<_>>();
    if pose {
        let mut estimated = estimate_markerless_pose(
            &refs,
            width,
            height,
            max_features,
            feature_spacing,
            block_radius,
            search_radius,
        );
        let _ = filter_pose_frames_by_quality(&mut estimated, 1, min_confidence);
        let named = name_and_connect_markerless_pose_track(&estimated);
        let serialized = match std::path::Path::new(output)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json")
        {
            "bvh" => markerless_pose_to_bvh(
                &named,
                if fps.is_finite() && fps > 0.0 {
                    1.0 / fps
                } else {
                    1.0 / 30.0
                },
            ),
            "csv" => markerless_pose_to_csv(&named),
            _ => {
                let pose_value: serde_json::Value =
                    serde_json::from_str(&markerless_pose_to_json(&estimated)?)?;
                serde_json::to_string_pretty(&serde_json::json!({
                    "width": width,
                    "height": height,
                    "fps": if fps.is_finite() && fps > 0.0 { fps } else { 30.0 },
                    "max_features": max_features,
                    "feature_spacing": feature_spacing,
                    "pose": pose_value,
                }))?
            }
        };
        std::fs::write(output, serialized)?;
        println!("wrote pose track to {}", output);
        return Ok(());
    }
    let tracks = track_markerless_motion(&refs, width, height, points, block_radius, search_radius);
    let json_tracks = tracks
        .iter()
        .map(|track| {
            let samples = track
                .samples
                .iter()
                .filter(|sample| {
                    sample.confidence.is_finite() && sample.confidence >= min_confidence
                })
                .map(|sample| {
                    serde_json::json!({
                        "frame": sample.frame,
                        "x": sample.position[0],
                        "y": sample.position[1],
                        "confidence": sample.confidence,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({ "samples": samples })
        })
        .collect::<Vec<_>>();
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "width": width, "height": height, "tracks": json_tracks
        }))?,
    )?;
    println!("wrote {} track(s) to {}", tracks.len(), output);
    Ok(())
}

fn load_project(path: &str) -> Result<Project, Box<dyn std::error::Error>> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Failed to stat project file '{}': {}", path, e))?;
    if meta.len() > 100 * 1024 * 1024 {
        return Err(format!(
            "Project file too large ({} MB, limit 100 MB)",
            meta.len() / (1024 * 1024)
        )
        .into());
    }
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read project file '{}': {}", path, e))?;
    let project = kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
        .map(|document| document.project().clone())
        .or_else(|_| kagari_vfx::core::project_migration::load_project_migrated(&json))
        .map_err(|e| format!("Failed to parse project JSON: {}", e))?;
    Ok(project)
}

fn find_comp<'a>(
    project: &'a Project,
    comp_ref: Option<&str>,
) -> Result<&'a Composition, Box<dyn std::error::Error>> {
    match comp_ref {
        None => {
            if project.compositions.is_empty() {
                return Err("Project has no compositions".into());
            }
            Ok(project.compositions.first().unwrap())
        }
        Some(name) => {
            if let Ok(idx) = name.parse::<usize>() {
                project.compositions.get(idx).ok_or_else(|| {
                    format!(
                        "Composition index {} out of range (project has {})",
                        idx,
                        project.compositions.len()
                    )
                    .into()
                })
            } else {
                project
                    .compositions
                    .iter()
                    .find(|c| c.name == name)
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
    bindings: &'a std::collections::HashMap<String, f64>,
}

struct RenderArgs {
    project_path: String,
    comp_ref: Option<String>,
    output: String,
    format: OutputFormat,
    from: u32,
    to: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    exposure: f32,
    lut: u32,
    bindings: Vec<(String, f64)>,
}

fn cmd_render(args: RenderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let production = std::fs::read_to_string(&args.project_path)
        .ok()
        .and_then(|json| {
            kagari_vfx::core::production_document::ProductionDocument::from_json(&json).ok()
        });
    let project = if let Some(document) = production.as_ref() {
        document.project.clone()
    } else {
        load_project(&args.project_path)?
    };
    let comp = find_comp(&project, args.comp_ref.as_deref())?;
    let comp_index = project
        .compositions
        .iter()
        .position(|candidate| candidate.id == comp.id)
        .unwrap_or(0);

    let binding_values = args
        .bindings
        .iter()
        .cloned()
        .collect::<std::collections::HashMap<_, _>>();
    let spec = RenderSpec {
        output: &args.output,
        from: args.from,
        to: args.to.unwrap_or(comp.duration_frames.saturating_sub(1)),
        w: args.width.unwrap_or(comp.width),
        h: args.height.unwrap_or(comp.height),
        exposure: args.exposure,
        lut: args.lut,
        bindings: &binding_values,
    };
    let format = args.format;

    eprintln!("Rendering composition: {}", comp.name);
    eprintln!("  Size: {}x{}", spec.w, spec.h);
    eprintln!(
        "  Frames: {}..={} ({} frames)",
        spec.from,
        spec.to,
        spec.to.saturating_sub(spec.from) + 1
    );
    eprintln!("  Format: {:?}", format);

    match format {
        OutputFormat::Mp4 => render_to_mp4(comp, &spec, production.as_ref(), comp_index)?,
        OutputFormat::Gif => render_to_gif(comp, &spec, production.as_ref(), comp_index)?,
        OutputFormat::Png => {
            if let Some(document) = production.as_ref() {
                render_to_png_sequence_with_bindings(document, comp_index, &spec)?;
            } else {
                render_to_png_sequence(comp, &spec)?;
            }
        }
    }

    Ok(())
}

fn render_to_png_sequence_with_bindings(
    document: &kagari_vfx::core::production_document::ProductionDocument,
    composition_index: usize,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(spec.output)?;
    let total = spec.to.saturating_sub(spec.from) + 1;
    for frame in spec.from..=spec.to {
        let comp = document
            .composition_for_frame_with_sources(composition_index, frame, spec.bindings)
            .ok_or("composition index out of range")?;
        let pixels = render_frame_to_pixels(&comp, frame, spec.w, spec.h, spec.exposure, spec.lut);
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

fn render_to_png_sequence(
    comp: &Composition,
    spec: &RenderSpec,
) -> Result<(), Box<dyn std::error::Error>> {
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

/// Render a single frame as JPEG with quality setting.
#[allow(dead_code)]
fn write_jpeg(
    path: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or("pixel buffer size mismatch")?;
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let file = std::fs::File::create(path)?;
    let mut writer = std::io::BufWriter::new(file);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, quality.min(100));
    rgb.write_with_encoder(encoder)?;
    Ok(())
}

fn render_to_mp4(
    comp: &Composition,
    spec: &RenderSpec,
    production: Option<&kagari_vfx::core::production_document::ProductionDocument>,
    composition_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::ffmpeg_export::{
        is_ffmpeg_available, start_export_cancelable, ExportConfig,
    };
    use std::sync::{atomic::AtomicBool, Arc};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let audio_wav = comp.layers.iter().find_map(|l| match &l.layer_type {
        kagari_vfx::core::timeline::LayerType::Video { audio_wav, .. } => audio_wav.clone(),
        _ => None,
    });
    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        audio_wav,
        codec: kagari_vfx::core::ffmpeg_export::VideoCodec::H264,
    };

    let (from, _to, w, h, exposure, lut) =
        (spec.from, spec.to, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let production_clone = production.cloned();
    let binding_values = spec.bindings.clone();
    let start_export = move || {
        let _ = start_export_cancelable(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            let frame_comp = production_clone.as_ref().and_then(|document| {
                document.composition_for_frame_with_sources(
                    composition_index,
                    actual_frame,
                    &binding_values,
                )
            });
            render_frame_to_pixels(
                frame_comp.as_ref().unwrap_or(&comp_clone),
                actual_frame,
                w,
                h,
                exposure,
                lut,
            )
        });
    };

    std::thread::spawn(start_export);

    while let Ok(event) = rx.recv() {
        match event {
            kagari_vfx::ExportEvent::Progress(frac, msg) => {
                let pct = (frac * 100.0) as u32;
                eprint!("\r  Encoding: {}% {}", pct, msg);
            }
            kagari_vfx::ExportEvent::Finished(msg) => {
                eprintln!("\n  {}", msg);
            }
            kagari_vfx::ExportEvent::Error(msg) => {
                eprintln!("\n  Error: {}", msg);
                return Err(msg.into());
            }
        }
    }

    Ok(())
}

fn render_to_gif(
    comp: &Composition,
    spec: &RenderSpec,
    production: Option<&kagari_vfx::core::production_document::ProductionDocument>,
    composition_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use kagari_vfx::core::ffmpeg_export::{
        is_ffmpeg_available, start_gif_export, ExportConfig,
    };
    use std::sync::{atomic::AtomicBool, Arc};

    if !is_ffmpeg_available() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let audio_wav = comp.layers.iter().find_map(|l| match &l.layer_type {
        kagari_vfx::core::timeline::LayerType::Video { audio_wav, .. } => audio_wav.clone(),
        _ => None,
    });
    let config = ExportConfig {
        output_path: spec.output.to_string(),
        width: spec.w,
        height: spec.h,
        fps: comp.fps,
        total_frames: spec.to.saturating_sub(spec.from) + 1,
        audio_wav,
        codec: kagari_vfx::core::ffmpeg_export::VideoCodec::H264,
    };

    let (from, w, h, exposure, lut) = (spec.from, spec.w, spec.h, spec.exposure, spec.lut);
    let comp_clone = comp.clone();
    let production_clone = production.cloned();
    let binding_values = spec.bindings.clone();
    let start_export = move || {
        let _ = start_gif_export(config, tx, cancel_clone, move |frame_idx| {
            let actual_frame = from + frame_idx;
            let frame_comp = production_clone.as_ref().and_then(|document| {
                document.composition_for_frame_with_sources(
                    composition_index,
                    actual_frame,
                    &binding_values,
                )
            });
            render_frame_to_pixels(
                frame_comp.as_ref().unwrap_or(&comp_clone),
                actual_frame,
                w,
                h,
                exposure,
                lut,
            )
        });
    };

    std::thread::spawn(start_export);

    while let Ok(event) = rx.recv() {
        match event {
            kagari_vfx::ExportEvent::Progress(frac, msg) => {
                let pct = (frac * 100.0) as u32;
                eprint!("\r  GIF Encoding: {}% {}", pct, msg);
            }
            kagari_vfx::ExportEvent::Finished(msg) => {
                eprintln!("\n  {}", msg);
            }
            kagari_vfx::ExportEvent::Error(msg) => {
                eprintln!("\n  Error: {}", msg);
                return Err(msg.into());
            }
        }
    }

    Ok(())
}

fn cmd_effects(json_output: bool) {
    let effects = vec![
        ("GaussianBlur", "blur", "Fast box blur (CPU)"),
        ("ColorTint", "color", "Color tint overlay"),
        ("DropShadow", "stylize", "Drop shadow with blur"),
        ("ChromaticAberration", "lens", "RGB channel split (CPU)"),
        ("Vignette", "lens", "Darkened edges (CPU)"),
        ("Levels", "color", "Input/output levels + gamma (CPU)"),
        ("HueSaturation", "color", "HSL adjustment (CPU)"),
        ("Glow", "stylize", "Bloom / glow effect"),
        ("MotionBlur", "blur", "Shutter-based motion blur (CPU)"),
        ("MeshWarp", "distort", "4-corner mesh warp (CPU)"),
        ("ColorGradeLUT", "color", "LUT color grading (CPU)"),
        ("ColorSpaceConvert", "color", "Color space transform (CPU)"),
        ("FilmGrain", "noise", "Film grain noise (CPU)"),
        ("FractalNoise", "noise", "Procedural noise (fBm/turb/ridge) (CPU)"),
        ("Curves", "color", "Per-channel bezier tone curve (CPU)"),
        ("DisplacementMap", "distort", "Layer-based displacement warp (CPU)"),
        ("CompoundBlur", "blur", "Variable blur with intensity map (CPU)"),
        ("Minimax", "matte", "Dilate/erode matte (CPU)"),
        ("ShiftChannels", "color", "RGBA channel swap/remap (CPU)"),
        ("Twirl", "distort", "Twirl distortion (CPU)"),
        ("Bulge", "distort", "Bulge distortion (CPU)"),
        ("Posterize", "color", "Reduce color levels (CPU)"),
        ("Invert", "color", "Invert colors (CPU)"),
        ("Offset", "distort", "Pixel offset with wrap (CPU)"),
        ("DirectionalBlur", "blur", "Directional blur (CPU)"),
        ("RadialBlur", "blur", "Radial blur (CPU)"),
        ("Sharpen", "blur", "Unsharp mask (CPU)"),
        ("Threshold", "color", "Binary threshold (CPU)"),
        ("LinearWipe", "transition", "Linear wipe transition (CPU)"),
        ("SimpleChoker", "matte", "Matte choker (CPU)"),
        ("ChromaKey", "keying", "Green screen key (CPU)"),
        ("Spherize", "distort", "Sphere distortion (CPU)"),
        ("TurbulentDisplace", "distort", "Turbulence displacement (CPU)"),
        ("Colorama", "color", "Color cycle (CPU)"),
    ];
    if json_output {
        let arr: Vec<serde_json::Value> = effects
            .iter()
            .map(|(name, cat, desc)| {
                serde_json::json!({
                    "name": name,
                    "category": cat,
                    "description": desc,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&arr).unwrap());
    } else {
        println!("Available Effects:");
        println!("==================");
        for (name, _cat, desc) in &effects {
            println!("  {:<24} {}", name, desc);
        }
        println!("\nTotal: {} effects", effects.len());
    }
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
        println!(
            "  Duration: {} frames ({:.2}s)",
            comp.duration_frames,
            comp.duration_frames as f32 / comp.fps as f32
        );
        println!("  Layers: {}", comp.layers.len());

        for layer in comp.layers.iter() {
            let type_name = match &layer.layer_type {
                kagari_vfx::core::timeline::LayerType::Solid { .. } => "Solid",
                kagari_vfx::core::timeline::LayerType::Text { .. } => "Text",
                kagari_vfx::core::timeline::LayerType::Image { .. } => "Image",
                kagari_vfx::core::timeline::LayerType::Video { .. } => "Video",
                kagari_vfx::core::timeline::LayerType::Shape { .. } => "Shape",
                kagari_vfx::core::timeline::LayerType::Null => "Null",
                kagari_vfx::core::timeline::LayerType::PreComp { .. } => "PreComp",
                kagari_vfx::core::timeline::LayerType::Audio { .. } => "Audio",
                kagari_vfx::core::timeline::LayerType::AdjustmentLayer => "Adjustment",
                kagari_vfx::core::timeline::LayerType::Particle { .. } => "Particle",
            };
            let vis = if layer.visible { "●" } else { "○" };
            println!(
                "    {} [{}] {} ({} effects)",
                vis,
                type_name,
                layer.name,
                layer.effects.len()
            );
        }
        println!();
    }

    Ok(())
}

fn cmd_production_info(
    project_path: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(project_path)?;
    let document =
        kagari_vfx::core::production_document::ProductionDocument::from_json(&json)
            .map_err(|error| format!("Not a valid production document: {error}"))?;
    let clock = document.clock();
    if json_output {
        let output = production_info_value(&document);
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    println!("Production document schema: {}", document.schema_version);
    println!("Audio sample rate: {} Hz", document.audio.sample_rate);
    println!("Audio channels: {}", document.audio.channels.len());
    println!("Tempo changes: {}", document.tempo.changes.len());
    println!("Initial BPM: {:.3}", document.tempo.changes[0].bpm);
    println!("Automation bindings: {}", document.bindings.len());
    println!(
        "Beat at 1 second: {:.3}",
        clock.beat(kagari_vfx::core::unified_time::Time::new(1, 1))
    );
    for binding in &document.bindings {
        println!(
            "  {} -> {} ({} points)",
            binding.source,
            binding.target,
            binding.curve.points.len()
        );
    }
    Ok(())
}

fn production_info_value(
    document: &kagari_vfx::core::production_document::ProductionDocument,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": document.schema_version,
        "sample_rate": document.audio.sample_rate,
        "audio_channels": document.audio.channels.len(),
        "tempo_changes": document.tempo.changes.len(),
        "initial_bpm": document.tempo.changes[0].bpm,
        "automation_bindings": document.bindings.iter().map(|binding| serde_json::json!({
            "source": binding.source,
            "target": binding.target,
            "points": binding.curve.points.len(),
        })).collect::<Vec<_>>(),
    })
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
            errors.push(format!(
                "Composition '{}': invalid dimensions {}x{}",
                comp.name, comp.width, comp.height
            ));
        }
        if comp.width > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
            || comp.height > kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
        {
            errors.push(format!(
                "Composition '{}': dimensions {}x{} exceed render limit {}",
                comp.name,
                comp.width,
                comp.height,
                kagari_vfx::core::software_renderer::MAX_RENDER_DIMENSION
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
                warnings.push(format!(
                    "Composition '{}' layer '{}': in_frame >= out_frame ({} >= {})",
                    comp.name, layer.name, layer.in_frame, layer.out_frame
                ));
            }

            if let Some(ref parent_id) = layer.parent_id {
                if !comp.layers.iter().any(|l| &l.id == parent_id) {
                    errors.push(format!(
                        "Composition '{}' layer '{}': parent '{}' not found",
                        comp.name, layer.name, parent_id
                    ));
                }
            }

            for effect in &layer.effects {
                if effect.name.is_empty() {
                    warnings.push(format!(
                        "Composition '{}' layer '{}': effect has empty name",
                        comp.name, layer.name
                    ));
                }
            }
        }
    }

    // ── Project-wide pre-comp graph analysis (references + cycle detection) ──
    {
        use std::collections::{HashMap, HashSet};
        // Collect every composition in the project (top-level + nested sub-comps)
        let mut all_comps: HashMap<&str, &kagari_vfx::core::timeline::Composition> =
            HashMap::new();
        fn collect<'a>(
            comp: &'a kagari_vfx::core::timeline::Composition,
            all: &mut HashMap<&'a str, &'a kagari_vfx::core::timeline::Composition>,
        ) {
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
                if let kagari_vfx::core::timeline::LayerType::PreComp { comp_id } =
                    &layer.layer_type
                {
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
                errors.push(format!(
                    "Pre-comp cycle detected: {}",
                    cycle_path.join(" -> ")
                ));
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
            dfs(
                k,
                &edges,
                &mut visiting,
                &mut visited,
                &mut reported,
                &mut errors,
            );
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
                if let Some(kagari_vfx::core::timeline::Expression::Raw(script)) = expr {
                    if script.len() > 10_000 {
                        errors.push(format!(
                            "Composition '{}' layer '{}': {} expression too long ({} bytes)",
                            comp.name,
                            layer.name,
                            prop,
                            script.len()
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
                        errors.push(format!(
                            "Composition '{}': parent cycle detected involving '{}'",
                            comp.name, current
                        ));
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
    project_path: &str,
    frame: u32,
    output: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let comp = project
        .compositions
        .first()
        .ok_or("Project has no compositions")?;

    let w = width.unwrap_or(comp.width);
    let h = height.unwrap_or(comp.height);

    eprintln!(
        "Rendering frame {} from \"{}\" ({}x{})",
        frame, comp.name, w, h
    );

    let pixels = render_frame_to_pixels(comp, frame, w, h, 0.0, 0);
    write_png(output, &pixels, w, h)?;

    eprintln!("Saved: {}", output);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct QcReport {
    composition: String,
    frame: u32,
    width: u32,
    height: u32,
    shadow_clipping_percent: f32,
    highlight_clipping_percent: f32,
    sharpness_score: f32,
    warnings: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct QcBatchReport {
    schema_version: u32,
    composition: String,
    from_frame: u32,
    to_frame: u32,
    analyzed_frames: usize,
    warning_frames: usize,
    frames: Vec<QcReport>,
}

struct QcArgs {
    project_path: String,
    comp_ref: Option<String>,
    frame: Option<u32>,
    from: u32,
    to: Option<u32>,
    json: bool,
    output: Option<String>,
    fail_on_warnings: bool,
}

fn cmd_qc(args: QcArgs) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(&args.project_path)?;
    let comp = find_comp(&project, args.comp_ref.as_deref())?;
    if comp.duration_frames == 0 {
        return Err(format!("Composition '{}' has no frames", comp.name).into());
    }
    let (from, to) = match args.frame {
        Some(frame) => (frame, frame),
        None => (args.from, args.to.unwrap_or(comp.duration_frames - 1)),
    };
    if from > to {
        return Err(format!("Invalid QC range: from {from} is after to {to}").into());
    }
    if to >= comp.duration_frames {
        return Err(format!(
            "Frame {} is outside composition '{}' (0..{})",
            to,
            comp.name,
            comp.duration_frames.saturating_sub(1)
        )
        .into());
    }
    let frame_count = to.saturating_sub(from) as usize + 1;
    if frame_count > 1_000_000 {
        return Err("QC range exceeds the 1,000,000 frame safety limit".into());
    }
    let mut reports = Vec::with_capacity(frame_count);
    for current in from..=to {
        let pixels = render_frame_to_pixels(comp, current, comp.width, comp.height, 0.0, 0);
        reports.push(build_qc_report(comp, current, &pixels)?);
        if frame_count > 1 && (reports.len() % 10 == 0 || current == to) {
            eprint!("\rQC analyzed {}/{} frames", reports.len(), frame_count);
        }
    }
    if frame_count > 1 {
        eprintln!();
    }
    let warning_frames = reports
        .iter()
        .filter(|report| !report.warnings.is_empty())
        .count();
    let batch = QcBatchReport {
        schema_version: 1,
        composition: comp.name.clone(),
        from_frame: from,
        to_frame: to,
        analyzed_frames: reports.len(),
        warning_frames,
        frames: reports,
    };
    let rendered = if args.json {
        serde_json::to_string_pretty(&batch)?
    } else {
        format_qc_batch_report(&batch)
    };
    if let Some(path) = args.output.as_deref() {
        std::fs::write(path, format!("{rendered}\n"))?;
        eprintln!("QC report saved → {path}");
    } else {
        println!("{rendered}");
    }
    if args.fail_on_warnings && warning_frames > 0 {
        return Err(format!("QC found warnings in {warning_frames} frame(s)").into());
    }
    Ok(())
}

fn build_qc_report(
    comp: &Composition,
    frame: u32,
    pixels: &[u8],
) -> Result<QcReport, Box<dyn std::error::Error>> {
    use kagari_vfx::core::editor_assist::{analyze_exposure_clipping, sharpness_score};

    let exposure = analyze_exposure_clipping(pixels, comp.width, comp.height, 4, 250)
        .ok_or("Rendered frame has an invalid pixel buffer")?;
    let sharpness = sharpness_score(pixels, comp.width, comp.height).unwrap_or(0.0);
    let shadow_percent = exposure.shadow_clipped_fraction * 100.0;
    let highlight_percent = exposure.highlight_clipped_fraction * 100.0;
    let mut warnings = Vec::new();
    if shadow_percent > 2.0 {
        warnings.push(format!("Shadow clipping is high ({shadow_percent:.2}%)"));
    }
    if highlight_percent > 2.0 {
        warnings.push(format!(
            "Highlight clipping is high ({highlight_percent:.2}%)"
        ));
    }
    if sharpness < 20.0 {
        warnings.push(format!(
            "Possible soft-focus frame (sharpness {sharpness:.1})"
        ));
    }
    Ok(QcReport {
        composition: comp.name.clone(),
        frame,
        width: comp.width,
        height: comp.height,
        shadow_clipping_percent: shadow_percent,
        highlight_clipping_percent: highlight_percent,
        sharpness_score: sharpness,
        warnings,
    })
}

fn format_qc_report(report: &QcReport) -> String {
    let status = if report.warnings.is_empty() {
        "PASS"
    } else {
        "WARN"
    };
    let mut lines = vec![
        format!(
            "QC {status} — {} frame {}",
            report.composition, report.frame
        ),
        format!("Size: {}x{}", report.width, report.height),
        format!("Shadow clipping: {:.2}%", report.shadow_clipping_percent),
        format!(
            "Highlight clipping: {:.2}%",
            report.highlight_clipping_percent
        ),
        format!("Sharpness: {:.1}", report.sharpness_score),
    ];
    lines.extend(report.warnings.iter().map(|warning| format!("⚠ {warning}")));
    lines.join("\n")
}

fn format_qc_batch_report(batch: &QcBatchReport) -> String {
    let status = if batch.warning_frames == 0 {
        "PASS"
    } else {
        "WARN"
    };
    let mut lines = vec![format!(
        "QC {status} — {} frames {}..{} ({} analyzed, {} with warnings)",
        batch.composition,
        batch.from_frame,
        batch.to_frame,
        batch.analyzed_frames,
        batch.warning_frames
    )];
    for report in &batch.frames {
        lines.push(String::new());
        lines.push(format_qc_report(report));
    }
    lines.join("\n")
}

fn cmd_lottie(project_path: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project = load_project(project_path)?;
    let json = kagari_vfx::core::lottie_exporter::export_project_to_json(&project);

    let effect_count: usize = project
        .compositions
        .iter()
        .flat_map(|c| c.layers.iter())
        .map(|l| l.effects.iter().filter(|e| e.enabled).count())
        .sum();

    std::fs::write(output, &json)?;
    eprintln!("Lottie exported → {} ({} bytes)", output, json.len());
    if effect_count > 0 {
        eprintln!(
            "Warning: {} enabled effect(s) are NOT part of Lottie output (format limitation)",
            effect_count
        );
    }
    Ok(())
}

fn write_png(
    path: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let w = &mut std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_comp() -> Composition {
        Composition::new(
            "qc-contract".to_string(),
            "QC Contract".to_string(),
            4,
            4,
            30,
            1,
        )
    }

    #[test]
    fn qc_report_flags_black_and_soft_frame() {
        let report = build_qc_report(&test_comp(), 0, &[0; 4 * 4 * 4]).unwrap();
        assert!(report.shadow_clipping_percent > 99.0);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("Shadow")));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("soft-focus")));
    }

    #[test]
    fn qc_json_schema_is_versioned() {
        let report = build_qc_report(&test_comp(), 0, &[255; 4 * 4 * 4]).unwrap();
        let batch = QcBatchReport {
            schema_version: 1,
            composition: "QC Contract".to_string(),
            from_frame: 0,
            to_frame: 0,
            analyzed_frames: 1,
            warning_frames: 1,
            frames: vec![report],
        };
        let json = serde_json::to_value(batch).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["frames"].as_array().map(Vec::len), Some(1));
        assert!(json["frames"][0]["warnings"].as_array().is_some());
    }

    #[test]
    fn production_info_json_schema_exposes_cross_domain_contract() {
        let mut document = kagari_vfx::core::production_document::ProductionDocument::new(
            Project::default(),
        );
        document.audio.sample_rate = 44_100;
        let value = production_info_value(&document);
        assert_eq!(value["schema_version"], document.schema_version);
        assert_eq!(value["sample_rate"], 44_100);
        assert_eq!(value["tempo_changes"], 1);
        assert_eq!(
            value["automation_bindings"].as_array().map(Vec::len),
            Some(0)
        );
    }

    #[test]
    fn render_binding_values_parse_and_reject_non_finite_input() {
        assert_eq!(
            parse_binding_value("audio.bass=0.75").unwrap(),
            ("audio.bass".into(), 0.75)
        );
        assert!(parse_binding_value("audio.bass=NaN").is_err());
        assert!(parse_binding_value(" audio.bass=0.5").is_err());
    }
}
