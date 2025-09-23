use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use bevy_perf_hud::{BarConfig, BevyPerfHudPlugin, CurveConfig, HudHandles, PerfKey, Settings};

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum HudMode {
    #[default]
    Full,
    GraphOnly,
    Hidden,
}

#[derive(Resource, Default)]
struct CubeState {
    count: u32, // 已生成立方体总数
}

#[derive(Component)]
struct DemoCube;

#[derive(Component)]
struct MainCamera;

// 取消 Orbit，相机改为自由飞行控制

#[derive(Resource)]
struct SpawnParams {
    batch: u32,       // 每次生成个数
    spacing: f32,     // 网格间距
    jitter_frac: f32, // 相对网格尺寸的抖动比例（0..1）
    min_dist: f32,    // 生成中心的最小距离（沿前向）
    max_dist: f32,    // 生成中心的最大距离（沿前向）
}

impl Default for SpawnParams {
    fn default() -> Self {
        Self {
            batch: 50,
            spacing: 1.4,
            jitter_frac: 0.35,
            min_dist: 6.0,
            max_dist: 40.0,
        }
    }
}

// 简单 LCG 伪随机数生成器（避免额外依赖）
#[derive(Resource)]
struct RngState {
    state: u64,
}

impl Default for RngState {
    fn default() -> Self {
        // 固定种子；如需不同运行产生不同序列，可替换为时间或外部种子
        Self {
            state: 0x9E3779B97F4A7C15,
        }
    }
}

impl RngState {
    fn next_u64(&mut self) -> u64 {
        // 64-bit LCG 常用参数
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }
    fn next_f32(&mut self) -> f32 {
        let v = self.next_u64() >> 40; // 取高位，24-bit 精度
        (v as f32) / ((1u32 << 24) as f32)
    }
    fn range_f32(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f32()
    }
}

fn setup_3d(mut commands: Commands) {
    let tf = Transform::from_xyz(-8.0, 8.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y);
    // 3D 相机
    commands.spawn((Camera3d::default(), tf, MainCamera));
    // 简单方向光
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.6, 0.0)),
    ));
}

fn spawn_cube_on_space(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CubeState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    spawn: Res<SpawnParams>,
    q_cam: Query<&Transform, With<MainCamera>>,
    mut rng: ResMut<RngState>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    // 基于“镜头位置”生成：以相机位置 + 前向 * 距离 为中心，
    // 在与视线垂直的平面（right/up）上生成一个网格，保证尽量在视野范围内。
    let Some(t) = q_cam.single().ok() else {
        return;
    };
    let fwd = t.forward();
    let right = t.right();
    let up = t.up();
    // 前向随机距离（由参数控制），避免过近/过远且每批次不同
    let mut min_d = spawn.min_dist;
    let mut max_d = spawn.max_dist;
    if max_d < min_d {
        core::mem::swap(&mut min_d, &mut max_d);
    }
    min_d = min_d.clamp(0.5, 500.0);
    max_d = max_d.clamp(min_d + 0.5, 1000.0);
    let dist = rng.range_f32(min_d, max_d);
    let center = t.translation + fwd * dist;

    // 网格参数
    let n = spawn.batch.max(1) as usize;
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = (n + cols - 1) / cols;
    let sx = spawn.spacing;

    // 平面抖动：以当前批网格的宽高为尺度，随机平移中心，避免多批完全重叠
    let grid_w = (cols.max(1) as f32 - 1.0) * sx;
    let grid_h = (rows.max(1) as f32 - 1.0) * sx;
    let j = spawn.jitter_frac.clamp(0.0, 1.0);
    let jitter_r = if grid_w > 0.0 {
        rng.range_f32(-grid_w * j, grid_w * j)
    } else {
        0.0
    };
    let jitter_u = if grid_h > 0.0 {
        rng.range_f32(-grid_h * j, grid_h * j)
    } else {
        0.0
    };
    let center = center + right * jitter_r + up * jitter_u;

    let mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    for k in 0..n {
        let c = (k % cols) as f32;
        let r = (k / cols) as f32;
        let off_r = (c - (cols as f32 - 1.0) * 0.5) * sx;
        let off_u = (r - (rows as f32 - 1.0) * 0.5) * sx;
        let pos = center + right * off_r + up * off_u;

        let hue = ((state.count as usize + k) % 360) as f32;
        let material = materials.add(StandardMaterial {
            base_color: Color::hsl(hue, 0.65, 0.55),
            perceptual_roughness: 0.6,
            metallic: 0.0,
            ..default()
        });

        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(pos),
            DemoCube,
        ));
    }

    state.count += n as u32;
}

fn clear_cubes_on_r(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CubeState>,
    mut commands: Commands,
    q: Query<Entity, With<DemoCube>>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    for e in q.iter() {
        commands.entity(e).despawn();
    }
    state.count = 0;
}

