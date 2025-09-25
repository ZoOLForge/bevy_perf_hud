use bevy::prelude::*;
use bevy_perf_hud::{
    BarConfig, BarScaleMode, BevyPerfHudPlugin, CurveConfig, MetricDefinition, MetricSampleContext,
    PerfHudAppExt, PerfHudSettings, PerfMetricProvider,
};

const CUSTOM_METRIC_ID: &str = "custom/network_latency_ms";

/// Simulated network latency metric provider
struct NetworkLatencyMetric {
    seed: u64,
    current_ms: f32,
}

impl Default for NetworkLatencyMetric {
    fn default() -> Self {
        Self {
            seed: 0x1234_5678_9ABC_DEF0,
            current_ms: 48.0,
        }
    }
}

impl NetworkLatencyMetric {
    /// Lightweight LCG random in [0, 1)
    fn next_noise(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let bits = (self.seed >> 32) as u32;
        (bits as f32) / (u32::MAX as f32)
    }
}

impl PerfMetricProvider for NetworkLatencyMetric {
    fn metric_id(&self) -> &str {
        CUSTOM_METRIC_ID
    }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        // Simulate baseline latency plus jitter
        let noise = (self.next_noise() - 0.5) * 30.0; // ±15ms jitter
        let target = 30.0 + noise.max(-45.0); // ~30ms base latency
        self.current_ms = self.current_ms + (target - self.current_ms) * 0.2;
        Some(self.current_ms.max(0.0))
    }
}

fn main() {
    // Extend default HUD with network latency metric
    let latency_metric = MetricDefinition {
        id: CUSTOM_METRIC_ID.into(),
        label: Some("Latency".into()),
        unit: Some("ms".into()),
        precision: 1,
        color: Color::srgb(0.65, 0.11, 0.0),
    };

    let mut settings = PerfHudSettings {
        origin: Vec2::new(16.0, 16.0),
        ..Default::default()
    };

    settings.graph.max_y = 160.0;
    settings.graph.curves.push(CurveConfig {
        metric: latency_metric.clone(),
        autoscale: Some(false),
        smoothing: Some(0.25),
        quantize_step: Some(0.5),
    });

    // Add custom latency metric with percentile scaling
    // Percentile mode is perfect for latency as it handles spikes gracefully
    settings.bars.bars.insert(
        0,
        BarConfig {
            metric: latency_metric.clone(),
            show_value: Some(true),
            min_value: 0.0,   // Fallback minimum
            max_value: 200.0, // Fallback maximum
            scale_mode: BarScaleMode::Percentile {
                lower: 10.0,       // P10 - ignore bottom 10% of samples
                upper: 95.0,       // P95 - ignore top 5% spikes (outliers)
                sample_count: 180, // 3 seconds of samples for good statistics
            },
            min_limit: Some(0.0),    // Hard minimum (latency can't be negative)
            max_limit: Some(1000.0), // Hard maximum (cap extreme outliers)
        },
    );

    // Also modify the entity count bar to use auto-scaling for better demo
    if let Some(entity_bar) = settings
        .bars
        .bars
        .iter_mut()
        .find(|bar| bar.metric.id == "entity_count")
    {
        entity_bar.scale_mode = BarScaleMode::Auto {
            smoothing: 0.7,   // Moderately smooth transitions
            min_span: 50.0,   // Minimum range of 50 entities
            margin_frac: 0.2, // 20% margin for headroom
        };
        entity_bar.min_limit = Some(0.0); // Can't be negative
        entity_bar.max_limit = Some(100000.0); // Reasonable upper bound
        entity_bar.show_value = Some(true); // Show actual count
    }

    App::new()
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.05, 1.0)))
        .insert_resource(settings)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_perf_hud custom metric".into(),
                resolution: (1200., 600.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BevyPerfHudPlugin)
        .add_systems(Startup, setup_scene)
        .add_perf_metric_provider(NetworkLatencyMetric::default())
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
}
