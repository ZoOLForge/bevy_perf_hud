//! Core Bevy systems for the performance HUD.
//!
//! This module contains the main systems that manage the HUD lifecycle:
//! - setup_hud: Creates all UI entities and materials during startup
//! - sample_diagnostics: Updates metric values each frame
//! - update_graph_and_bars: Renders current data to the HUD display

use bevy::{
    asset::{Assets, Handle},
    diagnostic::DiagnosticsStore,
    ecs::{
        entity::Entity,
        system::{Commands, Query, Res, ResMut},
    },
    prelude::*,
    text::{TextColor, TextFont},
    ui::{FlexDirection, MaterialNode, Node, Overflow, PositionType, UiRect, Val},
};

use crate::{
    components::{BarsConfig, GraphConfig},
    constants::*,
    providers::{MetricProviders, MetricSampleContext},
    render::{BarMaterial, BarParams, MultiLineGraphMaterial, MultiLineGraphParams},
    BarsHandles, GraphHandles, GraphLabelHandle, GraphScaleState, HistoryBuffers, HudHandles,
    SampledValues,
};

/// Function that creates all HUD UI entities and materials.
/// This function is designed to be called by user code to create the HUD layout.
/// The settings are now provided as components on the entity where HUD will be spawned.
pub fn create_hud(
    mut commands: Commands,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
) {
    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Spawn root UI node with default settings as components
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(960.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GraphConfig::default(),
            BarsConfig::default(),
            HudHandles::default(),
            SampledValues::default(),
            HistoryBuffers::default(),
            GraphScaleState::default(),
            crate::BarScaleStates::default(),
        ))
        .id();
    commands.entity(root).insert(Visibility::Visible);

    // Get the graph and bars configurations from the root entity
    let graph_config = GraphConfig::default();
    let bars_config = BarsConfig::default();

    // Graph material and node (optional)
    let mut graph_row_opt: Option<Entity> = None;
    let mut graph_entity_opt: Option<Entity> = None;
    let mut graph_handle_opt: Option<Handle<MultiLineGraphMaterial>> = None;
    let mut graph_labels: Vec<GraphLabelHandle> = Vec::new();
    {
        let mut graph_params = MultiLineGraphParams::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            graph_params.length = 0;
            graph_params.min_y = graph_config.min_y;
            graph_params.max_y = graph_config.max_y;
            graph_params.thickness = graph_config.thickness;
            graph_params.bg_color = graph_config.bg_color.to_linear().to_vec4();
            graph_params.border_color = graph_config.border.color.to_linear().to_vec4();
            graph_params.border_thickness = graph_config.border.thickness; // pixels
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
                let v = c.metric.color.to_linear().to_vec4();
                graph_params.colors[i] = v;
            }
        }
        // Row container: left labels + right graph
        let label_width = graph_config.label_width.max(40.0);
        let graph_row = commands
            .spawn((Node {
                width: Val::Px(graph_config.size.x + label_width),
                height: Val::Px(graph_config.size.y),
                flex_direction: FlexDirection::Row,
                ..default()
            },))
            .id();
        commands.entity(graph_row).insert(ChildOf(root));
        commands.entity(graph_row).insert(Visibility::Visible);
        graph_row_opt = Some(graph_row);

        // Label container (vertical to avoid overlap)
        let label_container = commands
            .spawn((Node {
                width: Val::Px(label_width),
                height: Val::Px(graph_config.size.y),
                flex_direction: FlexDirection::Column,
                ..default()
            },))
            .id();
        commands.entity(label_container).insert(ChildOf(graph_row));

        // Create label rows matching configured curves
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
            graph_labels.push(crate::GraphLabelHandle {
                metric_id: curve.metric.id.clone(),
                entity: eid,
            });
        }

        // Graph node
        let gh = graph_mats.add(MultiLineGraphMaterial {
            params: graph_params,
        });
        let ge = commands
            .spawn((
                MaterialNode(gh.clone()),
                Node {
                    width: Val::Px(graph_config.size.x),
                    height: Val::Px(graph_config.size.y),
                    ..default()
                },
            ))
            .id();
        commands.entity(ge).insert(ChildOf(graph_row));
        graph_entity_opt = Some(ge);
        graph_handle_opt = Some(gh);
    }

    // Bars container placed below the graph
    let mut bars_root_opt: Option<Entity> = None;
    let mut bar_entities = Vec::new();
    let mut bar_materials = Vec::new();
    let mut bar_labels = Vec::new();
    if !bars_config.bars.is_empty() {
        let column_count = 2;
        let column_width = (graph_config.size.x - 12.0) / column_count as f32;

        let bars_root = commands
            .spawn((Node {
                width: Val::Px(graph_config.size.x),
                height: Val::Px(
                    (bars_config.bars.len() as f32 / column_count as f32).ceil() * 25.0,
                ),
                flex_direction: FlexDirection::Column,
                margin: UiRect {
                    left: Val::Px(graph_config.label_width.max(40.0)),
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
                    width: Val::Px(graph_config.size.x),
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

    // Update the Node position using the origin component - this part is tricky because Commands
    // don't allow direct access to components on the same frame they're created
    // We'll handle position updates in a separate system instead

    // Update the HudHandles component on the root entity
    commands.entity(root).insert(HudHandles {
        root: Some(root),
        graph_row: graph_row_opt,
        graph_entity: graph_entity_opt,
        graph_material: graph_handle_opt,
        graph_labels,
        graph_label_width: graph_config.label_width.max(40.0),
        bars_root: bars_root_opt,
        bar_materials,
        bar_labels,
    });
}

/// Function that creates only the graph UI entities and materials.
/// This function allows for creating the performance graph independently of bars.
pub fn create_graph_hud(
    mut commands: Commands,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
) -> Entity {
    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Spawn root UI node with default settings as components
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(960.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GraphConfig::default(),
            GraphHandles::default(),
            SampledValues::default(),
            HistoryBuffers::default(),
            GraphScaleState::default(),
        ))
        .id();
    commands.entity(root).insert(Visibility::Visible);

    // Get the graph configuration from the default
    let graph_config = GraphConfig::default();

    // Graph material and node
    let mut graph_row_opt: Option<Entity> = None;
    let mut graph_entity_opt: Option<Entity> = None;
    let mut graph_handle_opt: Option<Handle<MultiLineGraphMaterial>> = None;
    let mut graph_labels: Vec<GraphLabelHandle> = Vec::new();

    {
        let mut graph_params = MultiLineGraphParams::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            graph_params.length = 0;
            graph_params.min_y = graph_config.min_y;
            graph_params.max_y = graph_config.max_y;
            graph_params.thickness = graph_config.thickness;
            graph_params.bg_color = graph_config.bg_color.to_linear().to_vec4();
            graph_params.border_color = graph_config.border.color.to_linear().to_vec4();
            graph_params.border_thickness = graph_config.border.thickness; // pixels
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
                let v = c.metric.color.to_linear().to_vec4();
                graph_params.colors[i] = v;
            }
        }
        // Row container: left labels + right graph
        let label_width = graph_config.label_width.max(40.0);
        let graph_row = commands
            .spawn((Node {
                width: Val::Px(graph_config.size.x + label_width),
                height: Val::Px(graph_config.size.y),
                flex_direction: FlexDirection::Row,
                ..default()
            },))
            .id();
        commands.entity(graph_row).insert(ChildOf(root));
        commands.entity(graph_row).insert(Visibility::Visible);
        graph_row_opt = Some(graph_row);

        // Label container (vertical to avoid overlap)
        let label_container = commands
            .spawn((Node {
                width: Val::Px(label_width),
                height: Val::Px(graph_config.size.y),
                flex_direction: FlexDirection::Column,
                ..default()
            },))
            .id();
        commands.entity(label_container).insert(ChildOf(graph_row));

        // Create label rows matching configured curves
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
            graph_labels.push(crate::GraphLabelHandle {
                metric_id: curve.metric.id.clone(),
                entity: eid,
            });
        }

        // Graph node
        let gh = graph_mats.add(MultiLineGraphMaterial {
            params: graph_params,
        });
        let ge = commands
            .spawn((
                MaterialNode(gh.clone()),
                Node {
                    width: Val::Px(graph_config.size.x),
                    height: Val::Px(graph_config.size.y),
                    ..default()
                },
            ))
            .id();
        commands.entity(ge).insert(ChildOf(graph_row));
        graph_entity_opt = Some(ge);
        graph_handle_opt = Some(gh);
    }

    // Update the GraphHandles component on the root entity
    commands.entity(root).insert(GraphHandles {
        root: Some(root),
        graph_row: graph_row_opt,
        graph_entity: graph_entity_opt,
        graph_material: graph_handle_opt,
        graph_labels,
        graph_label_width: graph_config.label_width.max(40.0),
    });

    root
}

