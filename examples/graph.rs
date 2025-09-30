//! Graph Example
//!
//! Demonstrates performance graph (time-series chart) features:
//! - Different curve configurations (autoscale, smoothing, quantization)
//! - Custom metric providers with various data patterns
//! - Manual graph UI setup
//!
//! Controls:
//! - Space: Toggle graph visibility
//!
//! The example shows three curves:
//! 1. Wave (red): Smooth sine wave with autoscale and moderate smoothing
//! 2. Noise (green): Random noisy data with heavy smoothing to demonstrate smoothing effect
//! 3. Step (blue): Step changes with quantization to show discrete value snapping

use bevy::prelude::*;
use bevy_perf_hud::{
    BevyPerfHudPlugin, CurveConfig, CurveDefaults, GraphBorder, GraphConfig, GraphContainer,
    GraphHandles, MetricDefinition, MetricRegistry, MetricSampleContext,
    PerfHudAppExt, PerfMetricProvider, ProviderRegistry,
};
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.05, 1.0)))
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .add_systems(Startup, setup_graph_hud)
        .add_perf_metric_provider(
            WaveMetric::new("wave/smooth", 10.0, 50.0, 0.5)
                .with_label("Wave (Smooth)")
                .with_color(Color::srgb(1.0, 0.2, 0.3)), // Bright red
        )
        .add_perf_metric_provider(
            NoiseMetric::new("noise/raw", 0.0, 30.0)
                .with_label("Noise (Raw)")
                .with_color(Color::srgb(0.2, 1.0, 0.3)), // Bright green
        )
        .add_perf_metric_provider(
            StepMetric::new("step/quantized", 0.0, 100.0)
                .with_label("Step (Quantized)")
                .with_color(Color::srgb(0.3, 0.7, 1.0)), // Bright cyan-blue
        )
        .add_systems(Update, toggle_visibility)
        .run();
}

/// System to create a graph-only HUD
fn setup_graph_hud(
    mut commands: Commands,
    provider_registry: Res<ProviderRegistry>,
    mut metric_registry: ResMut<MetricRegistry>,
) {
    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Register metric definitions with MetricRegistry so update_graph can find the colors
    // This is necessary because update_graph system queries MetricRegistry for colors
    for metric_id in ["wave/smooth", "noise/raw", "step/quantized"] {
        if let Some(display_config) = provider_registry.get_display_config(metric_id) {
            metric_registry.register(MetricDefinition {
                id: metric_id.to_string(),
                label: display_config.label.clone(),
                unit: display_config.unit.clone(),
                precision: display_config.precision,
                color: display_config.color,
            });
        }
    }

    // Spawn CurveConfig entities for each curve
    // Wave with autoscale and moderate smoothing
    commands.spawn(CurveConfig {
        metric_id: "wave/smooth".into(),
        autoscale: Some(true),
        smoothing: Some(0.3),
        quantize_step: Some(1.0),
    });

    // Noise with heavy smoothing to show smoothing effect
    commands.spawn(CurveConfig {
        metric_id: "noise/raw".into(),
        autoscale: Some(true),
        smoothing: Some(0.8), // Heavy smoothing for noisy data
        quantize_step: None,
    });

    // Step with quantization to show discrete values
    commands.spawn(CurveConfig {
        metric_id: "step/quantized".into(),
        autoscale: Some(false), // Fixed range
        smoothing: Some(0.1), // Minimal smoothing
        quantize_step: Some(10.0), // Snap to multiples of 10
    });

    // Configure graph appearance and behavior
    let graph_config = GraphConfig {
        size: Vec2::new(400.0, 120.0),
        label_width: 100.0,
        min_y: 0.0,
        max_y: 60.0,
        thickness: 0.015,
        curve_defaults: CurveDefaults {
            autoscale: true,
            smoothing: 0.2,
            quantize_step: 1.0,
        },
        bg_color: Color::srgba(0.0, 0.0, 0.0, 0.3),
        border: GraphBorder {
            color: Color::srgba(1.0, 1.0, 1.0, 1.0),
            thickness: 2.0,
            left: true,
            bottom: true,
            right: false,
            top: false,
        },
        y_ticks: 3,
        y_include_zero: true,
        y_min_span: 10.0,
        y_margin_frac: 0.15,
        y_step_quantize: 10.0,
        y_scale_smoothing: 0.3,
    };

    // Create GraphContainer - automatically includes GraphHandles, GraphConfig,
    // HistoryBuffers, GraphScaleState, SampledValues, and Visibility
    let graph_container = GraphContainer {
        size: graph_config.size,
        label_width: graph_config.label_width,
    };

    // Spawn root UI node with GraphContainer
    // The initialize_graph_ui system will automatically create all child UI entities
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        graph_config,
        graph_container,
    ));
}

