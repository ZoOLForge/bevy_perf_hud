use bevy::prelude::*;
use bevy_perf_hud::{
    BarConfig, BarScaleMode, BevyPerfHudPlugin, CurveConfig, MetricDefinition, MetricSampleContext,
    PerfHudAppExt, PerfMetricProvider, create_hud, GraphConfig, MetricRegistry
};

const CUSTOM_METRIC_ID: &str = "custom/network_latency_ms";

/// Simulated network latency metric provider
#[derive(Clone)]
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

fn apply_custom_metric_config(
    mut graph_config_query: Query<&mut GraphConfig>,
    mut commands: Commands,
    mut metric_registry: ResMut<MetricRegistry>,
) {
    // Extend default HUD with network latency metric
    let latency_metric = MetricDefinition {
        id: CUSTOM_METRIC_ID.into(),
        label: Some("Latency".into()),
        unit: Some("ms".into()),
        precision: 1,
        color: Color::srgb(0.65, 0.11, 0.0),
    };

    // Register the metric definition
    metric_registry.register(latency_metric.clone());

    // Spawn the metric definition as component
    commands.spawn(latency_metric.clone());

    let Ok(mut graph_config) = graph_config_query.single_mut() else {
        return;
    };

    // Update graph settings
    graph_config.max_y = 160.0;
    graph_config.curves.push(CurveConfig {
        metric_id: latency_metric.id.clone(),
        autoscale: Some(false),
        smoothing: Some(0.25),
        quantize_step: Some(0.5),
    });

    // Add a separate entity for the custom latency bar configuration with percentile scaling
    // Percentile mode is perfect for latency as it handles spikes gracefully
    commands.spawn((
        BarConfig {
            metric_id: latency_metric.id.clone(),
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
            bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6), // Default background color
        },
        latency_metric.clone(), // Also attach the metric definition to the same entity
    ));

    // To modify the entity count bar, we need to spawn a new BarConfig with updated values
    // Since there's no direct access to the existing bar config, we'll add a modified version
    if let Some(entity_count_metric) = metric_registry.get("entity_count").cloned() {
        commands.spawn((
            BarConfig {
                metric_id: "entity_count".into(),
                show_value: Some(true), // Show actual count
                min_value: 0.0,
                max_value: 10000.0, // Entity count range - fallback values
                scale_mode: BarScaleMode::Auto {
                    smoothing: 0.7,   // Moderately smooth transitions
                    min_span: 50.0,   // Minimum range of 50 entities
                    margin_frac: 0.2, // 20% margin for headroom
                },
                min_limit: Some(0.0),     // Entities can't be negative
                max_limit: Some(100000.0), // Reasonable upper bound
                bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6), // Default background color
            },
            entity_count_metric, // Get the existing entity count metric definition
        ));
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.05, 1.0)))
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
        .add_systems(Startup, create_hud) // Create HUD layout
        .add_systems(Startup, apply_custom_metric_config.after(create_hud)) // Apply custom configurations
        .add_perf_metric_provider(NetworkLatencyMetric::default())
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
}