/// System that samples all registered metric providers and updates current values.
/// This system now runs unconditionally to collect metric data.
pub fn sample_diagnostics(
    diagnostics: Option<Res<DiagnosticsStore>>,
    mut sampled_values_query: Query<&mut SampledValues>,
    mut providers: ResMut<MetricProviders>,
) {
    let Ok(mut samples) = sampled_values_query.single_mut() else {
        return;
    };

    let ctx = MetricSampleContext {
        diagnostics: diagnostics.as_deref(),
    };

    for provider in providers.iter_mut() {
        if let Some(value) = provider.sample(ctx) {
            samples.set(provider.metric_id(), value);
        }
    }
}

/// System that updates graph and bar displays with current performance data.
/// Now uses settings stored as components on the HUD entity.
#[allow(clippy::too_many_arguments)]
pub fn update_graph_and_bars(
    mut hud_query: Query<(
        &GraphConfig,
        &BarsConfig,
        &mut HudHandles,
        &mut SampledValues,
        &mut HistoryBuffers,
        &mut GraphScaleState,
        &mut crate::BarScaleStates,
    )>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
    _label_node_q: Query<&mut Node>,
    mut label_text_q: Query<&mut Text>,
    mut label_color_q: Query<&mut TextColor>,
) {
    let Ok((
        graph_config,
        bars_config,
        h,
        samples,
        mut history,
        mut scale_state,
        mut bar_scale_states,
    )) = hud_query.single_mut()
    else {
        return;
    };

    let curve_count = graph_config.curves.len().min(MAX_CURVES);

    // Process raw metric values through smoothing and quantization pipeline
    let mut filtered_values = [0.0_f32; MAX_CURVES];
    for (i, cfg) in graph_config.curves.iter().take(curve_count).enumerate() {
        let raw = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);

        // Step 1: Apply exponential smoothing to reduce noise
        // Formula: new_value = prev_value + (raw_value - prev_value) * smoothing_factor
        let smoothing = cfg
            .smoothing
            .unwrap_or(graph_config.curve_defaults.smoothing)
            .clamp(0.0, 1.0);

        // Get the most recent value from history as the previous value
        let prev = if history.length == 0 {
            raw // No history yet, use raw value
        } else if (history.length as usize) < MAX_SAMPLES {
            history.values[i][history.length as usize - 1] // Buffer not full
        } else {
            history.values[i][MAX_SAMPLES - 1] // Buffer is full, use last element
        };

        let smoothed = prev + (raw - prev) * smoothing;

        // Step 2: Apply quantization to create cleaner stepped values
        // Rounds to the nearest multiple of quantize_step
        let step = cfg
            .quantize_step
            .unwrap_or(graph_config.curve_defaults.quantize_step);
        filtered_values[i] = if step > 0.0 {
            (smoothed / step).round() * step
        } else {
            smoothed // No quantization
        };
    }

    // Update history buffers with new values using circular buffer approach
    if (history.length as usize) < MAX_SAMPLES {
        // Buffer not yet full: append new values at the end
        let idx = history.length as usize;
        for (i, value) in filtered_values.iter().enumerate().take(MAX_CURVES) {
            history.values[i][idx] = *value;
        }
        // Pad unused curves with zeros
        for i in curve_count..MAX_CURVES {
            history.values[i][idx] = 0.0;
        }
        history.length += 1;
    } else {
        // Buffer is full: implement sliding window by shifting all values left
        // This maintains the most recent MAX_SAMPLES values for graphing
        for (i, value) in filtered_values.iter().enumerate().take(MAX_CURVES) {
            history.values[i].copy_within(1..MAX_SAMPLES, 0); // Shift left
            history.values[i][MAX_SAMPLES - 1] = *value; // Insert new value at end
        }
        // Handle unused curves with zeros
        for i in curve_count..MAX_CURVES {
            history.values[i].copy_within(1..MAX_SAMPLES, 0); // Shift left
            history.values[i][MAX_SAMPLES - 1] = 0.0; // Insert zero at end
        }
    }

    // Calculate target Y-axis range: either fixed from config or auto-scaled from data
    let mut target_min = graph_config.min_y;
    let mut target_max = graph_config.max_y;

    // Check if any curves want autoscaling and we have historical data
    if graph_config
        .curves
        .iter()
        .any(|c| c.autoscale.unwrap_or(graph_config.curve_defaults.autoscale))
        && history.length > 0
    {
        // Scan all historical data to find the actual min/max range
        let len = history.length as usize;
        let mut mn = f32::INFINITY;
        let mut mx = f32::NEG_INFINITY;

        for (i, cfg) in graph_config.curves.iter().take(curve_count).enumerate() {
            // Only include curves that want autoscaling in the calculation
            if cfg
                .autoscale
                .unwrap_or(graph_config.curve_defaults.autoscale)
            {
                for k in 0..len {
                    mn = mn.min(history.values[i][k]);
                    mx = mx.max(history.values[i][k]);
                }
            }
        }

        // Use the calculated range if it's valid
        if mn.is_finite() && mx.is_finite() {
            target_min = mn;
            target_max = mx;
        }
    }

    if graph_config.y_include_zero {
        target_min = target_min.min(0.0);
        target_max = target_max.max(0.0);
    }

    let span = (target_max - target_min)
        .abs()
        .max(graph_config.y_min_span.max(1e-3));
    if target_max - target_min < span {
        let mid = 0.5 * (target_max + target_min);
        target_min = mid - 0.5 * span;
        target_max = mid + 0.5 * span;
    }

    // Margins
    let margin_frac = graph_config.y_margin_frac.clamp(0.0, 0.45);
    let margin = span * margin_frac;
    target_min -= margin;
    target_max += margin;
    // Step quantization
    if graph_config.y_step_quantize > 0.0 {
        let step = graph_config.y_step_quantize;
        target_min = (target_min / step).floor() * step;
        target_max = (target_max / step).ceil() * step;
    }

    // Smoothing
    let a = graph_config.y_scale_smoothing.clamp(0.0, 1.0);
    if scale_state.max_y <= scale_state.min_y {
        scale_state.min_y = target_min;
        scale_state.max_y = target_max;
    } else {
        scale_state.min_y = scale_state.min_y + (target_min - scale_state.min_y) * a;
        scale_state.max_y = scale_state.max_y + (target_max - scale_state.max_y) * a;
    }

    let current_min = scale_state.min_y;
    let current_max = (scale_state.max_y).max(current_min + 1e-3);

    // Update graph labels dynamically based on configured curves
    if !h.graph_labels.is_empty() {
        for label_handle in &h.graph_labels {
            let Some(curve) = graph_config
                .curves
                .iter()
                .find(|c| c.metric.id == label_handle.metric_id)
            else {
                continue;
            };

            let definition = &curve.metric;
            let precision = definition.precision as usize;
            let unit = definition.unit.as_deref().unwrap_or("");

            let value = samples.get(curve.metric.id.as_str()).unwrap_or(0.0);
            let formatted = if precision == 0 {
                format!("{value:.0}")
            } else {
                format!("{value:.precision$}", precision = precision)
            };
            let text_value = if unit.is_empty() {
                formatted
            } else {
                format!("{formatted} {unit}")
            };

            if let Ok(mut tx) = label_text_q.get_mut(label_handle.entity) {
                if **tx != text_value {
                    **tx = text_value.clone();
                }
            }
            if let Ok(mut col) = label_color_q.get_mut(label_handle.entity) {
                *col = TextColor(curve.metric.color);
            }
        }
    }

    // Update graph material (when enabled)
    {
        if let Some(handle) = &h.graph_material {
            if let Some(mat) = graph_mats.get_mut(handle) {
                mat.params.length = history.length;
                mat.params.min_y = current_min;
                mat.params.max_y = current_max;
                mat.params.thickness = graph_config.thickness;
                mat.params.bg_color = graph_config.bg_color.to_linear().to_vec4();
                mat.params.border_color = graph_config.border.color.to_linear().to_vec4();
                mat.params.border_thickness = graph_config.border.thickness; // pixels
                mat.params.border_thickness_uv_x =
                    (graph_config.border.thickness / graph_config.size.x).max(0.0001);
                mat.params.border_thickness_uv_y =
                    (graph_config.border.thickness / graph_config.size.y).max(0.0001);
                mat.params.border_left = if graph_config.border.left { 1 } else { 0 };
                mat.params.border_bottom = if graph_config.border.bottom { 1 } else { 0 };
                mat.params.border_right = if graph_config.border.right { 1 } else { 0 };
                mat.params.border_top = if graph_config.border.top { 1 } else { 0 };
                mat.params.curve_count = curve_count as u32;
                // Sync curve colors every frame to allow hot updates
                for (i, c) in graph_config.curves.iter().take(curve_count).enumerate() {
                    mat.params.colors[i] = c.metric.color.to_linear().to_vec4();
                }
                for i in curve_count..MAX_CURVES {
                    mat.params.colors[i] = Vec4::ZERO;
                }
                // Write values (pack into vec4)
                let len = MAX_SAMPLES.min(history.length as usize);
                let packed_len = len.div_ceil(4); // round up
                for i in 0..MAX_CURVES {
                    for j in 0..SAMPLES_VEC4 {
                        let base = j * 4;
                        let x0 = if base < len {
                            history.values[i][base]
                        } else {
                            0.0
                        };
                        let x1 = if base + 1 < len {
                            history.values[i][base + 1]
                        } else {
                            0.0
                        };
                        let x2 = if base + 2 < len {
                            history.values[i][base + 2]
                        } else {
                            0.0
                        };
                        let x3 = if base + 3 < len {
                            history.values[i][base + 3]
                        } else {
                            0.0
                        };
                        mat.params.values[i][j] = Vec4::new(x0, x1, x2, x3);
                    }
                    // Optional: zero unused segments packed_len..SAMPLES_VEC4
                    for j in packed_len..SAMPLES_VEC4 {
                        mat.params.values[i][j] = Vec4::ZERO;
                    }
                }
                // Colors set at init; update here if config changed
            }
        }
    }

    // Update bars (when enabled)
    {
        for (i, cfg) in bars_config.bars.iter().enumerate() {
            if i >= h.bar_materials.len() {
                break;
            }
            let val = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);

            // Get or create the scale state for this bar
            let scale_state = bar_scale_states.get_or_create(&cfg.metric.id);

            // Add current value to the scale state's history
            scale_state.add_sample(val);

            // Calculate the dynamic range based on the bar's scale mode
            let (range_min, range_max) = scale_state.calculate_range(
                &cfg.scale_mode,
                cfg.min_value,
                cfg.max_value,
                cfg.min_limit,
                cfg.max_limit,
            );

            // Normalize the value using the calculated range
            let norm = if range_max > range_min {
                ((val - range_min) / (range_max - range_min)).clamp(0.0, 1.0)
            } else {
                0.0
            };

            if let Some(mat) = bar_mats.get_mut(&h.bar_materials[i]) {
                mat.params.value = norm;
                let v = cfg.metric.color.to_linear().to_vec4();
                mat.params.r = v.x;
                mat.params.g = v.y;
                mat.params.b = v.z;
                mat.params.a = v.w;
                let bg = bars_config.bg_color.to_linear().to_vec4();
                mat.params.bg_r = bg.x;
                mat.params.bg_g = bg.y;
                mat.params.bg_b = bg.z;
                mat.params.bg_a = bg.w;
            }

            // Update bar labels with current values and formatting
            if let Some(&label_entity) = h.bar_labels.get(i) {
                let definition = &cfg.metric;
                let base_label = definition
                    .label
                    .clone()
                    .unwrap_or_else(|| definition.id.clone());
                let precision = definition.precision as usize;
                let unit = definition.unit.as_deref().unwrap_or("");

                let formatted = if precision == 0 {
                    format!("{val:.0}")
                } else {
                    format!("{val:.precision$}", precision = precision)
                };
                let show_value = cfg.show_value.unwrap_or(bars_config.show_value_default);
                let display_text = if show_value {
                    let value_text = if unit.is_empty() {
                        formatted
                    } else {
                        format!("{formatted}{unit}")
                    };
                    format!("{} {}", base_label, value_text)
                } else {
                    base_label.clone()
                };

                if let Ok(mut tx) = label_text_q.get_mut(label_entity) {
                    if **tx != display_text {
                        **tx = display_text;
                    }
                }
                if let Ok(mut col) = label_color_q.get_mut(label_entity) {
                    *col = TextColor(Color::WHITE);
                }
            }
        }
    }
}