/// Toggle graph visibility with Space key
fn toggle_visibility(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut graph_query: Query<&mut Visibility, With<GraphHandles>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Ok(mut visibility) = graph_query.single_mut() {
            *visibility = if *visibility == Visibility::Visible {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
    }
}

// ============================================================================
// Custom Metric Providers
// ============================================================================

/// A metric that produces smooth sine wave data
#[derive(Clone)]
struct WaveMetric {
    id: String,
    label: Option<String>,
    color: Color,
    time: f32,
    min_value: f32,
    max_value: f32,
    frequency: f32,
}

impl WaveMetric {
    fn new(id: &str, min_value: f32, max_value: f32, frequency: f32) -> Self {
        Self {
            id: id.to_string(),
            label: None,
            color: Color::srgb(1.0, 1.0, 1.0),
            time: 0.0,
            min_value,
            max_value,
            frequency,
        }
    }

    fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl PerfMetricProvider for WaveMetric {
    fn metric_id(&self) -> &str {
        &self.id
    }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        self.time += 0.016; // Assume ~60 FPS
        let wave = (self.time * self.frequency).sin() * 0.5 + 0.5; // 0.0 to 1.0
        Some(self.min_value + wave * (self.max_value - self.min_value))
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn color(&self) -> Color {
        self.color
    }
}

/// A metric that produces random noisy data (good for demonstrating smoothing)
#[derive(Clone)]
struct NoiseMetric {
    id: String,
    label: Option<String>,
    color: Color,
    time: f32,
    min_value: f32,
    max_value: f32,
    state: u64,
}

impl NoiseMetric {
    fn new(id: &str, min_value: f32, max_value: f32) -> Self {
        Self {
            id: id.to_string(),
            label: None,
            color: Color::srgb(1.0, 1.0, 1.0),
            time: 0.0,
            min_value,
            max_value,
            state: 0x9E3779B97F4A7C15, // LCG seed
        }
    }

    fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    fn next_random(&mut self) -> f32 {
        // Simple LCG
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let v = self.state >> 40;
        (v as f32) / ((1u32 << 24) as f32)
    }
}

impl PerfMetricProvider for NoiseMetric {
    fn metric_id(&self) -> &str {
        &self.id
    }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        self.time += 0.016;

        // Mix base wave with heavy noise
        let base = (self.time * 0.3).sin() * 0.5 + 0.5;
        let noise = self.next_random();
        let combined = base * 0.3 + noise * 0.7; // Heavy noise

        Some(self.min_value + combined * (self.max_value - self.min_value))
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn color(&self) -> Color {
        self.color
    }
}

/// A metric that produces step changes (good for demonstrating quantization)
#[derive(Clone)]
struct StepMetric {
    id: String,
    label: Option<String>,
    color: Color,
    time: f32,
    step_timer: f32,
    current_value: f32,
    min_value: f32,
    max_value: f32,
    state: u64,
}

impl StepMetric {
    fn new(id: &str, min_value: f32, max_value: f32) -> Self {
        Self {
            id: id.to_string(),
            label: None,
            color: Color::srgb(1.0, 1.0, 1.0),
            time: 0.0,
            step_timer: 0.0,
            current_value: (min_value + max_value) * 0.5,
            min_value,
            max_value,
            state: 0x9E3779B97F4A7C15,
        }
    }

    fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    fn next_random(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let v = self.state >> 40;
        (v as f32) / ((1u32 << 24) as f32)
    }
}

impl PerfMetricProvider for StepMetric {
    fn metric_id(&self) -> &str {
        &self.id
    }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        self.time += 0.016;
        self.step_timer -= 0.016;

        // Change value every 1-2 seconds
        if self.step_timer <= 0.0 {
            self.step_timer = 1.0 + self.next_random() * 1.0;
            self.current_value = self.min_value + self.next_random() * (self.max_value - self.min_value);
        }

        Some(self.current_value)
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn color(&self) -> Color {
        self.color
    }
}