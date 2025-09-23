use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
    text::{TextColor, TextFont},
    ui::{
        FlexDirection, MaterialNode, Node, PositionType, UiMaterial, UiMaterialPlugin, UiRect, Val,
    },
};

/// Plugin for displaying performance HUD
#[derive(Default)]
pub struct BevyPerfHudPlugin;

impl Plugin for BevyPerfHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            // Register UI materials (graph and bar)
            .add_plugins(UiMaterialPlugin::<MultiLineGraphMaterial>::default())
            .add_plugins(UiMaterialPlugin::<BarMaterial>::default())
            .init_resource::<SampledValues>()
            .init_resource::<HistoryBuffers>()
            .init_resource::<GraphScaleState>()
            .add_systems(Startup, setup_hud)
            .add_systems(Update, (sample_diagnostics, update_graph_and_bars).chain());
    }
}

/// Configuration for performance HUD
#[derive(Resource)]
pub struct PerfHudSettings {
    pub enabled: bool,
    pub origin: Vec2,
    pub graph: GraphSettings,
    pub bars: BarsSettings,
}

/// Graph (chart) settings
#[derive(Clone)]
pub struct GraphSettings {
    pub enabled: bool,
    pub size: Vec2,
    pub label_width: f32, // Left label width (pixels)
    pub min_y: f32,
    pub max_y: f32,
    pub thickness: f32,
    pub curves: Vec<CurveConfig>,
    pub bg_color: Color, // Graph background color (with alpha)
    // Border configuration
    pub border: GraphBorder,
    // Y-axis tick count (>=2). Unit/precision handled per curve
    pub y_ticks: u32,
    // Y-axis scale controls
    pub y_include_zero: bool,   // Force include 0
    pub y_min_span: f32,        // Minimum span to avoid tiny ranges
    pub y_margin_frac: f32,     // Vertical margin fraction (0..0.45)
    pub y_step_quantize: f32,   // Quantize min/max to step when > 0
    pub y_scale_smoothing: f32, // Scale smoothing factor (0..1)
}

#[derive(Clone)]
pub struct GraphBorder {
    pub color: Color,   // Color (with alpha)
    pub thickness: f32, // Thickness (pixels)
    pub left: bool,
    pub bottom: bool,
    pub right: bool,
    pub top: bool,
}

/// Bars settings
#[derive(Clone)]
pub struct BarsSettings {
    pub enabled: bool,
    pub bars: Vec<BarConfig>,
    pub bg_color: Color, // Bar background color (with alpha)
}

/// Configuration for a performance curve
#[derive(Clone)]
pub struct CurveConfig {
    pub key: PerfKey,
    pub color: Color,
    pub autoscale: bool,
    pub smoothing: f32,      // 0..1 exponential smoothing; 0=no filter, 1=follow new value
    pub quantize_step: f32,  // >0 rounds to nearest multiple of this step
    pub unit: String,        // Unit (e.g., "ms", "fps")
    pub unit_precision: u32, // Decimal precision
}

/// Configuration for a performance bar
#[derive(Clone)]
pub struct BarConfig {
    pub key: PerfKey,
    pub label: String,
    pub color: Color,
}

/// Performance metrics keys
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PerfKey {
    FrameTimeMs,
    Fps,
    CpuLoad,
    GpuLoad,
    NetLoad,
}

/// Handles to HUD entities
#[derive(Resource)]
pub struct HudHandles {
    pub graph_row: Option<Entity>,
    pub graph_entity: Option<Entity>,
    pub graph_material: Option<Handle<MultiLineGraphMaterial>>,
    pub graph_label_entities: Vec<Entity>,
    pub graph_label_width: f32,
    pub bars_root: Option<Entity>,
    pub bar_entities: Vec<Entity>,
    pub bar_materials: Vec<Handle<BarMaterial>>,
}

/// Sampled performance values
#[derive(Resource, Default)]
pub struct SampledValues {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub cpu_load: f32,
    pub gpu_load: f32,
    pub net_load: f32,
}

// History sample buffers
#[derive(Resource)]
pub struct HistoryBuffers {
    pub values: [[f32; MAX_SAMPLES]; MAX_CURVES],
    pub length: u32, // Valid length (<= MAX_SAMPLES)
}

impl Default for HistoryBuffers {
    fn default() -> Self {
        Self {
            values: [[0.0; MAX_SAMPLES]; MAX_CURVES],
            length: 0,
        }
    }
}

const MAX_SAMPLES: usize = 256;
const MAX_CURVES: usize = 6;
const SAMPLES_VEC4: usize = MAX_SAMPLES / 4;

