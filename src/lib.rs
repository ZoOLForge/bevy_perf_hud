use bevy::{
    diagnostic::{
        DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
    text::{TextColor, TextFont},
    ui::{
        FlexDirection, MaterialNode, Node, PositionType, UiMaterial, UiMaterialPlugin, UiRect, Val,
    },
};
use std::collections::HashMap;

/// Plugin for displaying performance HUD
#[derive(Default)]
pub struct BevyPerfHudPlugin;

impl Plugin for BevyPerfHudPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        };

        if !app.is_plugin_added::<EntityCountDiagnosticsPlugin>() {
            app.add_plugins(EntityCountDiagnosticsPlugin::default());
        };

        if !app.is_plugin_added::<SystemInformationDiagnosticsPlugin>() {
            app.add_plugins(SystemInformationDiagnosticsPlugin::default());
        };

        // Register UI materials (graph and bar)
        app.add_plugins(UiMaterialPlugin::<MultiLineGraphMaterial>::default())
            .add_plugins(UiMaterialPlugin::<BarMaterial>::default())
            .init_resource::<SampledValues>()
            .init_resource::<MetricProviders>()
            .init_resource::<HistoryBuffers>()
            .init_resource::<GraphScaleState>()
            .add_systems(Startup, setup_hud)
            .add_systems(Update, (sample_diagnostics, update_graph_and_bars).chain());

        app.world_mut()
            .resource_mut::<MetricProviders>()
            .ensure_default_entries();
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

impl Default for PerfHudSettings {
    fn default() -> Self {
        let frame_metric = MetricDefinition {
            id: "frame_time_ms".into(),
            label: Some("FT:".into()),
            unit: Some("ms".into()),
            precision: 1,
            color: Color::srgb(0.0, 1.0, 0.0),
        };
        let fps_metric = MetricDefinition {
            id: "fps".into(),
            label: Some("FPS:".into()),
            unit: Some("fps".into()),
            precision: 0,
            color: Color::srgb(0.9, 0.0, 0.0),
        };
        let entity_metric = MetricDefinition {
            id: "entity_count".into(),
            label: Some("Ent:".into()),
            unit: None,
            precision: 0,
            color: Color::srgb(0.1, 0.8, 0.4),
        };
        let sys_cpu_metric = MetricDefinition {
            id: SYSTEM_CPU_USAGE_ID.to_owned(),
            label: Some("SysCPU".into()),
            unit: Some("%".into()),
            precision: 1,
            color: Color::srgb(0.96, 0.76, 0.18),
        };
        let sys_mem_metric = MetricDefinition {
            id: SYSTEM_MEM_USAGE_ID.to_owned(),
            label: Some("SysMem".into()),
            unit: Some("%".into()),
            precision: 1,
            color: Color::srgb(0.28, 0.56, 0.89),
        };

        Self {
            enabled: true,
            origin: Vec2::new(960.0, 16.0),
            graph: GraphSettings {
                enabled: true,
                size: Vec2::new(200.0, 80.0),
                label_width: 60.0,
                min_y: 0.0,
                max_y: 30.0,
                thickness: 0.012,
                curves: vec![
                    CurveConfig {
                        metric: frame_metric.clone(),
                        autoscale: None,
                        smoothing: Some(0.25),
                        quantize_step: Some(0.1),
                    },
                    CurveConfig {
                        metric: fps_metric.clone(),
                        autoscale: None,
                        smoothing: None,
                        quantize_step: None,
                    },
                ],
                curve_defaults: CurveDefaults {
                    autoscale: true,
                    smoothing: 0.2,
                    quantize_step: 1.0,
                },
                bg_color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                border: GraphBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 1.0),
                    thickness: 2.0,
                    left: true,
                    bottom: true,
                    right: false,
                    top: false,
                },
                y_ticks: 2,
                y_include_zero: true,
                y_min_span: 5.0,
                y_margin_frac: 0.10,
                y_step_quantize: 5.0,
                y_scale_smoothing: 0.3,
            },
            bars: BarsSettings {
                enabled: true,
                bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
                show_value_default: true,
                bars: vec![
                    BarConfig {
                        metric: sys_cpu_metric,
                        show_value: Some(false),
                    },
                    BarConfig {
                        metric: sys_mem_metric,
                        show_value: Some(false),
                    },
                    BarConfig {
                        metric: entity_metric,
                        show_value: None,
                    },
                ],
            },
        }
    }
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
    pub curve_defaults: CurveDefaults,
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
    pub bg_color: Color,          // Bar background color (with alpha)
    pub show_value_default: bool, // Default setting for showing value and unit in bars
}

