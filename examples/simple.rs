use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, Settings, CurveConfig, BarConfig, PerfKey};


fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Settings {
            enabled: true,
            origin: Vec2::new(16.0, 16.0),
            graph: bevy_perf_hud::GraphSettings {
                enabled: true,
                size: Vec2::new(720.0, 180.0),
                min_y: 0.0,
                max_y: 30.0,
                thickness: 0.012,
                bg_color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                // Y 轴比例控制：包含 0、最小跨度与边距、步进量化与平滑
                y_include_zero: true,
                y_min_span: 5.0,
                y_margin_frac: 0.10,
                y_step_quantize: 5.0,
                y_scale_smoothing: 0.3,
                curves: vec![
                    CurveConfig { key: PerfKey::FrameTimeMs, color: Color::srgb(0.0, 1.0, 0.0), autoscale: true, smoothing: 0.25, quantize_step: 0.1 },
                    CurveConfig { key: PerfKey::Fps, color: Color::srgb(0.9, 0.0, 0.0), autoscale: true, smoothing: 0.2, quantize_step: 1.0 },
                ],
            },
            bars: bevy_perf_hud::BarsSettings {
                enabled: true,
                bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
                bars: vec![
                    BarConfig { key: PerfKey::CpuLoad, label: "CPU".into(), color: Color::srgb(1.0, 0.3, 0.0) },
                    BarConfig { key: PerfKey::GpuLoad, label: "GPU".into(), color: Color::srgb(0.0, 0.0, 1.0) },
                    BarConfig { key: PerfKey::NetLoad, label: "NET".into(), color: Color::srgb(0.0, 1.0, 0.0) },
                ],
            },
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "bevy_perf_hud demo".into(), resolution: (1280., 720.).into(), ..default() }),
            ..default()
        }))
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
