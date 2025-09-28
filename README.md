# bevy_perf_hud

[![CI](https://github.com/ZoOLForge/bevy_perf_hud/workflows/CI/badge.svg)](https://github.com/ZoOLForge/bevy_perf_hud/actions)
[![Crates.io](https://img.shields.io/crates/v/bevy_perf_hud)](https://crates.io/crates/bevy_perf_hud)
[![Downloads](https://img.shields.io/crates/d/bevy_perf_hud)](https://crates.io/crates/bevy_perf_hud)
[![Documentation](https://docs.rs/bevy_perf_hud/badge.svg)](https://docs.rs/bevy_perf_hud)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/ZoOLForge/bevy_perf_hud#license)
[![Discord](https://img.shields.io/discord/1319490473060073532?label=Discord&logo=discord&logoColor=white)](https://discord.gg/jwyXfjUP)

![Sep-24-2025 18-37-55](https://github.com/ZoOLForge/bevy_perf_hud/raw/main/media/Sep-24-2025%2018-37-55.gif)

![Bar Scaling Modes Demo](https://github.com/ZoOLForge/bevy_perf_hud/raw/main/media/Sep-25-2025%2019-26-06.gif)

A configurable performance heads-up display (HUD) plugin for Bevy applications. Visualize frame pacing, entity counts,
and resource usage in real time, with extensibility for your own metrics.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Advanced Configuration](#advanced-configuration)
- [Built-in Metrics](#built-in-metrics)
- [Custom Metrics](#custom-metrics)
- [Examples](#examples)
- [Performance Impact](#performance-impact)
- [Troubleshooting](#troubleshooting)
- [Getting Help](#getting-help)
- [Supported Versions](#supported-versions)
- [License](#license)
- [Acknowledgements](#acknowledgements)

## Features

- Flexible HUD layout with multi-curve graphs and resource bars.
- Built-in providers for FPS, frame time, entity count, and system/process CPU & memory usage.
- Fine-grained control over smoothing, quantization, autoscaling, and appearance.
- Extensible `PerfMetricProvider` trait for custom metrics that appear alongside built-ins.

## Installation

**Minimum Supported Rust Version (MSRV)**: 1.76.0

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
bevy = { version = "0.16", default-features = false, features = [
    "bevy_winit",
    "bevy_ui",
    "bevy_render",
    "bevy_diagnostic",
    "sysinfo_plugin",
] }
bevy_perf_hud = "0.1"
```

### Feature Flags

| Feature   | Description                        | Default |
|-----------|------------------------------------|---------|
| `default` | Enables all standard functionality | ✓       |

### Requirements

- **Bevy Features**: The HUD requires `bevy_ui`, `bevy_diagnostic`, and `bevy_render` features
- **System Metrics**: Add `sysinfo_plugin` feature for CPU/memory monitoring
- **Platform Support**: Windows, macOS, Linux (system metrics may have limited functionality on some platforms)

> **Tip**: If you use `DefaultPlugins`, the required features are already enabled. Without `sysinfo_plugin`,
> system/process CPU & memory providers will be silently skipped.

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

## Advanced Configuration

`PerfHudSettings` exposes additional knobs for tailoring the HUD:

- `graph`: adjust canvas size, curve smoothing, quantization, and decide which metrics appear in the time-series chart.
- `bars`: control whether resource bars render, set per-metric min/max bounds, and decide when to show numeric values.
- `enabled` / `origin`: toggle the HUD globally and anchor it anywhere on screen.

Example: expand the graph, smooth the FPS curve, and shrink the system CPU bar range.

```rust
use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, PerfHudSettings};

fn main() {
    App::new()
        .insert_resource({
            let mut settings = PerfHudSettings::default();
            settings.origin = Vec2::new(24.0, 24.0);
            settings.graph.size = Vec2::new(360.0, 120.0);
            if let Some(fps_curve) = settings
                .graph
                .curves
                .iter_mut()
                .find(|curve| curve.metric.id == "fps")
            {
                fps_curve.smoothing = Some(0.15);
            }
            // Modify the entity count bar to use auto-scaling
            if let Some(entity_bar) = settings
                .bars
                .bars
                .iter_mut()
                .find(|bar| bar.metric.id == "entity_count")
            {
                entity_bar.scale_mode = bevy_perf_hud::BarScaleMode::Auto {
                    smoothing: 0.8,     // Smooth range transitions
                    min_span: 100.0,    // Minimum range of 100 entities
                    margin_frac: 0.2,   // 20% margin for growth headroom
                };
                entity_bar.show_value = Some(true);
            }
            settings
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
```

### Bar Scaling Modes

Performance bars can use different scaling modes to adapt their range dynamically:

#### Fixed Mode (Default)
Uses static `min_value` and `max_value` - traditional behavior with predictable, stable ranges.

```rust
use bevy_perf_hud::{BarConfig, BarScaleMode, MetricDefinition};

BarConfig {
    metric: MetricDefinition { /* ... */ },
    min_value: 0.0,
    max_value: 100.0,
    scale_mode: BarScaleMode::Fixed, // Uses min_value/max_value directly
    // ...
}
```

#### Auto Mode
Automatically adjusts range based on historical data with smooth transitions:

```rust
BarConfig {
    metric: MetricDefinition { /* ... */ },
    min_value: 0.0,   // Fallback if no data
    max_value: 100.0, // Fallback if no data
    scale_mode: BarScaleMode::Auto {
        smoothing: 0.8,     // Range change smoothing (0.0=instant, 1.0=never)
        min_span: 10.0,     // Minimum range span to prevent division by zero
        margin_frac: 0.15,  // Margin fraction above/below data (0.0-0.5)
    },
    min_limit: Some(0.0),    // Hard minimum bound (optional)
    max_limit: Some(500.0),  // Hard maximum bound (optional)
    // ...
}
```

#### Percentile Mode
Uses percentiles of recent samples - ideal for spiky data like latency:

```rust
BarConfig {
    metric: MetricDefinition { /* ... */ },
    min_value: 0.0,   // Fallback if insufficient samples
    max_value: 100.0, // Fallback if insufficient samples
    scale_mode: BarScaleMode::Percentile {
        lower: 5.0,        // P5 percentile for minimum (ignores bottom 5%)
        upper: 95.0,       // P95 percentile for maximum (ignores top 5%)
        sample_count: 60,  // Number of recent samples to analyze
    },
    min_limit: Some(0.0),    // Hard bounds prevent extreme outliers
    max_limit: Some(1000.0),
    // ...
}
```

**Use Cases:**
- **Fixed**: CPU/memory percentages, FPS with known limits
- **Auto**: Variable metrics like entity counts, memory usage in MB
- **Percentile**: Network latency, frame spikes, any metric with outliers

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

### Basic Example

```rust
use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext};

#[derive(Default)]
struct NetworkLagProvider(f32);

impl PerfMetricProvider for NetworkLagProvider {
    fn metric_id(&self) -> &str { "net/lag_ms" }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        // Simulate network latency measurement
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

### Advanced Example

Here's a more realistic example that tracks multiple game metrics:

```rust
use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext, PerfHudSettings};
use std::collections::VecDeque;

// Track active player connections
#[derive(Resource, Default)]
struct GameStats {
    active_players: u32,
    packets_per_second: VecDeque<u32>,
    last_update: f64,
}

#[derive(Default)]
struct PlayerCountProvider;

impl PerfMetricProvider for PlayerCountProvider {
    fn metric_id(&self) -> &str { "game/players" }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        ctx.world.get_resource::<GameStats>()
            .map(|stats| stats.active_players as f32)
    }
}

#[derive(Default)]
struct NetworkThroughputProvider;

impl PerfMetricProvider for NetworkThroughputProvider {
    fn metric_id(&self) -> &str { "net/packets_sec" }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        ctx.world.get_resource::<GameStats>()
            .and_then(|stats| stats.packets_per_second.back().copied())
            .map(|pps| pps as f32)
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameStats::default())
        .insert_resource({
            let mut settings = PerfHudSettings::default();
            // Add our custom metrics to the HUD display
            settings.graph.curves.push(bevy_perf_hud::GraphCurveSettings {
                metric: bevy_perf_hud::MetricSettings {
                    id: "game/players".to_string(),
                    ..default()
                },
                color: Color::srgb(0.2, 0.8, 0.2),
                ..default()
            });
            settings.bars.bars.push(bevy_perf_hud::BarSettings {
                metric: bevy_perf_hud::MetricSettings {
                    id: "net/packets_sec".to_string(),
                    ..default()
                },
                max_value: 1000.0,
                ..default()
            });
            settings
        })
        .add_plugins(bevy_perf_hud::BevyPerfHudPlugin)
        .add_perf_metric_provider(PlayerCountProvider)
        .add_perf_metric_provider(NetworkThroughputProvider)
        .add_systems(Update, update_game_stats)
        .run();
}

// System to update our custom metrics data
fn update_game_stats(
    mut stats: ResMut<GameStats>,
    time: Res<Time>,
    // Your actual game systems would provide real data
) {
    let now = time.elapsed_secs_f64();
    if now - stats.last_update > 1.0 {
        // Simulate some realistic game data
        stats.active_players = (20.0 + 10.0 * (now * 0.1).sin()) as u32;

        let pps = (500.0 + 200.0 * (now * 0.05).cos()) as u32;
        stats.packets_per_second.push_back(pps);
        if stats.packets_per_second.len() > 60 {
            stats.packets_per_second.pop_front();
        }

        stats.last_update = now;
    }
}
```

### Custom Metric Guidelines

- **Unique IDs**: Use descriptive, hierarchical names like `"game/players"` or `"net/latency_ms"`
- **Performance**: Keep `sample()` implementations fast - they're called every frame
- **Optional Values**: Return `None` when data isn't available rather than placeholder values
- **Units**: Include units in the metric ID for clarity (`_ms`, `_mb`, `_percent`)

## Examples

The repository ships with several runnable examples:

- `examples/simple.rs`: 3D scene with keyboard shortcuts (Space spawns cubes, F1 toggles HUD modes).
- `examples/custom_metric.rs`: Demonstrates registering an additional metric provider with auto-scaling.
- `examples/bar_scaling_modes.rs`: Shows all three bar scaling modes (Fixed, Auto, Percentile) in action.

Run them with:

```bash
cargo run --example simple
cargo run --example custom_metric
cargo run --example bar_scaling_modes
```

## Performance Impact

The performance HUD is designed to have minimal impact on your application:

- **CPU Usage**: ~0.1-0.5% overhead on typical applications
- **Memory Usage**: ~2-4MB for storing historical data and UI components
- **Render Cost**: UI rendering typically adds <0.1ms to frame time

**Optimization Tips**:

- Reduce `history_samples` in graph settings for lower memory usage
- Disable unused metrics by removing them from curves/bars configuration
- Use larger `update_interval` for custom metrics that are expensive to sample
- Consider disabling the HUD in release builds using feature flags

## Troubleshooting

### Common Issues

**HUD not appearing**:

- Ensure `bevy_ui` feature is enabled in your Bevy dependency
- Check that you're using `DefaultPlugins` or have added the required UI plugins manually
- Verify the HUD isn't positioned outside your window bounds

**Missing system metrics (CPU/Memory)**:

- Add the `sysinfo_plugin` feature to your Bevy dependency
- Without this feature, system/process metrics will be silently skipped

**Poor performance with many entities**:

- The `entity_count` metric can impact performance with 100k+ entities
- Consider removing it from the HUD configuration for very large worlds

**Custom metrics not updating**:

- Ensure your `PerfMetricProvider::sample()` method returns `Some(value)`
- Check that the provider is properly registered with `add_perf_metric_provider()`
- Verify the metric ID is unique and doesn't conflict with built-in metrics

### Performance Debugging

If the HUD itself is causing performance issues:

```rust
// To disable the HUD, simply remove the plugin or components
// No longer using global enabled field
```

## Getting Help

- **Issues**: Report bugs or request features on [GitHub Issues](https://github.com/ZoOLForge/bevy_perf_hud/issues)
- **Discussions**: Ask questions on [GitHub Discussions](https://github.com/ZoOLForge/bevy_perf_hud/discussions)
- **Discord**: Join our [Discord server](https://discord.gg/jwyXfjUP) for real-time help
- **Documentation**: Detailed API docs are available on [docs.rs](https://docs.rs/bevy_perf_hud)

When reporting issues, please include:

- Your Bevy version
- Operating system and version
- Minimal code example that reproduces the problem
- Console output or error messages

## Supported Versions

| bevy | bevy_perf_hud |
|------|---------------|
| 0.16 | 0.1.3         |

## License

Dual-licensed under either the MIT License or Apache License 2.0.

## Acknowledgements

- [Bevy Engine](https://bevyengine.org/) for providing the ECS/game-engine foundation.
- `bevy_diagnostic` and `SystemInformationDiagnosticsPlugin` for the metrics that power the HUD.

Looking for the Chinese documentation? See [`README_CN.md`](README_CN.md).
