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
│   │   └── ...
│   └── ui/                     # egui dark-mode studio panels (Timeline, Viewport, Graph, Scopes)
└── tests/                      # Extensive fuzzing, stress tests & shader validation
```

---

## 📜 License

Distributed under the MIT License or Apache 2.0 License at your option.