/// Configuration for a performance curve
#[derive(Clone)]
pub struct CurveConfig {
    pub metric: MetricDefinition,
    pub autoscale: Option<bool>,
    pub smoothing: Option<f32>, // 0..1 exponential smoothing; 0=no filter, 1=follow new value
    pub quantize_step: Option<f32>, // >0 rounds to nearest multiple of this step
}

#[derive(Clone)]
pub struct CurveDefaults {
    pub autoscale: bool,
    pub smoothing: f32,
    pub quantize_step: f32,
}

/// Configuration for a performance bar
#[derive(Clone)]
pub struct BarConfig {
    pub metric: MetricDefinition,
    pub show_value: Option<bool>, // Whether to show value and unit, defaults to show_value_default
}

#[derive(Clone)]
pub struct MetricDefinition {
    pub id: String,
    pub label: Option<String>,
    pub unit: Option<String>,
    pub precision: u32,
    pub color: Color,
}

#[derive(Clone)]
pub struct GraphLabelHandle {
    pub metric_id: String,
    pub entity: Entity,
}

/// Handles to HUD entities
#[derive(Resource)]
pub struct HudHandles {
    pub graph_row: Option<Entity>,
    pub graph_entity: Option<Entity>,
    pub graph_material: Option<Handle<MultiLineGraphMaterial>>,
    pub graph_labels: Vec<GraphLabelHandle>,
    pub graph_label_width: f32,
    pub bars_root: Option<Entity>,
    pub bar_entities: Vec<Entity>,
    pub bar_materials: Vec<Handle<BarMaterial>>,
    pub bar_labels: Vec<Entity>,
}

/// Sampled performance values
#[derive(Resource, Default)]
pub struct SampledValues {
    values: HashMap<String, f32>,
}

impl SampledValues {
    /// 设置性能指标值 / Set performance metric value
    pub fn set(&mut self, id: &str, value: f32) {
        if let Some(existing) = self.values.get_mut(id) {
            *existing = value;
        } else {
            self.values.insert(id.to_owned(), value);
        }
    }

    /// 获取指标的当前值 / Fetch current value of a metric
    pub fn get(&self, id: &str) -> Option<f32> {
        self.values.get(id).copied()
    }
}

/// 性能度量采样上下文 / Metric sampling context
#[derive(Clone, Copy)]
pub struct MetricSampleContext<'a> {
    pub diagnostics: Option<&'a DiagnosticsStore>,
}

/// 性能度量提供者约定 / Trait for performance metric providers
pub trait PerfMetricProvider: Send + Sync + 'static {
    fn metric_id(&self) -> &str;
    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32>;
}

/// 性能度量提供者集合 / Registry for metric providers
#[derive(Resource, Default)]
pub struct MetricProviders {
    providers: Vec<Box<dyn PerfMetricProvider>>,
}

impl MetricProviders {
    /// 添加一个新的度量提供者 / Register a new provider
    pub fn add_provider<P: PerfMetricProvider>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }

    /// 检查是否已存在指定键值 / Detect if a provider for the key exists
    pub fn contains(&self, id: &str) -> bool {
        self.providers.iter().any(|p| p.metric_id() == id)
    }

    /// 确保内置度量可用 / Ensure built-in metrics are registered
    pub fn ensure_default_entries(&mut self) {
        self.ensure_provider(FpsMetricProvider::default());
        self.ensure_provider(FrameTimeMetricProvider::default());
        self.ensure_provider(EntityCountMetricProvider::default());
        self.ensure_provider(SystemCpuUsageMetricProvider::default());
        self.ensure_provider(SystemMemUsageMetricProvider::default());
        self.ensure_provider(ProcessCpuUsageMetricProvider::default());
        self.ensure_provider(ProcessMemUsageMetricProvider::default());
    }

    /// 迭代所有提供者 / Iterate through all providers
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn PerfMetricProvider> {
        self.providers.iter_mut().map(|p| p.as_mut())
    }

    fn ensure_provider<P: PerfMetricProvider>(&mut self, provider: P) {
        let id = provider.metric_id().to_owned();
        if !self.contains(&id) {
            self.providers.push(Box::new(provider));
        }
    }
}

/// App 扩展方法 / App extension methods
pub trait PerfHudAppExt {
    fn add_perf_metric_provider<P: PerfMetricProvider>(&mut self, provider: P) -> &mut Self;
}

impl PerfHudAppExt for App {
    fn add_perf_metric_provider<P: PerfMetricProvider>(&mut self, provider: P) -> &mut Self {
        self.init_resource::<MetricProviders>();
        self.world_mut()
            .resource_mut::<MetricProviders>()
            .add_provider(provider);
        self
    }
}