// Graph scale state (smooth min/max to reduce autoscale jitter)
#[derive(Resource, Default, Clone, Copy)]
pub struct GraphScaleState {
    pub min_y: f32,
    pub max_y: f32,
}

// UI material: multi-line graph
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MultiLineGraphMaterial {
    #[uniform(0)]
    pub params: MultiLineGraphParams,
}

impl UiMaterial for MultiLineGraphMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/multiline_graph.wgsl".into())
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct MultiLineGraphParams {
    pub values: [[Vec4; SAMPLES_VEC4]; MAX_CURVES],
    pub length: u32,
    pub min_y: f32,
    pub max_y: f32,
    pub thickness: f32,
    pub bg_color: Vec4,
    pub border_color: Vec4,
    pub border_thickness: f32,
    pub border_thickness_uv_x: f32,
    pub border_thickness_uv_y: f32,
    pub border_left: u32,
    pub border_bottom: u32,
    pub border_right: u32,
    pub border_top: u32,
    pub colors: [Vec4; MAX_CURVES],
    pub curve_count: u32,
}

impl Default for MultiLineGraphParams {
    fn default() -> Self {
        Self {
            values: [[Vec4::ZERO; SAMPLES_VEC4]; MAX_CURVES],
            length: 0,
            min_y: 0.0,
            max_y: 1.0,
            thickness: 0.01,
            bg_color: Vec4::ZERO,
            border_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_thickness: 2.0,
            border_thickness_uv_x: 0.003,
            border_thickness_uv_y: 0.003,
            border_left: 1,
            border_bottom: 1,
            border_right: 0,
            border_top: 0,
            colors: [Vec4::ZERO; MAX_CURVES],
            curve_count: 0,
        }
    }
}

// UI material: horizontal fill bar
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct BarMaterial {
    #[uniform(0)]
    pub params: BarParams,
}

impl UiMaterial for BarMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/bar.wgsl".into())
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct BarParams {
    pub value: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub bg_r: f32,
    pub bg_g: f32,
    pub bg_b: f32,
    pub bg_a: f32,
}

