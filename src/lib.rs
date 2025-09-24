//! Bevy Performance HUD Plugin
//!
//! A comprehensive performance monitoring overlay for Bevy applications that displays:
//! - Real-time performance graphs with configurable metrics
//! - System resource usage bars (CPU, memory)
//! - Custom metric tracking with extensible provider system
//! - Configurable visual appearance and positioning

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

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of samples to store in the history buffer for graph rendering
const MAX_SAMPLES: usize = 256;

/// Maximum number of curves that can be displayed simultaneously in a graph
const MAX_CURVES: usize = 6;

/// Number of Vec4 elements needed to pack all samples for shader
const SAMPLES_VEC4: usize = MAX_SAMPLES / 4;

/// Metric ID for system-wide CPU usage percentage
const SYSTEM_CPU_USAGE_ID: &str = "system/cpu_usage";

/// Metric ID for system-wide memory usage percentage
const SYSTEM_MEM_USAGE_ID: &str = "system/mem_usage";

/// Metric ID for process-specific CPU usage percentage
const PROCESS_CPU_USAGE_ID: &str = "process/cpu_usage";

/// Metric ID for process-specific memory usage in bytes
const PROCESS_MEM_USAGE_ID: &str = "process/mem_usage";

// ============================================================================
// CORE PLUGIN
// ============================================================================

/// The main plugin for the performance HUD system.
///
/// This plugin automatically sets up all necessary components:
/// - Diagnostic plugins for gathering performance metrics
/// - UI material plugins for custom shaders
/// - Resource initialization for state management
/// - System registration for HUD updates
///
/// # Usage
/// ```rust
/// use bevy::prelude::*;
/// use bevy_perf_hud::BevyPerfHudPlugin;
///
/// App::new()
///     .add_plugins(BevyPerfHudPlugin)
///     .run();
/// ```
#[derive(Default)]
pub struct BevyPerfHudPlugin;

impl Plugin for BevyPerfHudPlugin {
    fn build(&self, app: &mut App) {
        // Add diagnostic plugins if not already present
        // These provide the core metrics like FPS, frame time, entity count, etc.
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        };

        if !app.is_plugin_added::<EntityCountDiagnosticsPlugin>() {
            app.add_plugins(EntityCountDiagnosticsPlugin);
        };

        if !app.is_plugin_added::<SystemInformationDiagnosticsPlugin>() {
            app.add_plugins(SystemInformationDiagnosticsPlugin);
        };

        // Register custom UI materials for graph and bar rendering
        // These use custom shaders for efficient real-time performance visualization
        app.add_plugins(UiMaterialPlugin::<MultiLineGraphMaterial>::default())
            .add_plugins(UiMaterialPlugin::<BarMaterial>::default())
            // Initialize core resources for HUD state management
            .init_resource::<SampledValues>() // Current metric values
            .init_resource::<MetricProviders>() // Registry of metric sources
            .init_resource::<HistoryBuffers>() // Historical data for graphs
            .init_resource::<GraphScaleState>() // Dynamic scaling state
            // Register systems for HUD lifecycle
            .add_systems(Startup, setup_hud) // Create HUD entities on startup
            .add_systems(Update, (sample_diagnostics, update_graph_and_bars).chain()); // Update loop

        // Register default metric providers (FPS, frame time, entity count, system info)
        app.world_mut()
            .resource_mut::<MetricProviders>()
            .ensure_default_entries();
    }
}

// ============================================================================
// CONFIGURATION/SETTINGS TYPES
// ============================================================================
//
// This section contains all the configuration structures that define how the
// performance HUD should appear and behave. These are typically set up once
// during application initialization and can be modified at runtime.

/// Main configuration resource for the performance HUD.
///
/// This resource controls all aspects of the HUD's appearance and behavior.
/// Insert this resource into your Bevy app to customize the HUD settings.
///
/// # Example
/// ```rust
/// use bevy::prelude::*;
/// use bevy_perf_hud::PerfHudSettings;
///
/// App::new()
///     .insert_resource(PerfHudSettings {
///         origin: Vec2::new(10.0, 10.0), // Top-left corner
///         ..default()
///     })
///     .run();
/// ```
#[derive(Resource)]
pub struct PerfHudSettings {
    /// Whether the HUD is currently enabled and visible
    pub enabled: bool,
    /// Screen position (in pixels) where the HUD should be anchored
    pub origin: Vec2,
    /// Configuration for the performance graph display
    pub graph: GraphSettings,
    /// Configuration for the performance bars display
    pub bars: BarsSettings,
}