/// System that updates only the graph display with current performance data.
/// Uses entities with GraphConfig and GraphHandles components.
#[allow(clippy::too_many_arguments)]
pub fn update_graph(
    mut graph_query: Query<(
        &GraphConfig,
        &mut GraphHandles,
        &mut SampledValues,
        &mut HistoryBuffers,
        &mut GraphScaleState,
    )>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut label_text_q: Query<&mut Text>,
    mut label_color_q: Query<&mut TextColor>,
) {
    for (graph_config, h, samples, mut history, mut scale_state) in graph_query.iter_mut() {
        let curve_count = graph_config.curves.len().min(MAX_CURVES);

        // Process raw metric values through smoothing and quantization pipeline
        let mut filtered_values = [0.0_f32; MAX_CURVES];
        for (i, cfg) in graph_config.curves.iter().take(curve_count).enumerate() {
            let raw = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);

            // Step 1: Apply exponential smoothing to reduce noise
            let smoothing = cfg
                .smoothing
                .unwrap_or(graph_config.curve_defaults.smoothing)
                .clamp(0.0, 1.0);

            // Get the most recent value from history as the previous value
            let prev = if history.length == 0 {
                raw // No history yet, use raw value
            } else if (history.length as usize) < MAX_SAMPLES {
                history.values[i][history.length as usize - 1] // Buffer not full
            } else {
                history.values[i][MAX_SAMPLES - 1] // Buffer is full, use last element
            };

            let smoothed = prev + (raw - prev) * smoothing;

            // Step 2: Apply quantization to create cleaner stepped values
            let step = cfg
                .quantize_step
                .unwrap_or(graph_config.curve_defaults.quantize_step);
            filtered_values[i] = if step > 0.0 {
                (smoothed / step).round() * step
            } else {
                smoothed // No quantization
            };
        }

        // Update history buffers with new values using circular buffer approach
        if (history.length as usize) < MAX_SAMPLES {
            // Buffer not yet full: append new values at the end
            let idx = history.length as usize;
            for (i, value) in filtered_values.iter().enumerate().take(MAX_CURVES) {
                history.values[i][idx] = *value;
            }
            // Pad unused curves with zeros
            for i in curve_count..MAX_CURVES {
                history.values[i][idx] = 0.0;
            }
            history.length += 1;
        } else {
            // Buffer is full: implement sliding window by shifting all values left
            for (i, value) in filtered_values.iter().enumerate().take(MAX_CURVES) {
                history.values[i].copy_within(1..MAX_SAMPLES, 0); // Shift left
                history.values[i][MAX_SAMPLES - 1] = *value; // Insert new value at end
            }
            // Handle unused curves with zeros
            for i in curve_count..MAX_CURVES {
                history.values[i].copy_within(1..MAX_SAMPLES, 0); // Shift left
                history.values[i][MAX_SAMPLES - 1] = 0.0; // Insert zero at end
            }
        }

        // Calculate target Y-axis range: either fixed from config or auto-scaled from data
        let mut target_min = graph_config.min_y;
        let mut target_max = graph_config.max_y;

        // Check if any curves want autoscaling and we have historical data
        if graph_config
            .curves
            .iter()
            .any(|c| c.autoscale.unwrap_or(graph_config.curve_defaults.autoscale))
            && history.length > 0
        {
            // Scan all historical data to find the actual min/max range
            let len = history.length as usize;
            let mut mn = f32::INFINITY;
            let mut mx = f32::NEG_INFINITY;

            for (i, cfg) in graph_config.curves.iter().take(curve_count).enumerate() {
                // Only include curves that want autoscaling in the calculation
                if cfg
                    .autoscale
                    .unwrap_or(graph_config.curve_defaults.autoscale)
                {
                    for k in 0..len {
                        mn = mn.min(history.values[i][k]);
                        mx = mx.max(history.values[i][k]);
                    }
                }
            }

            // Use the calculated range if it's valid
            if mn.is_finite() && mx.is_finite() {
                target_min = mn;
                target_max = mx;
            }
        }

        if graph_config.y_include_zero {
            target_min = target_min.min(0.0);
            target_max = target_max.max(0.0);
        }

        let span = (target_max - target_min)
            .abs()
            .max(graph_config.y_min_span.max(1e-3));
        if target_max - target_min < span {
            let mid = 0.5 * (target_max + target_min);
            target_min = mid - 0.5 * span;
            target_max = mid + 0.5 * span;
        }

        // Margins
        let margin_frac = graph_config.y_margin_frac.clamp(0.0, 0.45);
        let margin = span * margin_frac;
        target_min -= margin;
        target_max += margin;
        // Step quantization
        if graph_config.y_step_quantize > 0.0 {
            let step = graph_config.y_step_quantize;
            target_min = (target_min / step).floor() * step;
            target_max = (target_max / step).ceil() * step;
        }

        // Smoothing
        let a = graph_config.y_scale_smoothing.clamp(0.0, 1.0);
        if scale_state.max_y <= scale_state.min_y {
            scale_state.min_y = target_min;
            scale_state.max_y = target_max;
        } else {
            scale_state.min_y = scale_state.min_y + (target_min - scale_state.min_y) * a;
            scale_state.max_y = scale_state.max_y + (target_max - scale_state.max_y) * a;
        }

        let current_min = scale_state.min_y;
        let current_max = (scale_state.max_y).max(current_min + 1e-3);

        // Update graph labels dynamically based on configured curves
        if !h.graph_labels.is_empty() {
            for label_handle in &h.graph_labels {
                let Some(curve) = graph_config
                    .curves
                    .iter()
                    .find(|c| c.metric.id == label_handle.metric_id)
                else {
                    continue;
                };

                let definition = &curve.metric;
                let precision = definition.precision as usize;
                let unit = definition.unit.as_deref().unwrap_or("");

                let value = samples.get(curve.metric.id.as_str()).unwrap_or(0.0);
                let formatted = if precision == 0 {
                    format!("{value:.0}")
                } else {
                    format!("{value:.precision$}", precision = precision)
                };
                let text_value = if unit.is_empty() {
                    formatted
                } else {
                    format!("{formatted} {unit}")
                };

                if let Ok(mut tx) = label_text_q.get_mut(label_handle.entity) {
                    if **tx != text_value {
                        **tx = text_value.clone();
                    }
                }
                if let Ok(mut col) = label_color_q.get_mut(label_handle.entity) {
                    *col = TextColor(curve.metric.color);
                }
            }
        }

        // Update graph material (when enabled)
        {
            if let Some(handle) = &h.graph_material {
                if let Some(mat) = graph_mats.get_mut(handle) {
                    mat.params.length = history.length;
                    mat.params.min_y = current_min;
                    mat.params.max_y = current_max;
                    mat.params.thickness = graph_config.thickness;
                    mat.params.bg_color = graph_config.bg_color.to_linear().to_vec4();
                    mat.params.border_color = graph_config.border.color.to_linear().to_vec4();
                    mat.params.border_thickness = graph_config.border.thickness; // pixels
                    mat.params.border_thickness_uv_x =
                        (graph_config.border.thickness / graph_config.size.x).max(0.0001);
                    mat.params.border_thickness_uv_y =
                        (graph_config.border.thickness / graph_config.size.y).max(0.0001);
                    mat.params.border_left = if graph_config.border.left { 1 } else { 0 };
                    mat.params.border_bottom = if graph_config.border.bottom { 1 } else { 0 };
                    mat.params.border_right = if graph_config.border.right { 1 } else { 0 };
                    mat.params.border_top = if graph_config.border.top { 1 } else { 0 };
                    mat.params.curve_count = curve_count as u32;
                    // Sync curve colors every frame to allow hot updates
                    for (i, c) in graph_config.curves.iter().take(curve_count).enumerate() {
                        mat.params.colors[i] = c.metric.color.to_linear().to_vec4();
                    }
                    for i in curve_count..MAX_CURVES {
                        mat.params.colors[i] = Vec4::ZERO;
                    }
                    // Write values (pack into vec4)
                    let len = MAX_SAMPLES.min(history.length as usize);
                    let packed_len = len.div_ceil(4); // round up
                    for i in 0..MAX_CURVES {
                        for j in 0..SAMPLES_VEC4 {
                            let base = j * 4;
                            let x0 = if base < len {
                                history.values[i][base]
                            } else {
                                0.0
                            };
                            let x1 = if base + 1 < len {
                                history.values[i][base + 1]
                            } else {
                                0.0
                            };
                            let x2 = if base + 2 < len {
                                history.values[i][base + 2]
                            } else {
                                0.0
                            };
                            let x3 = if base + 3 < len {
                                history.values[i][base + 3]
                            } else {
                                0.0
                            };
                            mat.params.values[i][j] = Vec4::new(x0, x1, x2, x3);
                        }
                        // Optional: zero unused segments packed_len..SAMPLES_VEC4
                        for j in packed_len..SAMPLES_VEC4 {
                            mat.params.values[i][j] = Vec4::ZERO;
                        }
                    }
                }
            }
        }
    }
}

