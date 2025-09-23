use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, Settings, CurveConfig, BarConfig, PerfKey};


fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Settings {
            enabled: true,
            origin: Vec2::new(16.0, 16.0),
            graph_size: Vec2::new(720.0, 180.0),
            graph_min_y: 0.0,
            graph_max_y: 30.0,
            graph_thickness: 0.008,
            curves: vec![
                // CurveConfig { key: PerfKey::FrameTimeMs, color: Color::srgb(0.0, 1.0, 0.0), autoscale: true },
                CurveConfig { key: PerfKey::Fps, color: Color::srgb(0.0, 1.0, 1.0), autoscale: true },
            ],
            bars: vec![
                BarConfig { key: PerfKey::CpuLoad, label: "CPU".into(), color: Color::srgb(1.0, 0.3, 0.0) },
                BarConfig { key: PerfKey::GpuLoad, label: "GPU".into(), color: Color::srgb(0.0, 0.0, 1.0) },
                BarConfig { key: PerfKey::NetLoad, label: "NET".into(), color: Color::srgb(0.0, 1.0, 0.0) },
            ],
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "bevy_perf_hud demo".into(), resolution: (1280., 720.).into(), ..default() }),
            ..default()
        }))
        .add_plugins(BevyPerfHudPlugin)
        .run();
}