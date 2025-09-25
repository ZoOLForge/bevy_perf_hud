# bevy_perf_hud

[![CI](https://github.com/ZoOLForge/bevy_perf_hud/workflows/CI/badge.svg)](https://github.com/ZoOLForge/bevy_perf_hud/actions)
[![Crates.io](https://img.shields.io/crates/v/bevy_perf_hud)](https://crates.io/crates/bevy_perf_hud)
[![Downloads](https://img.shields.io/crates/d/bevy_perf_hud)](https://crates.io/crates/bevy_perf_hud)
[![Documentation](https://docs.rs/bevy_perf_hud/badge.svg)](https://docs.rs/bevy_perf_hud)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/Seldom-SE/seldom_pixel#license)
[![Discord](https://img.shields.io/discord/1319490473060073532?label=Discord&logo=discord&logoColor=white)](https://discord.gg/jwyXfjUP)

![Sep-24-2025 18-37-55](https://github.com/ZoOLForge/bevy_perf_hud/raw/main/media/Sep-24-2025%2018-37-55.gif)

A configurable performance heads-up display (HUD) plugin for Bevy applications. Visualize frame pacing, entity counts,
and resource usage in real time, with extensibility for your own metrics.

## Features

- Flexible HUD layout with multi-curve graphs and resource bars.
- Built-in providers for FPS, frame time, entity count, and system/process CPU & memory usage.
- Fine-grained control over smoothing, quantization, autoscaling, and appearance.
- Extensible `PerfMetricProvider` trait for custom metrics that appear alongside built-ins.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
bevy = { version = "0.16", default-features = false, features = ["bevy_winit", "bevy_ui", "bevy_render"] }
bevy_perf_hud = "0.1"
```

> Tip: If you rely on `DefaultPlugins`, ensure `bevy_diagnostic` and `bevy_ui` features are enabled so the HUD can
> gather data and render correctly.

## Quick Start

```rust
use bevy::prelude::*;
use bevy_perf_hud::BevyPerfHudPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
```

By default the HUD appears near the top-right corner. To reposition or customize the layout, insert a `PerfHudSettings`
resource before adding the plugin:

```rust
use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, PerfHudSettings};

fn main() {
    App::new()
        .insert_resource(PerfHudSettings {
            origin: Vec2::new(32.0, 32.0),
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
```

## Built-in Metrics

| Metric ID           | Description                                    |
|---------------------|------------------------------------------------|
| `fps`               | Frames per second (floored to an integer).     |
| `frame_time_ms`     | Smoothed frame time in milliseconds.           |
| `entity_count`      | Active entity count in the `World`.            |
| `system/cpu_usage`  | Overall system CPU usage percentage.           |
| `system/mem_usage`  | Overall system memory usage percentage.        |
| `process/cpu_usage` | CPU usage of the running process.              |
| `process/mem_usage` | Memory footprint of the running process (MiB). |

## Custom Metrics

Implement the `PerfMetricProvider` trait and register it with the `PerfHudAppExt` helper:

```rust
use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext};

#[derive(Default)]
struct NetworkLagProvider(f32);

impl PerfMetricProvider for NetworkLagProvider {
    fn metric_id(&self) -> &str { "net/lag_ms" }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        self.0 = (self.0 + 1.0) % 120.0;
        Some(self.0)
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_perf_hud::BevyPerfHudPlugin)
        .add_perf_metric_provider(NetworkLagProvider::default())
        .run();
}
```

## Examples

The repository ships with two runnable examples:

- `examples/simple.rs`: 3D scene with keyboard shortcuts (Space spawns cubes, F1 toggles HUD modes).
- `examples/custom_metric.rs`: Demonstrates registering an additional metric provider.

Run them with:

```bash
cargo run --example simple
cargo run --example custom_metric
```

## Supported Versions

| bevy | bevy_perf_hud |
|------|---------------|
| 0.16 | 0.1           |

## License

Dual-licensed under either the MIT License or Apache License 2.0.

## Acknowledgements

- [Bevy Engine](https://bevyengine.org/) for providing the ECS/game-engine foundation.
- `bevy_diagnostic` and `SystemInformationDiagnosticsPlugin` for the metrics that power the HUD.

Looking for the Chinese documentation? See [`README_CN.md`](README_CN.md).