fn adjust_spawn_and_camera_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut spawn: ResMut<SpawnParams>,
    mut q_cam: Query<&mut Transform, With<MainCamera>>,
) {
    // 调整批量：[ 减少, ] 增加；按住 Shift 为大步长
    let big = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let step = if big { 50 } else { 10 };
    if keys.just_pressed(KeyCode::BracketLeft) {
        spawn.batch = spawn.batch.saturating_sub(step).max(1);
        println!("batch -> {}", spawn.batch);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        spawn.batch = (spawn.batch + step).min(20_000);
        println!("batch -> {}", spawn.batch);
    }

    // 自由飞行相机控制
    let mut tf = match q_cam.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };
    let boost = if big { 3.0 } else { 1.0 };
    let move_step = 0.4 * boost; // 单帧位移
    let yaw_step = 0.06 * boost;
    let pitch_step = 0.04 * boost;

    // 先取方向向量，避免在可变借用期间再对 tf 不可变借用（借用冲突 E0502）
    let fwd = tf.forward();
    let right = tf.right();
    let up = Vec3::Y;

    // 平移：W/S 前后，A/D 左右，Q/E 下/上
    if keys.pressed(KeyCode::KeyW) {
        tf.translation += fwd * move_step;
    }
    if keys.pressed(KeyCode::KeyS) {
        tf.translation -= fwd * move_step;
    }
    if keys.pressed(KeyCode::KeyA) {
        tf.translation -= right * move_step;
    }
    if keys.pressed(KeyCode::KeyD) {
        tf.translation += right * move_step;
    }
    if keys.pressed(KeyCode::KeyQ) {
        tf.translation -= up * move_step;
    }
    if keys.pressed(KeyCode::KeyE) {
        tf.translation += up * move_step;
    }

    // 旋转：方向键 左右=偏航，上下=俯仰
    if keys.pressed(KeyCode::ArrowLeft) {
        tf.rotate_y(yaw_step);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        tf.rotate_y(-yaw_step);
    }
    if keys.pressed(KeyCode::ArrowUp) {
        tf.rotate_local_x(pitch_step);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        tf.rotate_local_x(-pitch_step);
    }
}

fn toggle_hud_mode_on_f1(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<HudMode>,
    handles: Option<Res<HudHandles>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }
    let Some(h) = handles else {
        return;
    };

    match *mode {
        HudMode::Full => {
            // 下一态：只显示 Graph
            if let Some(e) = h.graph_row {
                commands.entity(e).insert(Visibility::Visible);
            }
            if let Some(e) = h.bars_root {
                commands.entity(e).insert(Visibility::Hidden);
            }
            *mode = HudMode::GraphOnly;
        }
        HudMode::GraphOnly => {
            // 下一态：全隐藏
            if let Some(e) = h.graph_row {
                commands.entity(e).insert(Visibility::Hidden);
            }
            if let Some(e) = h.bars_root {
                commands.entity(e).insert(Visibility::Hidden);
            }
            *mode = HudMode::Hidden;
        }
        HudMode::Hidden => {
            // 下一态：全显示
            if let Some(e) = h.graph_row {
                commands.entity(e).insert(Visibility::Visible);
            }
            if let Some(e) = h.bars_root {
                commands.entity(e).insert(Visibility::Visible);
            }
            *mode = HudMode::Full;
        }
    }
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<SpawnParams>()
        .init_resource::<RngState>()
        .init_resource::<CubeState>()
        .init_resource::<HudMode>()
        .insert_resource(Settings {
            enabled: true,
            origin: Vec2::new(16.0, 16.0),
            graph: bevy_perf_hud::GraphSettings {
                enabled: true,
                size: Vec2::new(400.0, 120.0),
                label_width: 60.0,
                min_y: 0.0,
                max_y: 30.0,
                thickness: 0.012,
                bg_color: Color::srgba(0.0, 0.0, 0.0, 0.25),
                y_ticks: 2,
                border: bevy_perf_hud::GraphBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 1.0),
                    thickness: 2.0,
                    left: true,
                    bottom: true,
                    right: false,
                    top: false,
                },
                // Y 轴比例控制：包含 0、最小跨度与边距、步进量化与平滑
                y_include_zero: true,
                y_min_span: 5.0,
                y_margin_frac: 0.10,
                y_step_quantize: 5.0,
                y_scale_smoothing: 0.3,
                curves: vec![
                    CurveConfig {
                        key: PerfKey::FrameTimeMs,
                        color: Color::srgb(0.0, 1.0, 0.0),
                        autoscale: true,
                        smoothing: 0.25,
                        quantize_step: 0.1,
                        unit: "MS".into(),
                        unit_precision: 1,
                    },
                    CurveConfig {
                        key: PerfKey::Fps,
                        color: Color::srgb(0.9, 0.0, 0.0),
                        autoscale: true,
                        smoothing: 0.2,
                        quantize_step: 1.0,
                        unit: "FPS".into(),
                        unit_precision: 0,
                    },
                ],
            },
            bars: bevy_perf_hud::BarsSettings {
                enabled: true,
                bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
                bars: vec![
                    BarConfig {
                        key: PerfKey::CpuLoad,
                        label: "CPU".into(),
                        color: Color::srgb(1.0, 0.3, 0.0),
                    },
                    BarConfig {
                        key: PerfKey::GpuLoad,
                        label: "GPU".into(),
                        color: Color::srgb(0.0, 0.0, 1.0),
                    },
                    BarConfig {
                        key: PerfKey::NetLoad,
                        label: "NET".into(),
                        color: Color::srgb(0.0, 1.0, 0.0),
                    },
                ],
            },
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_perf_hud demo".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BevyPerfHudPlugin)
        .add_systems(Startup, setup_3d)
        .add_systems(
            Update,
            (
                adjust_spawn_and_camera_keys,
                spawn_cube_on_space,
                clear_cubes_on_r,
            ),
        )
        .add_systems(Update, toggle_hud_mode_on_f1)
        .run();
}
