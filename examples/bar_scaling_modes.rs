use bevy::prelude::*;
use bevy_perf_hud::{
    BarConfig, BarsContainer, BarsHandles, BevyPerfHudPlugin, MetricSampleContext, PerfHudAppExt,
    PerfMetricProvider,
};

/// Demonstrates different bar scaling modes for dynamic range adjustment
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.05, 1.0)))
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .add_systems(Startup, setup_bars_hud)
        .add_perf_metric_provider(
            VariableMetric::new("variable/cpu_load", 0.0, 100.0)
                .with_label("CPU (Fixed 0-100%)")
                .with_unit("%")
                .with_precision(1)
                .with_color(Color::srgb(1.0, 0.3, 0.3)),
        )
        .add_perf_metric_provider(
            VariableMetric::new("variable/memory_usage", 100.0, 2000.0)
                .with_label("Memory (Auto)")
                .with_unit("MB")
                .with_precision(0)
                .with_color(Color::srgb(0.3, 1.0, 0.3)),
        )
        .add_perf_metric_provider(
            SpikyMetric::new("spiky/latency", 10.0, 500.0)
                .with_label("Latency (P5-P95)")
                .with_unit("ms")
                .with_precision(1)
                .with_color(Color::srgb(0.3, 0.3, 1.0)),
        )
        .add_systems(Update, simulate_input)
        .run();
}

/// System wrapper to create bars-only HUD
fn setup_bars_hud(mut commands: Commands) {
    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Create BarConfig instances with different scaling modes
    let bar_configs = vec![
        // Fixed mode bar - traditional static range
        // Using default fallback (0.0-100.0) is sufficient for most cases
        BarConfig::fixed_mode("variable/cpu_load"),

        // Auto mode bar - adapts to data range with smoothing
        // Default fallback works well as initial range before data is collected
        BarConfig::auto_mode("variable/memory_usage"),

        // Percentile mode bar - uses P5 to P95 range, good for spiky data
        // Default fallback provides a reasonable starting point
        BarConfig::percentile_mode("spiky/latency"),

        // If you need custom fallback values, use the _with_fallback variants:
        // BarConfig::fixed_mode_with_fallback("custom_metric", 0.0, 1000.0),
        // BarConfig::auto_mode_with_fallback("custom_metric", 0.0, 10000.0),
        // BarConfig::percentile_mode_with_fallback("custom_metric", 0.0, 500.0),
    ];

    // Spawn individual BarConfig entities for each bar
    for bar_config in &bar_configs {
        commands.spawn(bar_config.clone());
    }

    // Create BarsContainer with layout configuration
    // The initialize_bars_ui system will automatically create all child UI entities
    let bars_container = BarsContainer {
        column_count: 2,
        width: 300.0,
        row_height: 24.0,
    };

    let bars_width = bars_container.width;
    let row_height = bars_container.row_height;
    let total_height =
        (bar_configs.len() as f32 / bars_container.column_count as f32).ceil() * row_height;

    // Spawn root UI node with BarsContainer
    //  automatically includes: BarsHandles, BarMaterials, SampledValues, BarScaleStates
    // The initialize_bars_ui system will populate BarsHandles and BarMaterials with actual entities
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Px(20.0),
            width: Val::Px(bars_width),
            height: Val::Px(total_height),
            flex_direction: FlexDirection::Column,
            margin: UiRect {
                left: Val::Px(0.0), // No left margin for bars-only layout
                top: Val::Px(4.0),
                ..default()
            },
            ..default()
        },
        bars_container,
    ));
}

/// Simulates keyboard input for controlling the demo
fn simulate_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut bars_handles_query: Query<&mut Visibility, With<BarsHandles>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Ok(mut visibility) = bars_handles_query.single_mut() {
            *visibility = if *visibility == Visibility::Visible {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
    }
}

/// A variable metric that changes gradually over time
#[derive(Clone)]
struct VariableMetric {
    id: String,
    label: Option<String>,
    unit: Option<String>,
    color: Color,
    precision: u32,
    time: f32,
    min_value: f32,
    max_value: f32,
    current: f32,
}

