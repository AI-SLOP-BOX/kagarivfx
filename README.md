# Kagari VFX

うんち!!

Open-source CLI-first VFX & compositing engine.
Built in Rust with GPU acceleration, headless rendering, and automation-friendly tooling for AI agents.

Kagari is designed to handle the VFX work that code-first video tools eventually outgrow — compositing, effects, tracking, particles, masks, 3D, and more — without requiring a proprietary editor.

---

## Quick start

```bash
git clone https://github.com/AI-SLOP-BOX/kagarivfx.git
cd kagarivfx
cargo run --release --features gui --bin kagari-studio
```

## CLI

```bash
kagari render project.kagari --comp main --frame 120 --output frame.exr
kagari effect add --layer 1 --effect gaussian-blur --radius 20
kagari keyframe set --layer 1 --property opacity --frame 0 --value 0
```

## What it is

Kagari VFX is a node-based compositor and motion graphics engine written in Rust.
It exports MP4, ProRes, GIF, Lottie, and MLT XML, and can also run entirely headless for scripting or AI-agent automation.

## Stack

- **Rust + wgpu** — GPU compositing with Metal backend on macOS
- **egui** — dark professional UI
- **FFmpeg** — decode / encode pipeline
- **Rhai** — embedded expression engine for animatable properties
- **rayon** — parallel CPU effects

## License

MIT OR Apache-2.0. Do whatever you want.
