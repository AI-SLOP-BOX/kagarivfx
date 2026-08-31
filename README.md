# AEVFX Studio (Aether VFX)

[![Rust CI](https://github.com/iwatakoumei/aevfx/actions/workflows/ci.yml/badge.svg)](https://github.com/iwatakoumei/aevfx/actions)
[![License: MIT / Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust%20🦀-orange.svg)](https://www.rust-lang.org/)

An experimental, high-performance open-source 2D/3D compositing and motion graphics engine written in Rust, powered by GPU rendering (`wgpu`/Metal), an ergonomic dark GUI (`egui`), and an extensive procedural VFX pipeline.

---

> [!IMPORTANT]
> ### ⚠️ Trademark & Project Disclaimer
> - **Trademark Notice**: *Adobe* and *After Effects* are registered trademarks of Adobe Inc. in the United States and other countries. **AEVFX** is an independent, non-commercial open-source research project and is **NOT** affiliated with, associated with, sponsored by, or endorsed by Adobe Inc.
> - **Experimental Status**: This project is an experimental, hobby-driven research prototype provided **"AS-IS"** without warranties of any kind. Issues and pull requests are managed on a casual, best-effort basis. Feel free to explore, fork, hack, and build upon it!

---

## ✨ Key Features & Architecture

### 🚀 High-Performance Rendering & Color Science
- **Dual Pipeline Compositor**: Rayon-parallelized 64-bit multi-core CPU software compositor with instant GPU preview (`wgpu 22` on Metal/Vulkan/DirectX 12).
- **16/32bpc HDR Color Engine**: Full scene-linear floating-point compositing pipeline with ACES 1.3 Gamut Compression, Display P3, and TPDF (Triangular PDF) dithering for smooth gradients.
- **Subpixel Motion Blur & Ray-Traced 3D Shadows**: High-fidelity shutter angle time-sampling and stochastic soft shadows.

### 🎨 Motion Graphics & 3D Extrusion
- **Cinema-Style 3D Extrusion**: Real-time watertight solid mesh generation with customizable front/back caps and bevel styles (Linear, Convex, Concave).
- **Comprehensive Keyframing**: Bezier curve interpolation with 19 ease presets, spatial motion paths, and graph editor velocity curve adjustment.
- **Rhai Expression Engine**: High-speed, sandboxed expression evaluation supporting `wiggle()`, `loopOut()`, `smooth()`, seed randoms, and dynamic scope injection (`thisComp`, `thisLayer`).
- **Vector & Shape Pipeline**: Pathfinder boolean operations, signed offset paths, wiggle paths, and SVG/Illustrator Bezier path parsing.

### 🔍 VFX Tools & Tracking
- **47+ Built-in Effect Processors**: Gaussian Blur, Advanced Glow, Directional/Radial Blur, Displacement Map, Turbulent Displace, Set Matte, Matte Choker, Hue/Saturation, Curves, Lumetri WB, CRT Glitch, and more.
- **SAD Motion Tracker & Planar Homography**: Subpixel corner pinning, jitter rejection stabilization, and edge-contour extraction.
- **Motion Sketch**: Real-time freehand mouse/stylus gesture recording with automatic frame resampling and keyframe baking.

### 🌐 Interoperability & Interchange
- **Multi-NLE Timeline Bridge**: MLT XML (Kdenlive / Shotcut), OpenShot `.osp`, and bidirectional WebVTT / SRT subtitle interchange.
- **3D Camera Sync**: ASCII `.chan` camera tracking data importer and Blender 3D camera tracker integration.
- **Lottie / Bodymovin Export**: Clean JSON animation export ready for web and mobile deployment.
- **Plugin Bridges**: OpenFX host discovery and C++ Plugin SDK / AEGP bridge architecture.

---

## 📦 Getting Started

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (1.75+ recommended)
- `ffmpeg` (optional, for video sequence decoding & H.264/ProRes/GIF export)

### Build & Run GUI App
```bash
# Launch the desktop GUI editor
cargo run --release --features gui --bin aftereffects-oss
```

### Headless CLI Rendering
```bash
# Render a single frame from project JSON to PNG
cargo run --release --features cli --bin aevfx -- frame --project my_project.json --frame 0 --output /tmp/frame_000.png

# Render a frame sequence or MP4 video
cargo run --release --features cli --bin aevfx -- render --project my_project.json --out-dir /tmp/renders/
```

### Running Test Suite & Quality Checks
```bash
# Run all 1,000+ unit, integration, and fuzz tests
cargo test --all-features -- --test-threads=1

# Strict linter validation
cargo clippy --all-features -- -D warnings
```

---

## 🛠 Project Structure

```
├── src/
│   ├── app_state.rs            # Core application state & project orchestrator
│   ├── core/
│   │   ├── renderer.rs         # wgpu GPU rendering engine & WGSL shader
│   │   ├── software_renderer.rs# Rayon CPU compositor (ground truth render path)
│   │   ├── timeline.rs         # Compositions, Layers, Keyframes, Effects data model
│   │   ├── color_science.rs    # 16/32bpc HDR, ACES 1.3, Display P3, ICC conversions
│   │   ├── c4d_extrusion_engine.rs # 3D Ray-traced extrusion & beveling engine
│   │   ├── tracker_engine.rs   # SAD subpixel tracking & planar homography
│   │   ├── motion_sketch.rs    # Freehand real-time gesture keyframe baking
│   │   ├── parenting_engine.rs # Pick Whip world-transform maintaining parenting
│   │   ├── subtitles.rs        # WebVTT / SRT bidirectional caption parser
│   │   ├── ai_runtime_bridge.rs# Opt-in AI slot system (SAM / RIFE / Depth-Anything)
│   │   └── ...
│   └── ui/                     # egui dark-mode studio panels (Timeline, Viewport, Graph, Scopes)
│       ├── tutorial.rs         # 12-step interactive guided tutorial with chapter navigation
│       └── ...
└── tests/                      # Extensive fuzzing, stress tests & shader validation
```

---

## 🎓 Built-in Tutorial

AEVFX Studio includes a **comprehensive 12-step interactive guided tutorial** covering:

| Chapter | Topics |
|---|---|
| 1: Interface | Screen layout, toolbars, viewport navigation, Beginner/Pro mode toggle |
| 2: Compositions & Layers | Creating comps, adding Text / Solid / Shape / Video / Audio layers |
| 3: Animation & Keyframes | Stopwatch workflow, Easy Ease (F9), Graph Editor, 19 ease presets |
| 4: Effects | Effects Library, 47+ built-in effects, Expressions (wiggle/loopOut/smooth) |
| 5: 3D & Advanced | 3D layers, Camera, Lights, Cinema 4D extrusion, Puppet Tool, 32bit Paint |
| 6: Export & Sharing | MP4 / ProRes / Lottie / GIF / MLT export, headless CLI rendering |

Launch from **Help → 🎓 Start Guided Tutorial** or `Help → Tutorial` in the menu bar.

---

## 📊 Feature Completeness vs. Adobe After Effects

| Feature Area | Status | Notes |
|---|---|---|
| **Keyframe Interpolation** | ✅ Complete | 19 ease presets, Bezier, Hold, Linear, Graph Editor |
| **Expression Engine** | ✅ Complete | Rhai-based: wiggle, loopOut, smooth, time, thisComp |
| **GPU Real-time Preview** | ✅ Complete | wgpu Metal/Vulkan/DX12, adaptive resolution |
| **CPU Ground Truth Render** | ✅ Complete | Rayon-parallel, byte-deterministic |
| **HDR Color Pipeline** | ✅ Complete | 16/32bpc, ACES 1.3, TPDF dithering |
| **3D Extrusion & Bevel** | ✅ Complete | Cinema 4D-style solid mesh, ray-traced soft shadows |
| **Motion Tracker** | ✅ Complete | SAD + planar homography, subpixel refinement |
| **Audio Engine** | ✅ Complete | Multi-track WAV, Mute/Solo, EQ, Compressor |
| **Particle System** | ✅ Complete | Deterministic emission, GPU textures, force fields |
| **Puppet Tool (MLS)** | ✅ Complete | ARAP mesh deformation, bone rigs |
| **32bit HDR Paint** | ✅ Complete | Brush, Eraser, Clone Stamp |
| **MP4 / ProRes Export** | ✅ Complete | via FFmpeg bridge |
| **Lottie Export** | ✅ Complete | Bodymovin-compatible JSON |
| **Motion Sketch** | ✅ Complete | Freehand gesture → keyframe baking |
| **AI Features** | ⚡ Opt-in | Modular bridge: SAM, RIFE, Depth-Anything, ProPainter |
| **16/32bpc Full Pipeline** | 🚧 Partial | Viewport 8bpc; export path 16/32bpc on roadmap |
| **Real-time GPU Effects** | 🚧 Roadmap | Effects currently CPU-dispatched |

---

## 🤝 Contributing

Contributions are very welcome! Before submitting a PR:

```bash
# All tests must pass
cargo test --all-features -- --test-threads=1

# Zero Clippy warnings required
cargo clippy --all-features -- -D warnings

# Commit only the files you changed (multi-AI rule — never git add -A)
git add <your-files>
git commit -m "feat: your concise description"
```

See [AGENTS.md](./AGENTS.md) for full architecture notes, coding conventions, and the egui pitfall list.

---

## 📜 License

Distributed under the MIT License or Apache 2.0 License at your option.
