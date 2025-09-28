use bevy::prelude::*;
use bevy_perf_hud::{BarConfig, BarMaterial, BarParams, BarScaleMode, BarScaleStates, BarsConfig, BarsHandles, BevyPerfHudPlugin, MetricDefinition, MetricSampleContext, PerfHudAppExt, PerfMetricProvider, SampledValues};

/// Demonstrates different bar scaling modes for dynamic range adjustment
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgba(0.02, 0.02, 0.05, 1.0)))
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .add_systems(Startup, setup_bars_hud)
        .add_perf_metric_provider(VariableMetric::new("variable/cpu_load", 0.0, 100.0))
        .add_perf_metric_provider(VariableMetric::new("variable/memory_usage", 100.0, 2000.0))
        .add_perf_metric_provider(SpikyMetric::new("spiky/latency", 10.0, 500.0))
        .add_systems(Update, simulate_input)
        .run();
}

/// System wrapper to create bars-only HUD
fn setup_bars_hud(mut commands: Commands, mut bar_mats: ResMut<Assets<BarMaterial>>) {
    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Get the bars configuration from the default and customize it
    let mut bars_config = BarsConfig::default();
    
    // Configure custom bars with different scaling modes for the demo
    let fixed_mode_metric = MetricDefinition {
        id: "variable/cpu_load".into(),
        label: Some("CPU (Fixed 0-100%)".into()),
        unit: Some("%".into()),
        precision: 1,
        color: Color::srgb(1.0, 0.3, 0.3),
    };

    let auto_mode_metric = MetricDefinition {
        id: "variable/memory_usage".into(),
        label: Some("Memory (Auto)".into()),
        unit: Some("MB".into()),
        precision: 0,
        color: Color::srgb(0.3, 1.0, 0.3),
    };

    let percentile_mode_metric = MetricDefinition {
        id: "spiky/latency".into(),
        label: Some("Latency (P5-P95)".into()),
        unit: Some("ms".into()),
        precision: 1,
        color: Color::srgb(0.3, 0.3, 1.0),
    };

    // Configure bars with different scaling modes
    bars_config.bars = vec![
        // Fixed mode bar - traditional static range
        BarConfig {
            metric: fixed_mode_metric,
            show_value: Some(true),
            min_value: 0.0,
            max_value: 100.0,
            scale_mode: BarScaleMode::Fixed,
            min_limit: None,
            max_limit: None,
        },
        // Auto mode bar - adapts to data range with smoothing
        BarConfig {
            metric: auto_mode_metric,
            show_value: Some(true),
            min_value: 0.0,    // Used as fallback if no data
            max_value: 1000.0, // Used as fallback if no data
            scale_mode: BarScaleMode::Auto {
                smoothing: 0.8,   // Smooth transitions (0.0 = instant, 1.0 = never change)
                min_span: 100.0,  // Minimum range span
                margin_frac: 0.1, // 10% margin above and below data range
            },
            min_limit: Some(0.0),    // Hard minimum limit
            max_limit: Some(2500.0), // Hard maximum limit
        },
        // Percentile mode bar - uses P5 to P95 range, good for spiky data
        BarConfig {
            metric: percentile_mode_metric,
            show_value: Some(true),
            min_value: 0.0,   // Used as fallback if insufficient data
            max_value: 200.0, // Used as fallback if insufficient data
            scale_mode: BarScaleMode::Percentile {
                lower: 5.0,       // P5 percentile for minimum
                upper: 95.0,      // P95 percentile for maximum
                sample_count: 60, // Use last 60 samples (~1 second at 60fps)
            },
            min_limit: Some(0.0),    // Hard minimum limit
            max_limit: Some(1000.0), // Hard maximum limit
        },
    ];

    // Spawn root UI node with customized settings as components
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            bars_config.clone(),  // Use the customized config instead of default
            BarsHandles::default(),
            SampledValues::default(),
            BarScaleStates::default(),
        ))
        .id();
    commands.entity(root).insert(Visibility::Visible);

    // Bars container
    let mut bars_root_opt: Option<Entity> = None;
    let mut bar_entities = Vec::new();
    let mut bar_materials = Vec::new();
    let mut bar_labels = Vec::new();

    if !bars_config.bars.is_empty() {
        let column_count = 2;
        let default_width = 300.0; // Use a default width for bars-only layout
        let column_width = (default_width - 12.0) / column_count as f32;

        let bars_root = commands
            .spawn((Node {
                width: Val::Px(default_width),
                height: Val::Px(
                    (bars_config.bars.len() as f32 / column_count as f32).ceil() * 25.0,
                ),
                flex_direction: FlexDirection::Column,
                margin: UiRect {
                    left: Val::Px(0.0), // No left margin for bars-only layout
                    top: Val::Px(4.0),
                    ..default()
                },
                ..default()
            },))
            .id();
        commands.entity(bars_root).insert(ChildOf(root));
        commands
            .entity(bars_root)
            .insert(if !bars_config.bars.is_empty() {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
        bars_root_opt = Some(bars_root);

        for chunk in bars_config.bars.chunks(column_count) {
            let row = commands
                .spawn((Node {
                    width: Val::Px(default_width),
                    height: Val::Px(24.0),
                    flex_direction: FlexDirection::Row,
                    margin: UiRect {
                        top: Val::Px(1.0),
                        ..default()
                    },
                    ..default()
                },))
                .id();
            commands.entity(row).insert(ChildOf(bars_root));

            for (col_idx, bar_cfg) in chunk.iter().enumerate() {
                let base_label = bar_cfg
                    .metric
                    .label
                    .clone()
                    .unwrap_or_else(|| bar_cfg.metric.id.clone());

                let column = commands
                    .spawn((Node {
                        width: Val::Px(column_width),
                        height: Val::Px(24.0),
                        margin: UiRect {
                            right: if col_idx + 1 == column_count || col_idx + 1 == chunk.len() {
                                Val::Px(0.0)
                            } else {
                                Val::Px(8.0)
                            },
                            ..default()
                        },
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },))
                    .id();
                commands.entity(column).insert(ChildOf(row));

                let mat = bar_mats.add(BarMaterial {
                    params: BarParams {
                        value: 0.0,
                        r: bar_cfg.metric.color.to_linear().to_vec4().x,
                        g: bar_cfg.metric.color.to_linear().to_vec4().y,
                        b: bar_cfg.metric.color.to_linear().to_vec4().z,
                        a: bar_cfg.metric.color.to_linear().to_vec4().w,
                        bg_r: bars_config.bg_color.to_linear().to_vec4().x,
                        bg_g: bars_config.bg_color.to_linear().to_vec4().y,
                        bg_b: bars_config.bg_color.to_linear().to_vec4().z,
                        bg_a: bars_config.bg_color.to_linear().to_vec4().w,
                    },
                });

                let bar_entity = commands
                    .spawn((
                        MaterialNode(mat.clone()),
                        Node {
                            width: Val::Px(column_width),
                            height: Val::Px(20.0),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(bar_entity).insert(ChildOf(column));

                let bar_label = commands
                    .spawn((
                        Text::new(base_label),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(6.0),
                            top: Val::Px(5.0),
                            width: Val::Px(column_width - 12.0),
                            overflow: Overflow::hidden(),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(bar_label).insert(ChildOf(bar_entity));

                bar_entities.push(bar_entity);
                bar_materials.push(mat);
                bar_labels.push(bar_label);
            }
        }
    }

    // Update the BarsHandles component on the root entity
    commands.entity(root).insert(BarsHandles {
        bars_root: bars_root_opt,
        bar_materials,
        bar_labels,
    });
}



/// Simulates keyboard input for controlling the demo
fn simulate_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    bars_config_query: Query<Entity, With<BarsConfig>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        if let Ok(entity) = bars_config_query.single() {
            // Toggle bars visibility by removing/adding the component
            commands.entity(entity).remove::<BarsConfig>();
        }
    }
}

/// A variable metric that changes gradually over time
struct VariableMetric {
    id: String,
    time: f32,
    min_value: f32,
    max_value: f32,
    current: f32,
}

impl VariableMetric {
    fn new(id: &str, min_value: f32, max_value: f32) -> Self {
        Self {
            id: id.to_string(),
            time: 0.0,
            min_value,
            max_value,
            current: (min_value + max_value) * 0.5,
        }
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
}

/// A metric that has occasional spikes, good for demonstrating percentile scaling
struct SpikyMetric {
    id: String,
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
            time: 0.0,
            spike_timer: 0.0,
            min_value,
            max_value,
            base_value: min_value,
        }
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
}
