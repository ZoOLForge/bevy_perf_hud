use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use bevy_perf_hud::{
    BevyPerfHudPlugin, HudHandles,
    BarConfig, ProviderRegistry,
    BarsContainer, BarsHandles,
    GraphConfig, CurveConfig,
};

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum HudMode {
    #[default]
    Full,
    GraphOnly,
    Hidden,
}

#[derive(Resource, Default)]
struct CubeState {
    count: u32, // total number of spawned cubes
}

#[derive(Component)]
struct DemoCube;

#[derive(Component)]
struct MainCamera;

// No orbit: use a free-fly camera

#[derive(Resource)]
struct SpawnParams {
    batch: u32,       // cubes per batch
    spacing: f32,     // grid spacing
    jitter_frac: f32, // jitter as a fraction of grid size (0..1)
    min_dist: f32,    // min center distance along forward
    max_dist: f32,    // max center distance along forward
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

// Simple LCG PRNG (no extra dependencies)
#[derive(Resource)]
struct RngState {
    state: u64,
}

impl Default for RngState {
    fn default() -> Self {
        // Fixed seed; replace with time-based seed if per-run randomness is needed
        Self {
            state: 0x9E3779B97F4A7C15,
        }
    }
}

impl RngState {
    fn next_u64(&mut self) -> u64 {
        // 64-bit LCG constants
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }
    fn next_f32(&mut self) -> f32 {
        let v = self.next_u64() >> 40; // high bits, ~24-bit precision
        (v as f32) / ((1u32 << 24) as f32)
    }
    fn range_f32(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f32()
    }
}

fn setup_3d(mut commands: Commands) {
    let tf = Transform::from_xyz(-8.0, 8.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y);
    // 3D camera
    commands.spawn((Camera3d::default(), tf, MainCamera));
    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.6, 0.0)),
    ));
}

