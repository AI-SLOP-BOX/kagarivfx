# Hikari Studio (光)

[![Rust CI](https://github.com/kme20988-wq/aevfx/actions/workflows/ci.yml/badge.svg)](https://github.com/kme20988-wq/aevfx/actions)
[![License: MIT / Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust%20🦀-orange.svg)](https://www.rust-lang.org/)

**Hikari Studio (光)** is a high-performance, open-source Motion Graphics & Visual Effects compositor built with **Rust**, **wgpu (Metal/Vulkan)**, and a modern dark **egui** studio interface. Featuring 32bpc float HDR color science, real-time GPU simulation VFX shaders, and dynamic custom WGSL shader hot-reloading.

---

> [!IMPORTANT]
> **Experimental Status**: This project is a community-driven open-source research engine provided **"AS-IS"** without warranties. Issues and PRs are warmly welcome!

---

## ✨ Key Features & Architecture

### 🚀 High-Performance Rendering & Color Science
- **32bpc Float (Scene-Linear HDR) Pipeline**: Unlimited dynamic range with zero highlight clipping, ACES 1.3 Gamut Compression, and Reinhard tone mapping.
- **Dual Pipeline Compositor**: Rayon-parallelized 64-bit multi-core CPU software compositor with instant GPU preview (`wgpu 22` on Metal/Vulkan/DirectX 12).
- **Real-Time GPU Simulation VFX Shaders**: Procedural Fractal Noise (GPU fBM), Turbulent Displace, Wave Warp, Twirl, Bulge, Spherize, Heat Distortion, Rain Ripples, Bloom/Glow, and Lens Flare.
- **Dynamic Custom WGSL Shader Plugin**: Live code editing with real-time **naga** syntax validation and external `.wgsl` file hot-reloading.

### 🎨 Typography & Motion Graphics
- **Create Shapes from Text**: Decomposes typography into animatable vector Bézier shape paths with inner-hole counter support and zero external font license dependencies.
- **Cinema-Style 3D Extrusion**: Real-time watertight solid mesh generation with customizable front/back caps and bevel styles (Linear, Convex, Concave).
- **Comprehensive Keyframing**: Bezier curve interpolation with 19 ease presets, spatial motion paths, and graph editor velocity curve adjustment.
- **Rhai Expression Engine**: High-speed, sandboxed expression evaluation supporting `wiggle()`, `loopOut()`, `smooth()`, seed randoms, and dynamic scope injection (`thisComp`, `thisLayer`).

### 🔍 VFX Tools & Tracking
- **50+ Built-in Effect Processors**: Gaussian Blur, Advanced Glow, Directional/Radial Blur, Displacement Map, Turbulent Displace, Set Matte, Matte Choker, Hue/Saturation, Curves, Lumetri WB, CRT Glitch, and more.
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

### Build & Run GUI Studio
```bash
# Launch Hikari Studio GUI editor
cargo run --release --features gui --bin hikari-studio
```

### Headless CLI Rendering
```bash
# Render a single frame from project JSON to PNG
cargo run --release --features cli --bin hikari -- frame --project my_project.json --frame 0 --output /tmp/frame_000.png

# Render a frame sequence or MP4 video
cargo run --release --features cli --bin hikari -- render --project my_project.json --out-dir /tmp/renders/
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

## 📊 Feature Completeness Overview

| Feature Area | Status | Notes |
|---|---|---|
| **Keyframe Interpolation** | ✅ Complete | 19 ease presets, Bezier, Hold, Linear, Graph Editor |
| **Expression Engine** | ✅ Complete | Rhai-based: wiggle, loopOut, smooth, time, thisComp |
| **GPU Real-time Preview** | ✅ Complete | wgpu Metal/Vulkan/DX12, adaptive resolution |
| **32bpc / 16bpc HDR Pipeline** | ✅ Complete | Scene-linear float, ACES 1.3, TPDF dithering, instant UI toggle |
| **Real-time GPU Shader Effects** | ✅ Complete | WGSL shaders: Glow, Levels, Hue/Sat, Grain, Motion Blur, Flare, Corner Pin, Layer Styles |
| **CPU Ground Truth VFX Suite** | ✅ Complete | 47+ Rayon-parallel CPU kernels for pixel-perfect deterministic export |
| **3D Extrusion & Bevel** | ✅ Complete | Solid mesh generation, ray-traced soft shadows |
| **Motion Tracker** | ✅ Complete | SAD + planar homography, subpixel refinement |
| **Audio Engine** | ✅ Complete | Multi-track WAV, Mute/Solo, EQ, Compressor |
| **Particle System** | ✅ Complete | Deterministic emission, GPU textures, force fields |
| **Puppet Tool (MLS)** | ✅ Complete | ARAP mesh deformation, bone rigs |
| **32bit HDR Paint** | ✅ Complete | Brush, Eraser, Clone Stamp |
| **MP4 / ProRes Export** | ✅ Complete | via FFmpeg bridge |
| **Lottie Export** | ✅ Complete | Bodymovin-compatible JSON |
| **Motion Sketch** | ✅ Complete | Freehand gesture → keyframe baking |
| **AI Features** | ⚡ Opt-in | Modular bridge: SAM, RIFE, Depth-Anything, ProPainter |

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

---

## ⚖️ Trademark Disclaimer

**Hikari Studio (光)** is an independent, community-driven open-source software project. It is **not** affiliated with, sponsored by, endorsed by, or in any way associated with Adobe Inc. or its subsidiaries. "Adobe", "After Effects", and other product names or logos are trademarks or registered trademarks of their respective owners.