/// FPS 提供者 / FPS metric provider
#[derive(Default)]
pub struct FpsMetricProvider;

impl PerfMetricProvider for FpsMetricProvider {
    fn metric_id(&self) -> &str {
        "fps"
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)?
            .average()?;
        Some(fps.floor() as f32)
    }
}

/// 帧时间提供者 / Frame time metric provider
#[derive(Default)]
pub struct FrameTimeMetricProvider;

impl PerfMetricProvider for FrameTimeMetricProvider {
    fn metric_id(&self) -> &str {
        "frame_time_ms"
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let frame_time = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)?
            .smoothed()?;
        Some(frame_time.floor() as f32)
    }
}

/// 实体数量提供者 / Entity count metric provider
#[derive(Default)]
pub struct EntityCountMetricProvider;

impl PerfMetricProvider for EntityCountMetricProvider {
    fn metric_id(&self) -> &str {
        "entity_count"
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let entities = diagnostics
            .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)?
            .value()?;
        Some(entities as f32)
    }
}

const SYSTEM_CPU_USAGE_ID: &str = "system/cpu_usage";
const SYSTEM_MEM_USAGE_ID: &str = "system/mem_usage";
const PROCESS_CPU_USAGE_ID: &str = "process/cpu_usage";
const PROCESS_MEM_USAGE_ID: &str = "process/mem_usage";

/// 系统 CPU 使用率提供者 / System CPU usage metric provider
#[derive(Default)]
pub struct SystemCpuUsageMetricProvider;

impl PerfMetricProvider for SystemCpuUsageMetricProvider {
    fn metric_id(&self) -> &str {
        SYSTEM_CPU_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}

/// 系统内存使用率提供者 / System memory usage metric provider
#[derive(Default)]
pub struct SystemMemUsageMetricProvider;

impl PerfMetricProvider for SystemMemUsageMetricProvider {
    fn metric_id(&self) -> &str {
        SYSTEM_MEM_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}

/// 进程 CPU 使用率提供者 / Process CPU usage metric provider
#[derive(Default)]
pub struct ProcessCpuUsageMetricProvider;

impl PerfMetricProvider for ProcessCpuUsageMetricProvider {
    fn metric_id(&self) -> &str {
        PROCESS_CPU_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}

/// 进程内存使用量提供者 / Process memory usage metric provider
#[derive(Default)]
pub struct ProcessMemUsageMetricProvider;

impl PerfMetricProvider for ProcessMemUsageMetricProvider {
    fn metric_id(&self) -> &str {
        PROCESS_MEM_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE)?
            .value()?;
        Some(usage as f32)
    }
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

mod shader_params {
    #![allow(dead_code)]

    use super::*;

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
        pub values: [[Vec4; super::SAMPLES_VEC4]; super::MAX_CURVES],
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
        pub colors: [Vec4; super::MAX_CURVES],
        pub curve_count: u32,
    }

    impl Default for MultiLineGraphParams {
        fn default() -> Self {
            Self {
                values: [[Vec4::ZERO; super::SAMPLES_VEC4]; super::MAX_CURVES],
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
                colors: [Vec4::ZERO; super::MAX_CURVES],
                curve_count: 0,
            }
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
}

use shader_params::{BarMaterial, BarParams, MultiLineGraphMaterial, MultiLineGraphParams};

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
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Root UI node
    let root = commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(s.origin.y),
            left: Val::Px(s.origin.x),
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .id();

    // Graph material and node (optional)
    let mut graph_row_opt: Option<Entity> = None;
    let mut graph_entity_opt: Option<Entity> = None;
    let mut graph_handle_opt: Option<Handle<MultiLineGraphMaterial>> = None;
    let mut graph_labels: Vec<GraphLabelHandle> = Vec::new();
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
            let v = c.metric.color.to_linear().to_vec4();
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

