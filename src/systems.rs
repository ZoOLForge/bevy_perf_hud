//! Core Bevy systems for the performance HUD.
//!
//! This module contains the main systems that manage the HUD lifecycle:
//! - setup_hud: Creates all UI entities and materials during startup
//! - sample_diagnostics: Updates metric values each frame
//! - update_graph: Renders graph display with current data
//! - update_bars: Renders bar display with current data

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
    bar_components::{BarConfig, BarMaterials, BarsContainer, BarsHandles},
    components::{MetricRegistry, SampledValues},
    constants::*,
    graph_components::{GraphConfig, GraphHandles, GraphLabelHandle, GraphScaleState, HistoryBuffers},
    providers::{MetricProviders, MetricSampleContext, ProviderRegistry},
    render::{BarMaterial, BarParams, MultiLineGraphMaterial, MultiLineGraphParams},
};


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


/// System that updates only the graph display with current performance data.
/// Uses entities with GraphConfig and GraphHandles components.
/// Queries CurveConfig entities to get curve configurations.
#[allow(clippy::too_many_arguments)]
pub fn update_graph(
    mut graph_query: Query<(
        &GraphConfig,
        &mut GraphHandles,
        &mut SampledValues,
        &mut HistoryBuffers,
        &mut GraphScaleState,
    )>,
    curve_config_query: Query<&crate::CurveConfig>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut label_text_q: Query<&mut Text>,
    mut label_color_q: Query<&mut TextColor>,
    metric_registry: Res<MetricRegistry>,
) {
    for (graph_config, h, samples, mut history, mut scale_state) in graph_query.iter_mut() {
        // Collect all curve configurations
        let curve_configs: Vec<crate::CurveConfig> =
            curve_config_query.iter().map(|cfg| cfg.clone()).collect();
        let curve_count = curve_configs.len().min(MAX_CURVES);

        // Process raw metric values through smoothing and quantization pipeline
        let mut filtered_values = [0.0_f32; MAX_CURVES];
        for (i, cfg) in curve_configs.iter().take(curve_count).enumerate() {
            let raw = samples.get(cfg.metric_id.as_str()).unwrap_or(0.0);

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
        if curve_configs
            .iter()
            .any(|c| c.autoscale.unwrap_or(graph_config.curve_defaults.autoscale))
            && history.length > 0
        {
            // Scan all historical data to find the actual min/max range
            let len = history.length as usize;
            let mut mn = f32::INFINITY;
            let mut mx = f32::NEG_INFINITY;

            for (i, cfg) in curve_configs.iter().take(curve_count).enumerate() {
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
                let Some(curve) = curve_configs
                    .iter()
                    .find(|c| c.metric_id == label_handle.metric_id)
                else {
                    continue;
                };

                let definition = metric_registry.get(&curve.metric_id);
                let precision = definition.map(|d| d.precision).unwrap_or(2) as usize;
                let unit = definition.and_then(|d| d.unit.as_deref()).unwrap_or("");

                let value = samples.get(curve.metric_id.as_str()).unwrap_or(0.0);
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
                    if let Some(def) = definition {
                        *col = TextColor(def.color);
                    }
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
                    for (i, c) in curve_configs.iter().take(curve_count).enumerate() {
                        if let Some(metric_def) = metric_registry.get(&c.metric_id) {
                            mat.params.colors[i] = metric_def.color.to_linear().to_vec4();
                        } else {
                            mat.params.colors[i] = bevy::color::Color::WHITE.to_linear().to_vec4();
                        }
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

/// System that creates UI elements for bar configs when needed.
/// This system dynamically creates bar materials and labels for each BarConfig component.
#[allow(clippy::too_many_arguments)]
pub fn create_bar_ui_elements(
    _commands: Commands,
    _bar_config_query: Query<(Entity, &BarConfig), Changed<BarConfig>>,
    _bars_handles_query: Query<&mut BarsHandles>,
    _bar_mats: ResMut<Assets<BarMaterial>>,
    _provider_registry: Res<ProviderRegistry>,
) {
    // Placeholder for a future implementation
    // This system would handle dynamic creation of bar UI elements
    // For now, bar UI elements are created in create_hud function or initialize_bars_ui
}

/// System that updates only the bars display with current performance data.
/// Uses entities with BarConfig and BarsHandles components.
/// Assumes UI elements have already been created by create_hud function.
#[allow(clippy::too_many_arguments)]
pub fn update_bars(
    bar_config_query: Query<&BarConfig>,
    mut bars_handles_query: Query<&mut BarsHandles>,
    mut bar_materials_query: Query<&mut BarMaterials>,
    mut sampled_values_query: Query<&mut SampledValues>,
    mut bar_scale_states_query: Query<&mut crate::BarScaleStates>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
    mut label_text_q: Query<&mut Text>,
    mut label_color_q: Query<&mut TextColor>,
    provider_registry: Res<ProviderRegistry>,
) {
    // Get global resources/components that are shared across all bars
    let Ok(samples) = sampled_values_query.single_mut() else {
        return;
    };
    let Ok(mut bar_scale_states) = bar_scale_states_query.single_mut() else {
        return;
    };
    let Ok(h) = bars_handles_query.single_mut() else {
        return;
    };
    let Ok(materials) = bar_materials_query.single_mut() else {
        return;
    };

    // Update bars (when enabled)
    let mut bar_index = 0;
    for bar_config in bar_config_query.iter() {
        if bar_index >= materials.len() {
            break;
        }

        // Get display config from provider registry
        let display_config = provider_registry.get_display_config(&bar_config.metric_id);

        let val = samples.get(&bar_config.metric_id).unwrap_or(0.0);

        // Get or create the scale state for this bar
        let scale_state = bar_scale_states.get_or_create(&bar_config.metric_id);

        // Add current value to the scale state's history
        scale_state.add_sample(val);

        // Calculate the dynamic range based on the bar's scale mode
        let (range_min, range_max) = scale_state.calculate_range(
            &bar_config.scale_mode,
            bar_config.min_value,
            bar_config.max_value,
            bar_config.min_limit,
            bar_config.max_limit,
        );

        // Normalize the value using the calculated range
        let norm = if range_max > range_min {
            ((val - range_min) / (range_max - range_min)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        if let Some(mat) = bar_mats.get_mut(&materials[bar_index]) {
            mat.params.value = norm;
            // Use color from provider or default white
            let color = display_config
                .map(|c| c.color)
                .unwrap_or(Color::srgb(1.0, 1.0, 1.0));
            let v = color.to_linear().to_vec4();
            mat.params.r = v.x;
            mat.params.g = v.y;
            mat.params.b = v.z;
            mat.params.a = v.w;
            let bg = bar_config.bg_color.to_linear().to_vec4();
            mat.params.bg_r = bg.x;
            mat.params.bg_g = bg.y;
            mat.params.bg_b = bg.z;
            mat.params.bg_a = bg.w;
        }

        // Update bar labels with current values and formatting
        if let Some(&label_entity) = h.bar_labels.get(bar_index) {
            let base_label = display_config
                .and_then(|c| c.label.clone())
                .unwrap_or_else(|| bar_config.metric_id.clone());
            let precision = display_config.map(|c| c.precision).unwrap_or(1) as usize;
            let unit = display_config.and_then(|c| c.unit.as_ref()).map(|s| s.as_str()).unwrap_or("");

            let formatted = if precision == 0 {
                format!("{val:.0}")
            } else {
                format!("{val:.precision$}", precision = precision)
            };
            let show_value = bar_config.show_value.unwrap_or(true);
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

        bar_index += 1;
    }
}

/// System that automatically creates bar UI entities when a BarsContainer is added.
/// This eliminates the need for manual UI hierarchy creation in setup functions.
///
/// Queries for newly added BarsContainer components and all BarConfig + MetricDefinition entities,
/// then generates the complete UI hierarchy (rows → columns → bars → labels) based on the
/// BarsContainer layout configuration.
///
/// If the entity has a BarsHandles component with a bars_root set, bars will be created as children
/// of that bars_root. Otherwise, bars will be created as direct children of the BarsContainer entity.
pub fn initialize_bars_ui(
    mut commands: Commands,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
    bars_container_query: Query<(Entity, &BarsContainer, Option<&BarsHandles>), Added<BarsContainer>>,
    bar_config_query: Query<&BarConfig>,
    provider_registry: Res<ProviderRegistry>,
) {
    for (container_entity, bars_container, bars_handles_opt) in bars_container_query.iter() {
        // Collect all bar configurations
        let bar_configs: Vec<BarConfig> = bar_config_query
            .iter()
            .map(|cfg| cfg.clone())
            .collect();

        if bar_configs.is_empty() {
            continue;
        }

        // Extract layout configuration
        let column_count = bars_container.column_count;
        let bars_width = bars_container.width;
        let row_height = bars_container.row_height;
        let column_width = (bars_width - 12.0) / column_count as f32;

        // Determine the parent entity for bar rows:
        // If there's a bars_root in BarsHandles, use it; otherwise use the container itself
        let bars_parent = bars_handles_opt
            .and_then(|h| h.bars_root)
            .unwrap_or(container_entity);

        // Create bar materials and labels for each bar configuration
        let mut bar_materials: Vec<Handle<BarMaterial>> = Vec::new();
        let mut bar_labels: Vec<Entity> = Vec::new();

        for chunk in bar_configs.chunks(column_count) {
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
            commands.entity(row).insert(ChildOf(bars_parent));

            for (col_idx, bar_config) in chunk.iter().enumerate() {
                // Get display config from provider registry
                let display_config = provider_registry
                    .get_display_config(&bar_config.metric_id);
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

                // Create bar material using color from provider or default white
                let color = display_config
                    .map(|c| c.color)
                    .unwrap_or(Color::srgb(1.0, 1.0, 1.0));
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

                // Create bar label using label from provider or metric ID
                let base_label = display_config
                    .and_then(|c| c.label.clone())
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

        // Update the BarsHandles component (auto-created by BarsContainer)
        commands.entity(container_entity).insert(BarsHandles {
            bars_root: None,
            bar_labels: bar_labels.clone(),
        });

        // Update the BarMaterials component (auto-created by BarsContainer)
        commands.entity(container_entity).insert(BarMaterials {
            materials: bar_materials.clone(),
        });
    }
}



/// System that creates UI elements for graph when GraphContainer is added.
/// Similar to initialize_bars_ui but for graphs.
/// Queries all CurveConfig entities and creates UI for them.
pub fn initialize_graph_ui(
    mut commands: Commands,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    graph_container_query: Query<
        (Entity, &crate::GraphContainer, Option<&GraphHandles>),
        Added<crate::GraphContainer>,
    >,
    curve_config_query: Query<&crate::CurveConfig>,
    metric_registry: Res<MetricRegistry>,
) {
    for (container_entity, graph_container, graph_handles_opt) in graph_container_query.iter() {
        // Collect all curve configurations
        let curve_configs: Vec<crate::CurveConfig> =
            curve_config_query.iter().map(|cfg| cfg.clone()).collect();

        if curve_configs.is_empty() {
            continue;
        }

        // Get the graph configuration from the container entity if it exists
        // Otherwise use default
        let graph_config = crate::GraphConfig::default();

        // Determine the parent entity for graph UI:
        // If there's a root in GraphHandles, use it; otherwise use the container itself
        let graph_parent = graph_handles_opt
            .and_then(|h| h.root)
            .unwrap_or(container_entity);

        // Create graph material and UI
        let mut graph_params = MultiLineGraphParams::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            graph_params.length = 0;
            graph_params.min_y = graph_config.min_y;
            graph_params.max_y = graph_config.max_y;
            graph_params.thickness = graph_config.thickness;
            graph_params.bg_color = graph_config.bg_color.to_linear().to_vec4();
            graph_params.border_color = graph_config.border.color.to_linear().to_vec4();
            graph_params.border_thickness = graph_config.border.thickness;
            graph_params.border_thickness_uv_x =
                (graph_config.border.thickness / graph_container.size.x).max(0.0001);
            graph_params.border_thickness_uv_y =
                (graph_config.border.thickness / graph_container.size.y).max(0.0001);
            graph_params.border_left = if graph_config.border.left { 1 } else { 0 };
            graph_params.border_bottom = if graph_config.border.bottom { 1 } else { 0 };
            graph_params.border_right = if graph_config.border.right { 1 } else { 0 };
            graph_params.border_top = if graph_config.border.top { 1 } else { 0 };
            graph_params.curve_count = curve_configs.len().min(MAX_CURVES) as u32;
            // Write curve colors
            for (i, c) in curve_configs.iter().take(MAX_CURVES).enumerate() {
                let v = if let Some(metric_def) = metric_registry.get(&c.metric_id) {
                    metric_def.color.to_linear().to_vec4()
                } else {
                    Color::WHITE.to_linear().to_vec4()
                };
                graph_params.colors[i] = v;
            }
        }

        // Row container: left labels + right graph
        let label_width = graph_container.label_width.max(40.0);
        let graph_row = commands
            .spawn(Node {
                width: Val::Px(graph_container.size.x + label_width),
                height: Val::Px(graph_container.size.y),
                flex_direction: FlexDirection::Row,
                ..default()
            })
            .id();
        commands.entity(graph_row).insert(ChildOf(graph_parent));
        commands.entity(graph_row).insert(Visibility::Visible);

        // Label container (vertical to avoid overlap)
        let label_container = commands
            .spawn(Node {
                width: Val::Px(label_width),
                height: Val::Px(graph_container.size.y),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        commands.entity(label_container).insert(ChildOf(graph_row));

        // Create label rows matching configured curves
        let mut graph_labels: Vec<GraphLabelHandle> = Vec::new();
        for curve in curve_configs.iter().take(MAX_CURVES) {
            let eid = commands
                .spawn((
                    Text::new(""),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 11.0,
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

        // Graph node
        let graph_material = graph_mats.add(MultiLineGraphMaterial {
            params: graph_params,
        });
        let graph_entity = commands
            .spawn((
                MaterialNode(graph_material.clone()),
                Node {
                    width: Val::Px(graph_container.size.x),
                    height: Val::Px(graph_container.size.y),
                    ..default()
                },
            ))
            .id();
        commands.entity(graph_entity).insert(ChildOf(graph_row));

        // Update the GraphHandles component on the container entity
        commands.entity(container_entity).insert(GraphHandles {
            root: Some(graph_parent),
            graph_row: Some(graph_row),
            graph_entity: Some(graph_entity),
            graph_material: Some(graph_material),
            graph_labels,
            graph_label_width: label_width,
        });
    }
}
