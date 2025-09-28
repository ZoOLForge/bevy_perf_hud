//! Configuration structures for the bevy_perf_hud.
//!
//! This module contains all configuration types for customizing the performance HUD.

use crate::constants::*;
use bevy::{color::Color, math::Vec2, prelude::Resource};

/// Main configuration resource for the performance HUD.
///
/// This resource controls all aspects of the HUD's appearance and behavior.
/// Insert this resource into your Bevy app to customize the HUD settings.
/// Note: The HUD is always active when the plugin is added, individual components (graph/bars) have their own enabled flags.
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
#[derive(Debug, Resource)]
pub struct PerfHudSettings {
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
                        min_value: 0.0,
                        max_value: 100.0,                // CPU usage percentage
                        scale_mode: BarScaleMode::Fixed, // Keep fixed for CPU % (known 0-100% range)
                        min_limit: None,
                        max_limit: None,
                    },
                    BarConfig {
                        metric: sys_mem_metric,
                        show_value: Some(false),
                        min_value: 0.0,
                        max_value: 100.0,                // Memory usage percentage
                        scale_mode: BarScaleMode::Fixed, // Keep fixed for memory % (known 0-100% range)
                        min_limit: None,
                        max_limit: None,
                    },
                    BarConfig {
                        metric: entity_metric,
                        show_value: None,
                        min_value: 0.0,
                        max_value: 10000.0, // Entity count range - fallback values
                        scale_mode: BarScaleMode::Auto {
                            smoothing: 0.85,  // Smooth transitions for entity count changes
                            min_span: 50.0,   // Minimum range of 50 entities
                            margin_frac: 0.2, // 20% margin for growth headroom
                        },
                        min_limit: Some(0.0),     // Entities can't be negative
                        max_limit: Some(50000.0), // Cap at reasonable maximum
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct CurveDefaults {
    /// Default autoscale setting for curves
    pub autoscale: bool,
    /// Default smoothing factor for curves (0.0-1.0)
    pub smoothing: f32,
    /// Default quantization step for curve values
    pub quantize_step: f32,
}

/// Bar scaling mode determines how the bar range is calculated.
#[derive(Debug, Clone, PartialEq)]
pub enum BarScaleMode {
    /// Fixed range using min_value and max_value (default behavior)
    Fixed,
    /// Automatic range adjustment based on historical data
    Auto {
        /// Smoothing factor for range changes (0.0 = instant, 1.0 = never change)
        smoothing: f32,
        /// Minimum span between min and max to avoid division by zero
        min_span: f32,
        /// Margin fraction to add above and below data range (0.0-0.5)
        margin_frac: f32,
    },
    /// Range based on percentiles of recent data
    Percentile {
        /// Lower percentile (e.g., 5.0 for P5)
        lower: f32,
        /// Upper percentile (e.g., 95.0 for P95)
        upper: f32,
        /// Number of recent samples to consider
        sample_count: usize,
    },
}

impl Default for BarScaleMode {
    fn default() -> Self {
        Self::Fixed
    }
}

/// Configuration for a single performance bar.
///
/// Each bar represents one metric displayed as a horizontal progress indicator.
#[derive(Debug, Clone)]
pub struct BarConfig {
    /// The metric this bar represents (ID, label, color, etc.)
    pub metric: MetricDefinition,
    /// Whether to show numeric value and unit (None = use bars default)
    pub show_value: Option<bool>,
    /// Minimum value for bar normalization (0% fill) - used in Fixed mode or as hard limit
    pub min_value: f32,
    /// Maximum value for bar normalization (100% fill) - used in Fixed mode or as hard limit
    pub max_value: f32,
    /// How to calculate the bar's value range
    pub scale_mode: BarScaleMode,
    /// Hard minimum limit (values below this are clamped) - optional override
    pub min_limit: Option<f32>,
    /// Hard maximum limit (values above this are clamped) - optional override
    pub max_limit: Option<f32>,
}

/// Definition of a performance metric for display purposes.
///
/// This structure defines how a metric should be presented in the HUD,
/// including its visual appearance and formatting options.
#[derive(Debug, Clone)]
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