impl VariableMetric {
    fn new(id: &str, min_value: f32, max_value: f32) -> Self {
        Self {
            id: id.to_string(),
            label: None,
            unit: None,
            color: Color::srgb(1.0, 1.0, 1.0),
            precision: 1,
            time: 0.0,
            min_value,
            max_value,
            current: (min_value + max_value) * 0.5,
        }
    }

    fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    fn with_precision(mut self, precision: u32) -> Self {
        self.precision = precision;
        self
    }
}

impl PerfMetricProvider for VariableMetric {
    fn metric_id(&self) -> &str {
        &self.id
    }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        // Use a simple time increment since we don't have delta_time
        self.time += 0.016; // Assume ~60 FPS

        // Create a slowly varying sine wave with some noise
        let base_wave = (self.time * 0.3).sin() * 0.5 + 0.5; // 0.0 to 1.0
        let noise = ((self.time * 2.7).sin() * 0.1 + (self.time * 4.1).cos() * 0.05) * 0.5 + 0.5;
        let combined = (base_wave * 0.8 + noise * 0.2).clamp(0.0, 1.0);

        self.current = self.min_value + combined * (self.max_value - self.min_value);

        // Add some gradual drift
        let drift_speed = if self.id.contains("cpu") { 0.5 } else { 2.0 };
        let target_drift = (self.time * 0.1).sin() * 0.3 + 0.5; // 0.2 to 0.8
        let target = self.min_value + target_drift * (self.max_value - self.min_value);
        self.current = self.current * 0.95 + target * 0.05 * drift_speed * 0.016;

        Some(self.current)
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn unit(&self) -> Option<String> {
        self.unit.clone()
    }

    fn precision(&self) -> u32 {
        self.precision
    }

    fn color(&self) -> Color {
        self.color
    }
}

/// A metric that has occasional spikes, good for demonstrating percentile scaling
#[derive(Clone)]
struct SpikyMetric {
    id: String,
    label: Option<String>,
    unit: Option<String>,
    color: Color,
    precision: u32,
    time: f32,
    spike_timer: f32,
    min_value: f32,
    max_value: f32,
    base_value: f32,
}

impl SpikyMetric {
    fn new(id: &str, min_value: f32, max_value: f32) -> Self {
        Self {
            id: id.to_string(),
            label: None,
            unit: None,
            color: Color::srgb(1.0, 1.0, 1.0),
            precision: 1,
            time: 0.0,
            spike_timer: 0.0,
            min_value,
            max_value,
            base_value: min_value,
        }
    }

    fn with_label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    fn with_unit(mut self, unit: &str) -> Self {
        self.unit = Some(unit.to_string());
        self
    }

    fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    fn with_precision(mut self, precision: u32) -> Self {
        self.precision = precision;
        self
    }
}

impl PerfMetricProvider for SpikyMetric {
    fn metric_id(&self) -> &str {
        &self.id
    }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        // Use a simple time increment since we don't have delta_time
        let delta_time = 0.016; // Assume ~60 FPS
        self.time += delta_time;
        self.spike_timer -= delta_time;

        // Base level that varies slowly
        self.base_value = self.min_value
            + ((self.time * 0.2).sin() * 0.5 + 0.5) * (self.max_value - self.min_value) * 0.3;

        // Trigger random spikes
        if self.spike_timer <= 0.0 {
            // Random spike every 1-3 seconds
            self.spike_timer = 1.0 + (self.time.sin() * 0.5 + 0.5) * 2.0;
        }

        // Generate spike if we're in spike period
        let spike_intensity = if self.spike_timer > 2.5 {
            // In spike - exponential decay
            let spike_progress = (3.0 - self.spike_timer) / 0.5; // 0 to 1 over 0.5 seconds
            (1.0 - (-spike_progress * 3.0).exp()) * (0.5 + (self.time * 7.3).sin().abs() * 0.5)
        // Vary spike intensity
        } else {
            0.0
        };

        let spike_value = self.max_value * spike_intensity;
        let result = self.base_value + spike_value;

        Some(result.clamp(self.min_value, self.max_value))
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn unit(&self) -> Option<String> {
        self.unit.clone()
    }

    fn precision(&self) -> u32 {
        self.precision
    }

    fn color(&self) -> Color {
        self.color
    }
}