fn setup_hud(
    mut commands: Commands,
    settings: Option<Res<PerfHudSettings>>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
) {
    let Some(s) = settings else {
        return;
    };
    if !s.enabled {
        return;
    }

    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera { order: 1, ..default() });

    // Root UI node
    let root = commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(s.origin.y),
            left: Val::Px(s.origin.x),
            ..default()
        },))
        .id();

    // Graph material and node (optional)
    let mut graph_row_opt: Option<Entity> = None;
    let mut graph_entity_opt: Option<Entity> = None;
    let mut graph_handle_opt: Option<Handle<MultiLineGraphMaterial>> = None;
    let mut graph_label_entities: Vec<Entity> = Vec::new();
    if s.graph.enabled {
        let mut graph_params = MultiLineGraphParams::default();
        graph_params.length = 0;
        graph_params.min_y = s.graph.min_y;
        graph_params.max_y = s.graph.max_y;
        graph_params.thickness = s.graph.thickness;
        graph_params.bg_color = s.graph.bg_color.to_linear().to_vec4();
        graph_params.border_color = s.graph.border.color.to_linear().to_vec4();
        graph_params.border_thickness = s.graph.border.thickness; // pixels
        graph_params.border_thickness_uv_x =
            (s.graph.border.thickness / s.graph.size.x).max(0.0001);
        graph_params.border_thickness_uv_y =
            (s.graph.border.thickness / s.graph.size.y).max(0.0001);
        graph_params.border_left = if s.graph.border.left { 1 } else { 0 };
        graph_params.border_bottom = if s.graph.border.bottom { 1 } else { 0 };
        graph_params.border_right = if s.graph.border.right { 1 } else { 0 };
        graph_params.border_top = if s.graph.border.top { 1 } else { 0 };
        graph_params.curve_count = s.graph.curves.len().min(MAX_CURVES) as u32;
        // Write curve colors
        for (i, c) in s.graph.curves.iter().take(MAX_CURVES).enumerate() {
            let v = c.color.to_linear().to_vec4();
            graph_params.colors[i] = v;
        }
        // Row container: left labels + right graph
        let label_width = s.graph.label_width.max(40.0);
        let graph_row = commands
            .spawn((Node {
                width: Val::Px(s.graph.size.x + label_width),
                height: Val::Px(s.graph.size.y),
                flex_direction: FlexDirection::Row,
                ..default()
            },))
            .id();
        commands.entity(graph_row).insert(ChildOf(root));
        graph_row_opt = Some(graph_row);

        // Label container (vertical to avoid overlap)
        let label_container = commands
            .spawn((Node {
                width: Val::Px(label_width),
                height: Val::Px(s.graph.size.y),
                flex_direction: FlexDirection::Column,
                ..default()
            },))
            .id();
        commands.entity(label_container).insert(ChildOf(graph_row));

        // Create two label rows
        for _ in 0..2usize {
            let eid = commands
                .spawn((
                    Text::new(""),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 12.0,
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
            graph_label_entities.push(eid);
        }

        // Graph node
        let gh = graph_mats.add(MultiLineGraphMaterial {
            params: graph_params,
        });
        let ge = commands
            .spawn((
                MaterialNode(gh.clone()),
                Node {
                    width: Val::Px(s.graph.size.x),
                    height: Val::Px(s.graph.size.y),
                    ..default()
                },
            ))
            .id();
        commands.entity(ge).insert(ChildOf(graph_row));
        graph_entity_opt = Some(ge);
        graph_handle_opt = Some(gh);
    }

    // Bars container
    let mut bars_root_opt: Option<Entity> = None;
    let mut bar_entities = Vec::new();
    let mut bar_materials = Vec::new();
    if s.bars.enabled && !s.bars.bars.is_empty() {
        let bars_root = commands
            .spawn((Node {
                width: Val::Px(s.graph.size.x),
                height: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },))
            .id();
        commands.entity(bars_root).insert(ChildOf(root));
        bars_root_opt = Some(bars_root);

        for bar_cfg in &s.bars.bars {
            // Single bar row (label + bar)
            let row = commands
                .spawn((Node {
                    width: Val::Px(s.graph.size.x),
                    height: Val::Px(18.0),
                    margin: UiRect {
                        top: Val::Px(4.0),
                        ..default()
                    },
                    ..default()
                },))
                .id();
            commands.entity(row).insert(ChildOf(bars_root));

            // Left label
            let label = commands
                .spawn((
                    Text::new(bar_cfg.label.clone()),
                    TextColor(Color::WHITE),
                    Node {
                        width: Val::Px(48.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                ))
                .id();
            commands.entity(label).insert(ChildOf(row));

            // Right bar material
            let mat = bar_mats.add(BarMaterial {
                params: BarParams {
                    value: 0.0,
                    r: bar_cfg.color.to_linear().to_vec4().x,
                    g: bar_cfg.color.to_linear().to_vec4().y,
                    b: bar_cfg.color.to_linear().to_vec4().z,
                    a: bar_cfg.color.to_linear().to_vec4().w,
                    bg_r: s.bars.bg_color.to_linear().to_vec4().x,
                    bg_g: s.bars.bg_color.to_linear().to_vec4().y,
                    bg_b: s.bars.bg_color.to_linear().to_vec4().z,
                    bg_a: s.bars.bg_color.to_linear().to_vec4().w,
                },
            });
            let bar_entity = commands
                .spawn((
                    MaterialNode(mat.clone()),
                    Node {
                        width: Val::Px(s.graph.size.x - 56.0),
                        height: Val::Px(16.0),
                        ..default()
                    },
                ))
                .id();
            commands.entity(bar_entity).insert(ChildOf(row));

            bar_entities.push(bar_entity);
            bar_materials.push(mat);
        }
    }

    // Store handles
    commands.insert_resource(HudHandles {
        graph_row: graph_row_opt,
        graph_entity: graph_entity_opt,
        graph_material: graph_handle_opt,
        graph_label_entities,
        graph_label_width: s.graph.label_width.max(40.0),
        bars_root: bars_root_opt,
        bar_entities,
        bar_materials,
    });
}

fn sample_diagnostics(
    diagnostics: Option<Res<DiagnosticsStore>>,
    settings: Option<Res<PerfHudSettings>>,
    mut samples: ResMut<SampledValues>,
) {
    let Some(diagnostics) = diagnostics else {
        return;
    };
    let Some(s) = settings else {
        return;
    };
    if !s.enabled {
        return;
    }

    // Sample FPS
    if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps) = fps_diagnostic.average() {
            samples.fps = fps.floor() as f32;
        }
    }

    // Sample frame time
    if let Some(frame_time_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        if let Some(frame_time) = frame_time_diagnostic.smoothed() {
            samples.frame_time_ms = frame_time.floor() as f32;
        }
    }

    // Placeholder values for other metrics
    samples.cpu_load = 0.5;
    samples.gpu_load = 0.3;
    samples.net_load = 0.1;
}

fn update_graph_and_bars(
    settings: Option<Res<PerfHudSettings>>,
    handles: Option<Res<HudHandles>>,
    samples: Res<SampledValues>,
    mut history: ResMut<HistoryBuffers>,
    mut scale_state: ResMut<GraphScaleState>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
    _label_node_q: Query<&mut Node>,
    mut label_text_q: Query<&mut Text>,
    mut label_color_q: Query<&mut TextColor>,
) {
    let Some(s) = settings else {
        return;
    };
    if !s.enabled {
        return;
    }
    let Some(h) = handles else {
        return;
    };

    // Sample mapping -> exponential smoothing -> quantization
    let mut filtered_values = [0.0_f32; MAX_CURVES];
    for (i, cfg) in s.graph.curves.iter().take(MAX_CURVES).enumerate() {
        let raw = match cfg.key {
            PerfKey::FrameTimeMs => samples.frame_time_ms,
            PerfKey::Fps => samples.fps,
            PerfKey::CpuLoad => samples.cpu_load,
            PerfKey::GpuLoad => samples.gpu_load,
            PerfKey::NetLoad => samples.net_load,
        };
        let alpha = cfg.smoothing.clamp(0.0, 1.0);
        // Use the last value as prev (read before shifting)
        let prev = if history.length == 0 {
            raw
        } else if (history.length as usize) < MAX_SAMPLES {
            history.values[i][history.length as usize - 1]
        } else {
            history.values[i][MAX_SAMPLES - 1]
        };
        let smoothed = prev + (raw - prev) * alpha;
        // Quantize: round to nearest multiple; disabled when step <= 0
        let step = cfg.quantize_step;
        filtered_values[i] = if step > 0.0 {
            (smoothed / step).round() * step
        } else {
            smoothed
        };
    }

    // Push into history (shift-right/append)
    if (history.length as usize) < MAX_SAMPLES {
        // Fill the next index directly
        let idx = history.length as usize;
        for i in 0..MAX_CURVES {
            history.values[i][idx] = filtered_values[i];
        }
        history.length += 1;
    } else {
        // Sliding window: shift left by one
        for i in 0..MAX_CURVES {
            history.values[i].copy_within(1..MAX_SAMPLES, 0);
            history.values[i][MAX_SAMPLES - 1] = filtered_values[i];
        }
    }

    // Compute target Y scale (autoscale or fixed), then smooth/quantize
    let mut target_min = s.graph.min_y;
    let mut target_max = s.graph.max_y;
    if s.graph.curves.iter().any(|c| c.autoscale) && history.length > 0 {
        let len = history.length as usize;
        let mut mn = f32::INFINITY;
        let mut mx = f32::NEG_INFINITY;
        for (i, cfg) in s.graph.curves.iter().take(MAX_CURVES).enumerate() {
            if cfg.autoscale {
                for k in 0..len {
                    mn = mn.min(history.values[i][k]);
                    mx = mx.max(history.values[i][k]);
                }
            }
        }
        if mn.is_finite() && mx.is_finite() {
            target_min = mn;
            target_max = mx;
        }
    }

    if s.graph.y_include_zero {
        target_min = target_min.min(0.0);
        target_max = target_max.max(0.0);
    }

    let mut span = (target_max - target_min)
        .abs()
        .max(s.graph.y_min_span.max(1e-3));
    if target_max - target_min < span {
        let mid = 0.5 * (target_max + target_min);
        target_min = mid - 0.5 * span;
        target_max = mid + 0.5 * span;
    }

    // Margins
    let margin_frac = s.graph.y_margin_frac.clamp(0.0, 0.45);
    let margin = span * margin_frac;
    target_min -= margin;
    target_max += margin;
    span = (target_max - target_min).max(1e-3);

    // Step quantization
    if s.graph.y_step_quantize > 0.0 {
        let step = s.graph.y_step_quantize;
        target_min = (target_min / step).floor() * step;
        target_max = (target_max / step).ceil() * step;
    }

    // Smoothing
    let a = s.graph.y_scale_smoothing.clamp(0.0, 1.0);
    if scale_state.max_y <= scale_state.min_y {
        scale_state.min_y = target_min;
        scale_state.max_y = target_max;
    } else {
        scale_state.min_y = scale_state.min_y + (target_min - scale_state.min_y) * a;
        scale_state.max_y = scale_state.max_y + (target_max - scale_state.max_y) * a;
    }

    let current_min = scale_state.min_y;
    let current_max = (scale_state.max_y).max(current_min + 1e-3);

    // Update two left labels: FPS and FrameTimeMs (value + unit)
    if s.graph.enabled && h.graph_label_entities.len() >= 2 {
        // Row 1: FPS (integer) value + unit
        let fps_text = format!("{:.0} fps", samples.fps);
        if let Ok(mut tx) = label_text_q.get_mut(h.graph_label_entities[0]) {
            if **tx != fps_text {
                **tx = fps_text;
            }
        }
        if let Some(cur) = s
            .graph
            .curves
            .iter()
            .find(|c| matches!(c.key, PerfKey::Fps))
        {
            if let Ok(mut col) = label_color_q.get_mut(h.graph_label_entities[0]) {
                *col = TextColor(cur.color);
            }
        }
        // Use column layout to avoid label overlap

        // Row 2: Frame time (1 decimal) value + unit
        let ft_text = format!("{:.1} ms", samples.frame_time_ms);
        if let Ok(mut tx) = label_text_q.get_mut(h.graph_label_entities[1]) {
            if **tx != ft_text {
                **tx = ft_text;
            }
        }
        if let Some(cur) = s
            .graph
            .curves
            .iter()
            .find(|c| matches!(c.key, PerfKey::FrameTimeMs))
        {
            if let Ok(mut col) = label_color_q.get_mut(h.graph_label_entities[1]) {
                *col = TextColor(cur.color);
            }
        }
        // Use column layout to avoid label overlap
    }

    // Update graph material (when enabled)
    if s.graph.enabled {
        if let Some(handle) = &h.graph_material {
            if let Some(mat) = graph_mats.get_mut(handle) {
                mat.params.length = history.length;
                mat.params.min_y = current_min;
                mat.params.max_y = current_max;
                mat.params.thickness = s.graph.thickness;
                mat.params.bg_color = s.graph.bg_color.to_linear().to_vec4();
                mat.params.border_color = s.graph.border.color.to_linear().to_vec4();
                mat.params.border_thickness = s.graph.border.thickness; // pixels
                mat.params.border_thickness_uv_x =
                    (s.graph.border.thickness / s.graph.size.x).max(0.0001);
                mat.params.border_thickness_uv_y =
                    (s.graph.border.thickness / s.graph.size.y).max(0.0001);
                mat.params.border_left = if s.graph.border.left { 1 } else { 0 };
                mat.params.border_bottom = if s.graph.border.bottom { 1 } else { 0 };
                mat.params.border_right = if s.graph.border.right { 1 } else { 0 };
                mat.params.border_top = if s.graph.border.top { 1 } else { 0 };
                mat.params.curve_count = s.graph.curves.len().min(MAX_CURVES) as u32;
                // Sync curve colors every frame to allow hot updates
                for (i, c) in s.graph.curves.iter().take(MAX_CURVES).enumerate() {
                    mat.params.colors[i] = c.color.to_linear().to_vec4();
                }
                // Write values (pack into vec4)
                let len = MAX_SAMPLES.min(history.length as usize);
                let packed_len = (len + 3) / 4; // round up
                for i in 0..MAX_CURVES {
                    for j in 0..SAMPLES_VEC4 {
                        let base = j * 4;
                        let x0 = if base + 0 < len {
                            history.values[i][base + 0]
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
    if s.bars.enabled {
        for (i, cfg) in s.bars.bars.iter().enumerate() {
            if i >= h.bar_materials.len() {
                break;
            }
            let val = match cfg.key {
                PerfKey::FrameTimeMs => samples.frame_time_ms,
                PerfKey::Fps => samples.fps,
                PerfKey::CpuLoad => samples.cpu_load,
                PerfKey::GpuLoad => samples.gpu_load,
                PerfKey::NetLoad => samples.net_load,
            };
            // Normalize: map current_min..current_max to 0..1
            let norm = if current_max > current_min {
                ((val - current_min) / (current_max - current_min)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            if let Some(mat) = bar_mats.get_mut(&h.bar_materials[i]) {
                mat.params.value = norm;
                let v = cfg.color.to_linear().to_vec4();
                mat.params.r = v.x;
                mat.params.g = v.y;
                mat.params.b = v.z;
                mat.params.a = v.w;
                let bg = s.bars.bg_color.to_linear().to_vec4();
                mat.params.bg_r = bg.x;
                mat.params.bg_g = bg.y;
                mat.params.bg_b = bg.z;
                mat.params.bg_a = bg.w;
            }
        }
    }
}

// Re-export helper API
pub use PerfHudSettings as Settings;
pub use PerfKey::*;