        // Create label rows matching configured curves
        for curve in s.graph.curves.iter().take(MAX_CURVES) {
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

    // Bars container placed below the graph / 将柱状图容器放在图表下方
    let mut bars_root_opt: Option<Entity> = None;
    let mut bar_entities = Vec::new();
    let mut bar_materials = Vec::new();
    let mut bar_labels = Vec::new();
    if s.bars.enabled && !s.bars.bars.is_empty() {
        let column_count = 2;
        let column_width = (s.graph.size.x - 12.0) / column_count as f32;

        let bars_root = commands
            .spawn((Node {
                width: Val::Px(s.graph.size.x),
                height: Val::Px((s.bars.bars.len() as f32 / column_count as f32).ceil() * 25.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect {
                    left: Val::Px(s.graph.label_width.max(40.0)),
                    top: Val::Px(4.0),
                    ..default()
                },
                ..default()
            },))
            .id();
        commands.entity(bars_root).insert(ChildOf(root));
        bars_root_opt = Some(bars_root);

        for chunk in s.bars.bars.chunks(column_count) {
            let row = commands
                .spawn((Node {
                    width: Val::Px(s.graph.size.x),
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

    // Store handles
    commands.insert_resource(HudHandles {
        graph_row: graph_row_opt,
        graph_entity: graph_entity_opt,
        graph_material: graph_handle_opt,
        graph_labels,
        graph_label_width: s.graph.label_width.max(40.0),
        bars_root: bars_root_opt,
        bar_entities,
        bar_materials,
        bar_labels,
    });
}

fn sample_diagnostics(
    diagnostics: Option<Res<DiagnosticsStore>>,
    settings: Option<Res<PerfHudSettings>>,
    mut samples: ResMut<SampledValues>,
    mut providers: ResMut<MetricProviders>,
) {
    let Some(s) = settings else {
        return;
    };
    if !s.enabled {
        return;
    }

    let ctx = MetricSampleContext {
        diagnostics: diagnostics.as_deref(),
    };

    for provider in providers.iter_mut() {
        if let Some(value) = provider.sample(ctx) {
            samples.set(provider.metric_id(), value);
        }
    }
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

    let curve_count = s.graph.curves.len().min(MAX_CURVES);

    // Sample mapping -> exponential smoothing -> quantization
    let mut filtered_values = [0.0_f32; MAX_CURVES];
    for (i, cfg) in s.graph.curves.iter().take(curve_count).enumerate() {
        let raw = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);
        let smoothing = cfg
            .smoothing
            .unwrap_or(s.graph.curve_defaults.smoothing)
            .clamp(0.0, 1.0);
        // Use the last value as prev (read before shifting)
        let prev = if history.length == 0 {
            raw
        } else if (history.length as usize) < MAX_SAMPLES {
            history.values[i][history.length as usize - 1]
        } else {
            history.values[i][MAX_SAMPLES - 1]
        };
        let smoothed = prev + (raw - prev) * smoothing;
        // Quantize: round to nearest multiple; disabled when step <= 0
        let step = cfg
            .quantize_step
            .unwrap_or(s.graph.curve_defaults.quantize_step);
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
            let value = if i < curve_count {
                filtered_values[i]
            } else {
                0.0
            };
            history.values[i][idx] = value;
        }
        history.length += 1;
    } else {
        // Sliding window: shift left by one
        for i in 0..MAX_CURVES {
            history.values[i].copy_within(1..MAX_SAMPLES, 0);
            let value = if i < curve_count {
                filtered_values[i]
            } else {
                0.0
            };
            history.values[i][MAX_SAMPLES - 1] = value;
        }
    }

    // Compute target Y scale (autoscale or fixed), then smooth/quantize
    let mut target_min = s.graph.min_y;
    let mut target_max = s.graph.max_y;
    if s.graph
        .curves
        .iter()
        .any(|c| c.autoscale.unwrap_or(s.graph.curve_defaults.autoscale))
        && history.length > 0
    {
        let len = history.length as usize;
        let mut mn = f32::INFINITY;
        let mut mx = f32::NEG_INFINITY;
        for (i, cfg) in s.graph.curves.iter().take(curve_count).enumerate() {
            if cfg.autoscale.unwrap_or(s.graph.curve_defaults.autoscale) {
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

    let span = (target_max - target_min)
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

    // Update graph labels dynamically based on configured curves
    if s.graph.enabled && !h.graph_labels.is_empty() {
        for label_handle in &h.graph_labels {
            let Some(curve) = s
                .graph
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
                mat.params.curve_count = curve_count as u32;
                // Sync curve colors every frame to allow hot updates
                for (i, c) in s.graph.curves.iter().take(curve_count).enumerate() {
                    mat.params.colors[i] = c.metric.color.to_linear().to_vec4();
                }
                for i in curve_count..MAX_CURVES {
                    mat.params.colors[i] = Vec4::ZERO;
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
            let val = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);
            // Normalize: map current_min..current_max to 0..1
            let norm = if current_max > current_min {
                ((val - current_min) / (current_max - current_min)).clamp(0.0, 1.0)
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
                let bg = s.bars.bg_color.to_linear().to_vec4();
                mat.params.bg_r = bg.x;
                mat.params.bg_g = bg.y;
                mat.params.bg_b = bg.z;
                mat.params.bg_a = bg.w;
            }

            // 在柱条内部刷新文字与颜色 / Update bar label text inside the bar
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
                let show_value = cfg.show_value.unwrap_or(s.bars.show_value_default);
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
