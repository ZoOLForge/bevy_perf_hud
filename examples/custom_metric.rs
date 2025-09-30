use bevy::prelude::*;
use bevy_perf_hud::{
    BarConfig, BevyPerfHudPlugin, MetricDefinition, MetricSampleContext,
    PerfHudAppExt, PerfMetricProvider, MetricRegistry,
    BarMaterial, BarParams, BarMaterials, BarsContainer, BarsHandles,
    GraphConfig, GraphHandles, GraphLabelHandle, HistoryBuffers, GraphScaleState,
    MultiLineGraphMaterial, MultiLineGraphParams, CurveConfig, HudHandles, MAX_CURVES
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

fn setup_hud(
    mut commands: Commands,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut metric_registry: ResMut<MetricRegistry>,
) {
    // Define custom network latency metric
    let latency_metric = MetricDefinition {
        id: CUSTOM_METRIC_ID.into(),
        label: Some("Latency".into()),
        unit: Some("ms".into()),
        precision: 1,
        color: Color::srgb(0.65, 0.11, 0.0),
    };

    // Register the metric definition
    metric_registry.register(latency_metric.clone());

    // Create root HUD entity with custom graph configuration
    let mut graph_config = GraphConfig::default();
    graph_config.max_y = 160.0;
    graph_config.curves.push(CurveConfig {
        metric_id: latency_metric.id.clone(),
        autoscale: Some(false),
        smoothing: Some(0.25),
        quantize_step: Some(0.5),
    });

    // BarsContainer brings in: BarsHandles, BarMaterials, SampledValues, BarScaleStates
    let bars_container = BarsContainer {
        column_count: 2,
        width: 300.0,
        row_height: 24.0,
    };

    // Cache layout values before moving bars_container
    let column_count = bars_container.column_count;
    let bars_width = bars_container.width;
    let row_height = bars_container.row_height;

    let hud_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            graph_config.clone(),
            HudHandles::default(),
            GraphHandles::default(),
            HistoryBuffers::default(),
            GraphScaleState::default(),
            bars_container, // BarsContainer automatically brings SampledValues
        ))
        .id();
    commands.entity(hud_root).insert(Visibility::Visible);

    // Create graph UI
    let mut graph_params = MultiLineGraphParams::default();
    graph_params.length = 0;
    graph_params.min_y = graph_config.min_y;
    graph_params.max_y = graph_config.max_y;
    graph_params.thickness = graph_config.thickness;
    graph_params.bg_color = graph_config.bg_color.to_linear().to_vec4();
    graph_params.border_color = graph_config.border.color.to_linear().to_vec4();
    graph_params.border_thickness = graph_config.border.thickness;
    graph_params.border_thickness_uv_x =
        (graph_config.border.thickness / graph_config.size.x).max(0.0001);
    graph_params.border_thickness_uv_y =
        (graph_config.border.thickness / graph_config.size.y).max(0.0001);
    graph_params.border_left = if graph_config.border.left { 1 } else { 0 };
    graph_params.border_bottom = if graph_config.border.bottom { 1 } else { 0 };
    graph_params.border_right = if graph_config.border.right { 1 } else { 0 };
    graph_params.border_top = if graph_config.border.top { 1 } else { 0 };
    graph_params.curve_count = graph_config.curves.len().min(MAX_CURVES) as u32;

    // Write curve colors
    for (i, c) in graph_config.curves.iter().take(MAX_CURVES).enumerate() {
        let v = if let Some(metric_def) = metric_registry.get(&c.metric_id) {
            metric_def.color.to_linear().to_vec4()
        } else {
            Color::WHITE.to_linear().to_vec4()
        };
        graph_params.colors[i] = v;
    }

    // Create graph row container
    let label_width = graph_config.label_width.max(40.0);
    let graph_row = commands
        .spawn((Node {
            width: Val::Px(graph_config.size.x + label_width),
            height: Val::Px(graph_config.size.y),
            flex_direction: FlexDirection::Row,
            ..default()
        },))
        .id();
    commands.entity(graph_row).insert(ChildOf(hud_root));
    commands.entity(graph_row).insert(Visibility::Visible);

    // Create label container
    let label_container = commands
        .spawn((Node {
            width: Val::Px(label_width),
            height: Val::Px(graph_config.size.y),
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .id();
    commands.entity(label_container).insert(ChildOf(graph_row));

    // Create graph labels
    let mut graph_labels: Vec<GraphLabelHandle> = Vec::new();
    for curve in graph_config.curves.iter().take(MAX_CURVES) {
        let eid = commands
            .spawn((
                Text::new(""),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                Node {
                    width: Val::Px(label_width),
                    height: Val::Px(16.0),
                    ..default()
                },
            ))
            .id();
        commands.entity(eid).insert(ChildOf(label_container));
        graph_labels.push(GraphLabelHandle {
            metric_id: curve.metric_id.clone(),
            entity: eid,
        });
    }

    // Create graph material and entity
    let graph_material = graph_mats.add(MultiLineGraphMaterial {
        params: graph_params,
    });
    let graph_entity = commands
        .spawn((
            MaterialNode(graph_material.clone()),
            Node {
                width: Val::Px(graph_config.size.x),
                height: Val::Px(graph_config.size.y),
                ..default()
            },
        ))
        .id();
    commands.entity(graph_entity).insert(ChildOf(graph_row));

    // Update GraphHandles
    commands.entity(hud_root).insert(GraphHandles {
        root: Some(hud_root),
        graph_row: Some(graph_row),
        graph_entity: Some(graph_entity),
        graph_material: Some(graph_material.clone()),
        graph_labels: graph_labels.clone(),
        graph_label_width: label_width,
    });

    // Get entity count metric from registry
    let entity_count_metric = metric_registry.get("entity_count").cloned().unwrap();

    // Configure bars with different scaling modes using helper methods
    let bar_configs_and_metrics = vec![
        // Latency - percentile mode to handle spikes
        (
            BarConfig::percentile_mode(CUSTOM_METRIC_ID, 0.0, 200.0),
            latency_metric.clone()
        ),
        // Entity count - auto mode for dynamic range
        (
            BarConfig::auto_mode("entity_count", 0.0, 10000.0),
            entity_count_metric.clone()
        ),
    ];

    // Spawn individual BarConfig entities for each bar
    for (bar_config, metric_def) in &bar_configs_and_metrics {
        commands.spawn((
            bar_config.clone(),
            metric_def.clone(),
        ));
    }

    // Calculate layout dimensions from cached values
    let column_width = (bars_width - 12.0) / column_count as f32;
    let total_height = (bar_configs_and_metrics.len() as f32 / column_count as f32).ceil() * row_height;

    // Create bars root container below the graph (plain Node, not BarsContainer)
    let bars_root = commands
        .spawn(Node {
            width: Val::Px(bars_width),
            height: Val::Px(total_height),
            flex_direction: FlexDirection::Column,
            margin: UiRect {
                top: Val::Px(4.0),
                ..default()
            },
            ..default()
        })
        .id();
    commands.entity(bars_root).insert(ChildOf(hud_root));

    // Create bar materials and labels for each bar configuration
    let mut bar_materials: Vec<Handle<BarMaterial>> = Vec::new();
    let mut bar_labels: Vec<Entity> = Vec::new();

    for (_chunk_index, chunk) in bar_configs_and_metrics.chunks(column_count).enumerate() {
        let row = commands
            .spawn((Node {
                width: Val::Px(bars_width),
                height: Val::Px(row_height),
                flex_direction: FlexDirection::Row,
                margin: UiRect {
                    top: Val::Px(1.0),
                    ..default()
                },
                ..default()
            },))
            .id();
        commands.entity(row).insert(ChildOf(bars_root));

        for (col_idx, (bar_config, metric_definition)) in chunk.iter().enumerate() {
            // Create column container
            let column = commands
                .spawn((Node {
                    width: Val::Px(column_width),
                    height: Val::Px(row_height),
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

            // Create bar material
            let color = metric_definition.color;
            let mat = bar_mats.add(BarMaterial {
                params: BarParams {
                    value: 0.0,
                    r: color.to_linear().to_vec4().x,
                    g: color.to_linear().to_vec4().y,
                    b: color.to_linear().to_vec4().z,
                    a: color.to_linear().to_vec4().w,
                    bg_r: bar_config.bg_color.to_linear().to_vec4().x,
                    bg_g: bar_config.bg_color.to_linear().to_vec4().y,
                    bg_b: bar_config.bg_color.to_linear().to_vec4().z,
                    bg_a: bar_config.bg_color.to_linear().to_vec4().w,
                },
            });

            // Create bar entity
            let bar_entity = commands
                .spawn((
                    MaterialNode(mat.clone()),
                    Node {
                        width: Val::Px(column_width),
                        height: Val::Px(row_height - 4.0),
                        ..default()
                    },
                ))
                .id();
            commands.entity(bar_entity).insert(ChildOf(column));

            // Create bar label
            let base_label = metric_definition
                .label
                .clone()
                .unwrap_or_else(|| bar_config.metric_id.clone());
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

            bar_materials.push(mat);
            bar_labels.push(bar_label);
        }
    }

    // Update the BarsHandles component (auto-created by BarsContainer on hud_root)
    commands.entity(hud_root).insert(BarsHandles {
        bars_root: Some(bars_root),
        bar_labels: bar_labels.clone(),
    });

    // Update the BarMaterials component (auto-created by BarsContainer on hud_root)
    commands.entity(hud_root).insert(BarMaterials {
        materials: bar_materials.clone(),
    });

    // Update HudHandles on hud_root
    commands.entity(hud_root).insert(HudHandles {
        root: Some(hud_root),
        graph_row: Some(graph_row),
        graph_entity: Some(graph_entity),
        graph_material: Some(graph_material),
        graph_labels,
        graph_label_width: label_width,
        bars_root: Some(bars_root),
        bar_materials,
        bar_labels,
    });
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
        .add_systems(Startup, setup_hud) // Create HUD with custom bars
        .add_perf_metric_provider(NetworkLatencyMetric::default())
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
}
