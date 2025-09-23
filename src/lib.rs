use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
    text::TextColor,
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
            // 注册 UI 材质（图表与柱状条）
            .add_plugins(UiMaterialPlugin::<MultiLineGraphMaterial>::default())
            .add_plugins(UiMaterialPlugin::<BarMaterial>::default())
            .init_resource::<SampledValues>()
            .init_resource::<HistoryBuffers>()
            .add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    sample_diagnostics,
                    update_text_display,
                    update_graph_and_bars,
                )
                    .chain(),
            );
    }
}

/// Configuration for performance HUD
#[derive(Resource)]
pub struct PerfHudSettings {
    pub enabled: bool,
    pub origin: Vec2,
    pub graph_size: Vec2,
    pub graph_min_y: f32,
    pub graph_max_y: f32,
    pub graph_thickness: f32,
    pub curves: Vec<CurveConfig>,
    pub bars: Vec<BarConfig>,
}

/// Configuration for a performance curve
#[derive(Clone)]
pub struct CurveConfig {
    pub key: PerfKey,
    pub color: Color,
    pub autoscale: bool,
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
    pub text_entity: Entity,
    pub graph_entity: Option<Entity>,
    pub graph_material: Option<Handle<MultiLineGraphMaterial>>,
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

// 历史采样缓冲
#[derive(Resource)]
pub struct HistoryBuffers {
    pub values: [[f32; MAX_SAMPLES]; MAX_CURVES],
    pub length: u32, // 有效长度（<= MAX_SAMPLES）
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

// UI 材质：折线图
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
            colors: [Vec4::ZERO; MAX_CURVES],
            curve_count: 0,
        }
    }
}