impl Default for PerfHudSettings {
    fn default() -> Self {
        let frame_metric = MetricDefinition {
            id: "frame_time_ms".into(),
            label: Some("FT:".into()),
            unit: Some("ms".into()),
            precision: 1,
            color: Color::srgb(0.4, 0.4, 0.4),
        };
        let fps_metric = MetricDefinition {
            id: "fps".into(),
            label: Some("FPS:".into()),
            unit: Some("fps".into()),
            precision: 0,
            color: Color::srgb(1.0, 1.0, 1.0),
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
                size: Vec2::new(300.0, 80.0),
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

/// Configuration for the performance graph (chart) display.
///
/// Controls how performance metrics are visualized as time-series graphs,
/// including appearance, scaling behavior, and which metrics to show.
#[derive(Clone)]
pub struct GraphSettings {
    /// Whether the graph is enabled and should be rendered
    pub enabled: bool,
    /// Size of the graph area in pixels (width, height)
    pub size: Vec2,
    /// Width in pixels reserved for metric labels on the left side
    pub label_width: f32,
    /// Fixed minimum Y-axis value (used when autoscale is disabled)
    pub min_y: f32,
    /// Fixed maximum Y-axis value (used when autoscale is disabled)
    pub max_y: f32,
    /// Line thickness for graph curves (0.0-1.0 in normalized coordinates)
    pub thickness: f32,
    /// List of curves (metrics) to display on this graph
    pub curves: Vec<CurveConfig>,
    /// Default settings for curves that don't specify their own values
    pub curve_defaults: CurveDefaults,
    /// Background color of the graph area (supports transparency)
    pub bg_color: Color,
    /// Border configuration for the graph edges
    pub border: GraphBorder,
    /// Number of horizontal grid lines to display (minimum 2)
    pub y_ticks: u32,
    /// Whether to always include zero in the Y-axis range
    pub y_include_zero: bool,
    /// Minimum Y-axis range to prevent overly compressed scales
    pub y_min_span: f32,
    /// Additional margin around data as fraction of range (0.0-0.45)
    pub y_margin_frac: f32,
    /// Step size for quantizing Y-axis min/max values (0 = disabled)
    pub y_step_quantize: f32,
    /// Smoothing factor for Y-axis scale transitions (0.0-1.0)
    pub y_scale_smoothing: f32,
}

/// Configuration for graph border appearance.
#[derive(Clone)]
pub struct GraphBorder {
    /// Color of the border lines (supports transparency)
    pub color: Color,
    /// Thickness of border lines in pixels
    pub thickness: f32,
    /// Whether to draw the left border
    pub left: bool,
    /// Whether to draw the bottom border
    pub bottom: bool,
    /// Whether to draw the right border
    pub right: bool,
    /// Whether to draw the top border
    pub top: bool,
}

/// Configuration for the performance bars display.
///
/// Performance bars show current metric values as horizontal progress bars,
/// useful for displaying things like CPU usage, memory usage, etc.
#[derive(Clone)]
pub struct BarsSettings {
    /// Whether the bars are enabled and should be rendered
    pub enabled: bool,
    /// List of bars (metrics) to display
    pub bars: Vec<BarConfig>,
    /// Background color for all bars (supports transparency)
    pub bg_color: Color,
    /// Default setting for whether bars should show their numeric values
    pub show_value_default: bool,
}

/// Configuration for a single curve (line) in a performance graph.
///
/// Each curve represents one metric tracked over time, such as FPS or frame time.
#[derive(Clone)]
pub struct CurveConfig {
    /// The metric this curve represents (ID, label, color, etc.)
    pub metric: MetricDefinition,
    /// Whether this curve should use autoscaling (None = use graph default)
    pub autoscale: Option<bool>,
    /// Exponential smoothing factor 0.0-1.0 (None = use graph default)
    /// 0.0 = no smoothing, 1.0 = follow new values immediately
    pub smoothing: Option<f32>,
    /// Quantization step for values (None = use graph default)
    /// Values are rounded to nearest multiple of this step
    pub quantize_step: Option<f32>,
}

/// Default values for curve configuration options.
///
/// These values are used when individual curves don't specify their own settings.
#[derive(Clone)]
pub struct CurveDefaults {
    /// Default autoscale setting for curves
    pub autoscale: bool,
    /// Default smoothing factor for curves (0.0-1.0)
    pub smoothing: f32,
    /// Default quantization step for curve values
    pub quantize_step: f32,
}

/// Configuration for a single performance bar.
///
/// Each bar represents one metric displayed as a horizontal progress indicator.
#[derive(Clone)]
pub struct BarConfig {
    /// The metric this bar represents (ID, label, color, etc.)
    pub metric: MetricDefinition,
    /// Whether to show numeric value and unit (None = use bars default)
    pub show_value: Option<bool>,
}

/// Definition of a performance metric for display purposes.
///
/// This structure defines how a metric should be presented in the HUD,
/// including its visual appearance and formatting options.
#[derive(Clone)]
pub struct MetricDefinition {
    /// Unique identifier for this metric (must match provider metric_id)
    pub id: String,
    /// Display label for this metric (None = use ID as label)
    pub label: Option<String>,
    /// Unit string to show after values (e.g., "ms", "fps", "%")
    pub unit: Option<String>,
    /// Number of decimal places to display in values
    pub precision: u32,
    /// Color for this metric's curve/bar
    pub color: Color,
}

// ============================================================================
// RUNTIME STATE/RESOURCES
// ============================================================================
//
// This section contains resources and components that manage the runtime state
// of the performance HUD. These are created and maintained automatically by
// the plugin systems and typically don't need direct user interaction.

/// Handle to a graph label entity, linking it to its metric.
///
/// Used internally to update label text and colors for graph metrics.
#[derive(Clone)]
pub struct GraphLabelHandle {
    /// ID of the metric this label represents
    pub metric_id: String,
    /// Bevy entity ID for the text label
    pub entity: Entity,
}

/// Resource containing handles to all HUD-related entities and materials.
///
/// This resource is created automatically by the plugin and contains references
/// to all the UI entities and materials that make up the performance HUD.
/// Used internally by systems to update HUD appearance and content.
#[derive(Resource)]
pub struct HudHandles {
    /// Entity for the graph row container (contains labels + graph)
    pub graph_row: Option<Entity>,
    /// Entity for the actual graph rendering area
    pub graph_entity: Option<Entity>,
    /// Material handle for the graph shader
    pub graph_material: Option<Handle<MultiLineGraphMaterial>>,
    /// Handles to all graph label entities
    pub graph_labels: Vec<GraphLabelHandle>,
    /// Width allocated for graph labels in pixels
    pub graph_label_width: f32,
    /// Entity for the bars container
    pub bars_root: Option<Entity>,
    /// Entities for individual bar graphics
    pub bar_entities: Vec<Entity>,
    /// Material handles for bar shaders
    pub bar_materials: Vec<Handle<BarMaterial>>,
    /// Entities for bar label text
    pub bar_labels: Vec<Entity>,
}

/// Resource storing the most recent sampled values for all performance metrics.
///
/// This acts as a cache of current metric values, updated each frame by the
/// metric sampling system and consumed by the HUD rendering systems.
#[derive(Resource, Default)]
pub struct SampledValues {
    /// Map from metric ID to its current value
    values: HashMap<String, f32>,
}

impl SampledValues {
    /// Set the current value for a performance metric.
    ///
    /// # Arguments
    /// * `id` - The metric identifier
    /// * `value` - The new metric value
    pub fn set(&mut self, id: &str, value: f32) {
        if let Some(existing) = self.values.get_mut(id) {
            *existing = value;
        } else {
            self.values.insert(id.to_owned(), value);
        }
    }

    /// Get the current value for a performance metric.
    ///
    /// # Arguments
    /// * `id` - The metric identifier
    ///
    /// # Returns
    /// The current value if the metric exists, None otherwise
    pub fn get(&self, id: &str) -> Option<f32> {
        self.values.get(id).copied()
    }
}

/// Resource storing historical values for graph curve rendering.
///
/// Maintains a sliding window of historical values for each curve, used by
/// the graph shader to render time-series data. Values are stored in a
/// circular buffer format for efficient memory usage.
#[derive(Resource)]
pub struct HistoryBuffers {
    /// 2D array: \[curve_index\]\[sample_index\] containing historical values
    /// Each curve can store up to MAX_SAMPLES historical data points
    pub values: [[f32; MAX_SAMPLES]; MAX_CURVES],
    /// Number of valid samples currently stored (0 to MAX_SAMPLES)
    pub length: u32,
}

impl Default for HistoryBuffers {
    fn default() -> Self {
        Self {
            values: [[0.0; MAX_SAMPLES]; MAX_CURVES],
            length: 0,
        }
    }
}

/// Resource storing the current smoothed Y-axis scale for graphs.
///
/// When autoscaling is enabled, this maintains smoothed min/max values
/// to reduce visual jitter from rapid scale changes. The values are
/// interpolated over time to provide stable graph scaling.
#[derive(Resource, Default, Clone, Copy)]
pub struct GraphScaleState {
    /// Current smoothed minimum Y-axis value
    pub min_y: f32,
    /// Current smoothed maximum Y-axis value
    pub max_y: f32,
}

// ============================================================================
// METRIC PROVIDER SYSTEM
// ============================================================================
//
// This section implements the extensible metric provider system that allows
// the HUD to display both built-in and custom performance metrics. The system
// uses a trait-based approach for maximum flexibility and performance.

/// Context passed to metric providers during sampling.
///
/// Contains references to Bevy's diagnostic systems and other resources
/// that providers might need to calculate their metric values.
#[derive(Clone, Copy)]
pub struct MetricSampleContext<'a> {
    /// Reference to Bevy's diagnostics store for built-in metrics
    pub diagnostics: Option<&'a DiagnosticsStore>,
}

/// Trait for implementing custom performance metric providers.
///
/// This trait allows you to create custom metrics that can be displayed
/// in the performance HUD alongside built-in metrics like FPS and frame time.
///
/// # Example
/// ```rust
/// use bevy_perf_hud::{PerfMetricProvider, MetricSampleContext};
///
/// struct CustomMetricProvider {
///     counter: f32,
/// }
///
/// impl PerfMetricProvider for CustomMetricProvider {
///     fn metric_id(&self) -> &str {
///         "custom_metric"
///     }
///
///     fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
///         self.counter += 1.0;
///         Some(self.counter)
///     }
/// }
/// ```
pub trait PerfMetricProvider: Send + Sync + 'static {
    /// Returns the unique identifier for this metric.
    /// Must match the ID used in metric definitions.
    fn metric_id(&self) -> &str;

    /// Sample the current value of this metric.
    ///
    /// # Arguments
    /// * `ctx` - Context containing diagnostic data and other resources
    ///
    /// # Returns
    /// The current metric value, or None if unavailable
    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32>;
}

/// Resource managing the registry of all metric providers.
///
/// This resource maintains a collection of all metric providers (both built-in
/// and custom) and handles the sampling process during each frame update.
#[derive(Resource, Default)]
pub struct MetricProviders {
    /// Collection of all registered metric providers
    providers: Vec<Box<dyn PerfMetricProvider>>,
}

impl MetricProviders {
    /// Register a new metric provider.
    ///
    /// # Arguments
    /// * `provider` - The provider implementation to register
    pub fn add_provider<P: PerfMetricProvider>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }

    /// Check if a provider with the given metric ID is already registered.
    ///
    /// # Arguments
    /// * `id` - The metric ID to check for
    ///
    /// # Returns
    /// true if a provider for this metric exists, false otherwise
    pub fn contains(&self, id: &str) -> bool {
        self.providers.iter().any(|p| p.metric_id() == id)
    }

    /// Register all built-in metric providers if they haven't been added yet.
    ///
    /// This is called automatically by the plugin to ensure standard metrics
    /// (FPS, frame time, entity count, system resources) are available.
    pub fn ensure_default_entries(&mut self) {
        self.ensure_provider(FpsMetricProvider);
        self.ensure_provider(FrameTimeMetricProvider);
        self.ensure_provider(EntityCountMetricProvider);
        self.ensure_provider(SystemCpuUsageMetricProvider);
        self.ensure_provider(SystemMemUsageMetricProvider);
        self.ensure_provider(ProcessCpuUsageMetricProvider);
        self.ensure_provider(ProcessMemUsageMetricProvider);
    }

    /// Get a mutable iterator over all registered providers.
    ///
    /// Used internally by the sampling system to update metric values.
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

/// Extension trait for [`App`] to easily register custom metric providers.
///
/// This trait provides a convenient way to add custom metric providers
/// to your Bevy application without needing to manually access resources.
///
/// # Example
/// ```rust
/// use bevy::prelude::*;
/// use bevy_perf_hud::PerfHudAppExt;
///
/// App::new()
///     .add_perf_metric_provider(MyCustomProvider::default())
///     .run();
/// ```
pub trait PerfHudAppExt {
    /// Add a custom metric provider to the application.
    ///
    /// # Arguments
    /// * `provider` - The metric provider to register
    ///
    /// # Returns
    /// The app instance for method chaining
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

/// Built-in metric provider for frames per second (FPS).
///
/// Provides the current FPS value calculated by Bevy's frame time diagnostics.
/// The value is floored to the nearest integer for display purposes.
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

/// Built-in metric provider for frame time in milliseconds.
///
/// Provides the smoothed frame time duration from Bevy's diagnostics,
/// converted to milliseconds and floored to the nearest integer.
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

/// Built-in metric provider for the total number of entities.
///
/// Provides the current entity count from Bevy's entity diagnostics.
/// Useful for monitoring memory usage and performance impact of entities.
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

/// Built-in metric provider for system-wide CPU usage percentage.
///
/// Provides the overall CPU usage across all cores and processes,
/// as reported by Bevy's system information diagnostics.
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

/// Built-in metric provider for system-wide memory usage percentage.
///
/// Provides the overall memory usage as a percentage of total system RAM,
/// as reported by Bevy's system information diagnostics.
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

/// Built-in metric provider for process-specific CPU usage percentage.
///
/// Provides the CPU usage of the current Bevy application process,
/// as reported by Bevy's system information diagnostics.
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

/// Built-in metric provider for process-specific memory usage in bytes.
///
/// Provides the memory usage of the current Bevy application process,
/// as reported by Bevy's system information diagnostics.
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

// ============================================================================
// SHADER MODULE
// ============================================================================
//
// This module contains the custom UI materials and shader parameters for
// rendering performance graphs and bars. The shaders are optimized for
// real-time display of performance data with minimal CPU overhead.

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

// ============================================================================
// SYSTEM FUNCTIONS
// ============================================================================
//
// This section contains the core Bevy systems that manage the HUD lifecycle:
// - setup_hud: Creates all UI entities and materials during startup
// - sample_diagnostics: Updates metric values each frame
// - update_graph_and_bars: Renders current data to the HUD display

/// Startup system that creates all HUD UI entities and materials.
///
/// This system runs once during application startup and creates:
/// - UI camera for HUD rendering
/// - Root UI container positioned according to settings
/// - Graph entities with custom materials and labels
/// - Bar entities with materials and labels
/// - HudHandles resource containing all entity references
///
/// The system only runs if PerfHudSettings is present and enabled.
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
        #[allow(clippy::field_reassign_with_default)]
        {
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

    // Bars container placed below the graph
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

/// System that samples all registered metric providers and updates current values.
///
/// This system runs every frame and:
/// - Calls the sample() method on all registered metric providers
/// - Updates the SampledValues resource with fresh metric data
/// - Provides the foundation for graph and bar rendering
///
/// The system only runs if PerfHudSettings is present and enabled.
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

/// System that updates graph and bar displays with current performance data.
///
/// This system runs every frame after sample_diagnostics and handles:
/// - Processing raw metric values (smoothing, quantization)
/// - Managing historical data buffers for graph curves
/// - Calculating dynamic Y-axis scaling for graphs
/// - Updating shader material parameters for rendering
/// - Refreshing label text and colors
/// - Normalizing bar values for display
///
/// The system implements sophisticated features like:
/// - Exponential smoothing to reduce noise in metric values
/// - Autoscaling with smoothed transitions to prevent jitter
/// - Quantization for cleaner value display
/// - Efficient circular buffer management for historical data
///
/// The system only runs if both PerfHudSettings and HudHandles are present.
#[allow(clippy::too_many_arguments)]
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

    // Process raw metric values through smoothing and quantization pipeline
    let mut filtered_values = [0.0_f32; MAX_CURVES];
    for (i, cfg) in s.graph.curves.iter().take(curve_count).enumerate() {
        let raw = samples.get(cfg.metric.id.as_str()).unwrap_or(0.0);

        // Step 1: Apply exponential smoothing to reduce noise
        // Formula: new_value = prev_value + (raw_value - prev_value) * smoothing_factor
        let smoothing = cfg
            .smoothing
            .unwrap_or(s.graph.curve_defaults.smoothing)
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
            .unwrap_or(s.graph.curve_defaults.quantize_step);
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
    let mut target_min = s.graph.min_y;
    let mut target_max = s.graph.max_y;

    // Check if any curves want autoscaling and we have historical data
    if s.graph
        .curves
        .iter()
        .any(|c| c.autoscale.unwrap_or(s.graph.curve_defaults.autoscale))
        && history.length > 0
    {
        // Scan all historical data to find the actual min/max range
        let len = history.length as usize;
        let mut mn = f32::INFINITY;
        let mut mx = f32::NEG_INFINITY;

        for (i, cfg) in s.graph.curves.iter().take(curve_count).enumerate() {
            // Only include curves that want autoscaling in the calculation
            if cfg.autoscale.unwrap_or(s.graph.curve_defaults.autoscale) {
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