/// System that updates only the bars display with current performance data.
/// Uses entities with BarsConfig and BarsHandles components.
#[allow(clippy::too_many_arguments)]
pub fn update_bars(
    mut bars_query: Query<(
        &BarsConfig,
        &mut BarsHandles,
        &mut SampledValues,
        &mut crate::BarScaleStates,
    )>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
    mut label_text_q: Query<&mut Text>,
    mut label_color_q: Query<&mut TextColor>,
) {
    for (bars_config, h, samples, mut bar_scale_states) in bars_query.iter_mut() {
        // Update bars (when enabled)
        {
            for (i, cfg) in bars_config.bars.iter().enumerate() {
                if i >= h.bar_materials.len() {
                    break;
                }
                let val = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);

                // Get or create the scale state for this bar
                let scale_state = bar_scale_states.get_or_create(&cfg.metric.id);

                // Add current value to the scale state's history
                scale_state.add_sample(val);

                // Calculate the dynamic range based on the bar's scale mode
                let (range_min, range_max) = scale_state.calculate_range(
                    &cfg.scale_mode,
                    cfg.min_value,
                    cfg.max_value,
                    cfg.min_limit,
                    cfg.max_limit,
                );

                // Normalize the value using the calculated range
                let norm = if range_max > range_min {
                    ((val - range_min) / (range_max - range_min)).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                if let Some(mat) = bar_mats.get_mut(&h.bar_materials[i]) {
                    mat.params.value = norm;
                    let v = cfg.metric.color.to_linear().to_vec4();
                    mat.params.r = v.x;
                    mat.params.g = v.y;
                    mat.params.b = v.z;
                    mat.params.a = v.w;
                    let bg = bars_config.bg_color.to_linear().to_vec4();
                    mat.params.bg_r = bg.x;
                    mat.params.bg_g = bg.y;
                    mat.params.bg_b = bg.z;
                    mat.params.bg_a = bg.w;
                }

                // Update bar labels with current values and formatting
                if let Some(&label_entity) = h.bar_labels.get(i) {
                    let definition = &cfg.metric;
                    let base_label = definition
                        .label
                        .clone()
                        .unwrap_or_else(|| definition.id.clone());
                    let precision = definition.precision as usize;
                    let unit = definition.unit.as_deref().unwrap_or("");

                    let formatted = if precision == 0 {
                        format!("{val:.0}")
                    } else {
                        format!("{val:.precision$}", precision = precision)
                    };
                    let show_value = cfg.show_value.unwrap_or(bars_config.show_value_default);
                    let display_text = if show_value {
                        let value_text = if unit.is_empty() {
                            formatted
                        } else {
                            format!("{formatted}{unit}")
                        };
                        format!("{} {}", base_label, value_text)
                    } else {
                        base_label.clone()
                    };

                    if let Ok(mut tx) = label_text_q.get_mut(label_entity) {
                        if **tx != display_text {
                            **tx = display_text;
                        }
                    }
                    if let Ok(mut col) = label_color_q.get_mut(label_entity) {
                        *col = TextColor(Color::WHITE);
                    }
                }
            }
        }
    }
}