#[allow(clippy::too_many_arguments)]
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

    // Based on camera: center at camera.position + forward * distance,
    // and lay out a grid on the plane perpendicular to view (right/up) to keep it in view.
    let Some(t) = q_cam.single().ok() else {
        return;
    };
    let fwd = t.forward();
    let right = t.right();
    let up = t.up();
    // Random forward distance (configured), to avoid being too near/far and vary per batch
    let mut min_d = spawn.min_dist;
    let mut max_d = spawn.max_dist;
    if max_d < min_d {
        core::mem::swap(&mut min_d, &mut max_d);
    }
    min_d = min_d.clamp(0.5, 500.0);
    max_d = max_d.clamp(min_d + 0.5, 1000.0);
    let dist = rng.range_f32(min_d, max_d);
    let center = t.translation + fwd * dist;

    // Grid parameters
    let n = spawn.batch.max(1) as usize;
    let cols = (n as f32).sqrt().ceil() as usize;
    let rows = n.div_ceil(cols);
    let sx = spawn.spacing;

    // Planar jitter: randomly offset center within a fraction of grid width/height to avoid overlap
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
    // Adjust batch size: [ decrease, ] increase; hold Shift for larger step
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

    // Free-fly camera controls
    let mut tf = match q_cam.single_mut() {
        Ok(t) => t,
        Err(_) => return,
    };
    let boost = if big { 3.0 } else { 1.0 };
    let move_step = 0.4 * boost; // per-frame translation step
    let yaw_step = 0.06 * boost;
    let pitch_step = 0.04 * boost;

    // Capture direction vectors first to avoid E0502 borrow conflicts
    let fwd = tf.forward();
    let right = tf.right();
    let up = Vec3::Y;

    // Translate: W/S forward/back, A/D left/right, Q/E down/up
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

    // Rotate: arrows left/right = yaw, up/down = pitch
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
    hud_query: Query<&HudHandles>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }

    let Ok(h) = hud_query.single() else {
        return;
    };

    match *mode {
        HudMode::Full => {
            // Next state: graph only
            if let Some(e) = h.graph_row {
                commands.entity(e).insert(Visibility::Visible);
            }
            if let Some(e) = h.bars_root {
                commands.entity(e).insert(Visibility::Hidden);
            }
            *mode = HudMode::GraphOnly;
        }
        HudMode::GraphOnly => {
            // Next state: hidden
            if let Some(e) = h.graph_row {
                commands.entity(e).insert(Visibility::Hidden);
            }
            if let Some(e) = h.bars_root {
                commands.entity(e).insert(Visibility::Hidden);
            }
            *mode = HudMode::Hidden;
        }
        HudMode::Hidden => {
            // Next state: full
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

fn setup_hud(
    mut commands: Commands,
    provider_registry: Res<ProviderRegistry>,
    mut metric_registry: ResMut<bevy_perf_hud::MetricRegistry>,
) {
    use bevy_perf_hud::constants::{SYSTEM_CPU_USAGE_ID, SYSTEM_MEM_USAGE_ID};

    // UI 2D camera: render after 3D to avoid conflicts
    let ui_cam = commands.spawn(Camera2d).id();
    commands.entity(ui_cam).insert(Camera {
        order: 1,
        ..default()
    });

    // Register metric definitions with MetricRegistry so update_graph can find the colors
    for metric_id in ["frame_time_ms", "fps"] {
        if let Some(display_config) = provider_registry.get_display_config(metric_id) {
            metric_registry.register(bevy_perf_hud::MetricDefinition {
                id: metric_id.to_string(),
                label: display_config.label.clone(),
                unit: display_config.unit.clone(),
                precision: display_config.precision,
                color: display_config.color,
            });
        }
    }

    // Create GraphConfig - automatically includes GraphHandles, HistoryBuffers,
    // GraphScaleState, SampledValues, and Visibility via #[require]
    let graph_config = GraphConfig::default();

    // Create BarsContainer
    let bars_container = BarsContainer {
        column_count: 2,
        width: 300.0,
        row_height: 24.0,
    };

    // Cache layout values before moving bars_container
    let column_count = bars_container.column_count;
    let bars_width = bars_container.width;
    let row_height = bars_container.row_height;

    // Create root HUD entity with graph and bars components
    // GraphConfig brings in: GraphHandles, HistoryBuffers, GraphScaleState, SampledValues, Visibility
    // BarsContainer brings in: BarsHandles, BarMaterials, SampledValues, BarScaleStates
    let hud_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            graph_config,
            HudHandles::default(),
            bars_container,
        ))
        .with_children(|parent| {
            // Spawn CurveConfig entities as children of the graph container
            parent.spawn(CurveConfig {
                metric_id: "frame_time_ms".into(),
                autoscale: Some(true),
                smoothing: Some(0.2),
                quantize_step: Some(1.0),
            });

            parent.spawn(CurveConfig {
                metric_id: "fps".into(),
                autoscale: Some(true),
                smoothing: Some(0.2),
                quantize_step: Some(1.0),
            });
        })
        .id();

    // Configure bars with different scaling modes
    let bar_configs = vec![
        // CPU - fixed mode (0-100%) - default fallback is perfect here
        BarConfig::fixed_mode(SYSTEM_CPU_USAGE_ID),
        // Memory - fixed mode (0-100%) - default fallback is perfect here
        BarConfig::fixed_mode(SYSTEM_MEM_USAGE_ID),
        // FPS - percentile mode to handle spikes - custom fallback for higher range
        BarConfig::percentile_mode_with_fallback("fps", 0.0, 144.0),
        // Entity count - auto mode for dynamic range - custom fallback for expected range
        BarConfig::auto_mode_with_fallback("entity_count", 0.0, 10000.0),
    ];

    // Spawn individual BarConfig entities for each bar
    for bar_config in &bar_configs {
        commands.spawn(bar_config.clone());
    }

    // Calculate layout dimensions from cached values
    let total_height = (bar_configs.len() as f32 / column_count as f32).ceil() * row_height;

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

    // Update BarsHandles to point to bars_root BEFORE the automatic system runs
    // This tells initialize_bars_ui to create bars as children of bars_root
    commands.entity(hud_root).insert(BarsHandles {
        bars_root: Some(bars_root),
        bar_labels: vec![], // Will be populated by initialize_bars_ui
    });

    // The initialize_bars_ui and initialize_graph_ui systems will automatically create
    // all UI child entities based on BarConfig and CurveConfig entities.
    // No need to manually create graph UI - it will be handled automatically.

    // Update HudHandles for toggle_hud_mode_on_f1 to work
    // Note: Graph and bar elements will be populated by initialize_graph_ui and initialize_bars_ui
    commands.entity(hud_root).insert(HudHandles {
        root: Some(hud_root),
        graph_row: None, // Will be populated by initialize_graph_ui
        graph_entity: None, // Will be populated by initialize_graph_ui
        graph_material: None, // Will be populated by initialize_graph_ui
        graph_labels: vec![], // Will be populated by initialize_graph_ui
        graph_label_width: 0.0, // Will be populated by initialize_graph_ui
        bars_root: Some(bars_root),
        bar_materials: vec![], // Will be populated by initialize_bars_ui
        bar_labels: vec![], // Will be populated by initialize_bars_ui
    });
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<SpawnParams>()
        .init_resource::<RngState>()
        .init_resource::<CubeState>()
        .init_resource::<HudMode>()
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
        .add_systems(Startup, setup_hud) // Create HUD with custom bars
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