// UI 材质：水平填充柱条
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

    // Spawn camera if one doesn't exist (needed for UI rendering)
    commands.spawn(Camera2d);

    // 根 UI 节点
    let root = commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(s.origin.y),
            left: Val::Px(s.origin.x),
            ..default()
        },))
        .id();

    // 文本显示
    let text_entity = commands
        .spawn((
            Text::new("FPS: 0\nFrame Time: 0.0ms"),
            TextColor(Color::WHITE),
            Node {
                width: Val::Px(s.graph_size.x),
                height: Val::Px(36.0),
                ..default()
            },
        ))
        .set_parent(root)
        .id();

    // 折线图材质与节点
    let mut graph_params = MultiLineGraphParams::default();
    graph_params.length = 0;
    graph_params.min_y = s.graph_min_y;
    graph_params.max_y = s.graph_max_y;
    graph_params.thickness = s.graph_thickness;
    graph_params.curve_count = s.curves.len().min(MAX_CURVES) as u32;
    // 颜色写入
    for (i, c) in s.curves.iter().take(MAX_CURVES).enumerate() {
        let v = c.color.to_linear().to_vec4();
        graph_params.colors[i] = v;
    }
    let graph_handle = graph_mats.add(MultiLineGraphMaterial {
        params: graph_params,
    });

    let graph_entity = commands
        .spawn((
            MaterialNode(graph_handle.clone()),
            Node {
                width: Val::Px(s.graph_size.x),
                height: Val::Px(s.graph_size.y),
                ..default()
            },
        ))
        .set_parent(root)
        .id();

    // 柱状条容器
    let bars_root = commands
        .spawn((Node {
            width: Val::Px(s.graph_size.x),
            height: Val::Px(60.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .set_parent(root)
        .id();

    let mut bar_entities = Vec::new();
    let mut bar_materials = Vec::new();
    for bar_cfg in &s.bars {
        // 单条柱状条容器（水平：标签+条）
        let row = commands
            .spawn((Node {
                width: Val::Px(s.graph_size.x),
                height: Val::Px(18.0),
                margin: UiRect {
                    top: Val::Px(4.0),
                    ..default()
                },
                ..default()
            },))
            .set_parent(bars_root)
            .id();

        // 左侧标签
        commands
            .spawn((
                Text::new(bar_cfg.label.clone()),
                TextColor(Color::WHITE),
                Node {
                    width: Val::Px(48.0),
                    height: Val::Px(18.0),
                    ..default()
                },
            ))
            .set_parent(row);

        // 右侧条形材质
        let mat = bar_mats.add(BarMaterial {
            params: BarParams {
                value: 0.0,
                r: bar_cfg.color.to_linear().to_vec4().x,
                g: bar_cfg.color.to_linear().to_vec4().y,
                b: bar_cfg.color.to_linear().to_vec4().z,
                a: bar_cfg.color.to_linear().to_vec4().w,
                bg_r: 0.12,
                bg_g: 0.12,
                bg_b: 0.12,
                bg_a: 1.0,
            },
        });
        let bar_entity = commands
            .spawn((
                MaterialNode(mat.clone()),
                Node {
                    width: Val::Px(s.graph_size.x - 56.0),
                    height: Val::Px(16.0),
                    ..default()
                },
            ))
            .set_parent(row)
            .id();

        bar_entities.push(bar_entity);
        bar_materials.push(mat);
    }

    // Store handles
    commands.insert_resource(HudHandles {
        text_entity,
        graph_entity: Some(graph_entity),
        graph_material: Some(graph_handle),
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
            samples.frame_time_ms = (frame_time * 1000.0).floor() as f32;
        }
    }

    // Placeholder values for other metrics
    samples.cpu_load = 0.5;
    samples.gpu_load = 0.3;
    samples.net_load = 0.1;
}

fn update_text_display(
    settings: Option<Res<PerfHudSettings>>,
    handles: Option<Res<HudHandles>>,
    samples: Res<SampledValues>,
    mut text_query: Query<&mut Text>,
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

    if let Ok(mut text) = text_query.get_mut(h.text_entity) {
        let new_text = format!(
            "FPS: {:.0}\nFrame Time: {:.1}ms",
            samples.fps, samples.frame_time_ms
        );

        // Only update if the text actually changed to avoid unnecessary updates
        if **text != new_text {
            **text = new_text;
        }
    }
}

fn update_graph_and_bars(
    settings: Option<Res<PerfHudSettings>>,
    handles: Option<Res<HudHandles>>,
    samples: Res<SampledValues>,
    mut history: ResMut<HistoryBuffers>,
    mut graph_mats: ResMut<Assets<MultiLineGraphMaterial>>,
    mut bar_mats: ResMut<Assets<BarMaterial>>,
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

    // 采样映射
    let mut curve_values = [0.0_f32; MAX_CURVES];
    for (i, cfg) in s.curves.iter().take(MAX_CURVES).enumerate() {
        curve_values[i] = match cfg.key {
            PerfKey::FrameTimeMs => samples.frame_time_ms,
            PerfKey::Fps => samples.fps,
            PerfKey::CpuLoad => samples.cpu_load,
            PerfKey::GpuLoad => samples.gpu_load,
            PerfKey::NetLoad => samples.net_load,
        };
    }

    // 推入历史（右移，追加在末尾）
    if (history.length as usize) < MAX_SAMPLES {
        // 直接填充到下一个索引
        let idx = history.length as usize;
        for i in 0..MAX_CURVES {
            history.values[i][idx] = curve_values[i];
        }
        history.length += 1;
    } else {
        // 滑动窗口：整体左移一格
        for i in 0..MAX_CURVES {
            history.values[i].copy_within(1..MAX_SAMPLES, 0);
            history.values[i][MAX_SAMPLES - 1] = curve_values[i];
        }
    }

    // 更新折线图材质
    if let Some(handle) = &h.graph_material {
        if let Some(mat) = graph_mats.get_mut(handle) {
            mat.params.length = history.length;
            // 计算 autoscale（若任一曲线启用 autoscale，则以该曲线统计）
            let mut min_y = s.graph_min_y;
            let mut max_y = s.graph_max_y;
            if s.curves.iter().any(|c| c.autoscale) && history.length > 0 {
                let len = history.length as usize;
                let mut mn = f32::INFINITY;
                let mut mx = f32::NEG_INFINITY;
                for (i, cfg) in s.curves.iter().take(MAX_CURVES).enumerate() {
                    if cfg.autoscale {
                        for k in 0..len {
                            mn = mn.min(history.values[i][k]);
                            mx = mx.max(history.values[i][k]);
                        }
                    }
                }
                if mn.is_finite() && mx.is_finite() && mx > mn {
                    min_y = mn;
                    max_y = mx;
                }
            }
            mat.params.min_y = min_y;
            mat.params.max_y = max_y;
            mat.params.thickness = s.graph_thickness;
            mat.params.curve_count = s.curves.len().min(MAX_CURVES) as u32;
            // 写入值（打包为 vec4）
            let len = MAX_SAMPLES.min(history.length as usize);
            let packed_len = (len + 3) / 4; // 向上取整
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
                // 可选：若想清理未使用段，可在 packed_len..SAMPLES_VEC4 置零
                for j in packed_len..SAMPLES_VEC4 {
                    mat.params.values[i][j] = Vec4::ZERO;
                }
            }
            // 颜色已在初始化时写入，若配置改变可在此更新
        }
    }

    // 更新柱状条
    for (i, cfg) in s.bars.iter().enumerate() {
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
        // 简单归一化：在 graph_min_y..graph_max_y 映射到 0..1
        let norm = if s.graph_max_y > s.graph_min_y {
            ((val - s.graph_min_y) / (s.graph_max_y - s.graph_min_y)).clamp(0.0, 1.0)
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
        }
    }
}

// Re-export helper API
pub use PerfHudSettings as Settings;
pub use PerfKey::*;
