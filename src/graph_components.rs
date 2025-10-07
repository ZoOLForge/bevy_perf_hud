//! Graph-related component definitions for the bevy_perf_hud.
//!
//! This module contains all component types used by the graph rendering systems.

use bevy::prelude::Visibility;
use bevy::{asset::Handle, color::Color, ecs::entity::Entity, math::Vec2, prelude::Component};

use crate::{MultiLineGraphMaterial, MAX_CURVES, MAX_SAMPLES};

/// Handle to a graph label entity, linking it to its metric.
///
/// Used internally to update label text and colors for graph metrics.
#[derive(Clone, Component)]
pub struct GraphLabelHandle {
    /// ID of the metric this label represents
    pub metric_id: String,
    /// Bevy entity ID for the text label
    pub entity: Entity,
}

/// Component containing handles to graph-related entities and materials.
///
/// This component is placed on graph entities and contains references
/// to all the UI entities and materials that make up the performance graph.
/// Used internally by the graph update system.
#[derive(Component, Default)]
pub struct GraphHandles {
    /// Root entity for the graph UI hierarchy
    pub root: Option<Entity>,
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
}

/// Component storing historical values for graph curve rendering.
///
/// Maintains a sliding window of historical values for each curve, used by
/// the graph shader to render time-series data. Values are stored in a
/// circular buffer format for efficient memory usage.
#[derive(Component)]
pub struct HistoryBuffers {
    /// 2D array: [curve_index][sample_index] containing historical values
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

/// Component storing the current smoothed Y-axis scale for graphs.
///
/// When autoscaling is enabled, this maintains smoothed min/max values
/// to reduce visual jitter from rapid scale changes. The values are
/// interpolated over time to provide stable graph scaling.
#[derive(Component, Default, Clone, Copy)]
pub struct GraphScaleState {
    /// Current smoothed minimum Y-axis value
    pub min_y: f32,
    /// Current smoothed maximum Y-axis value
    pub max_y: f32,
}

/// Configuration for a single curve (line) in a performance graph.
///
/// Each curve represents one metric tracked over time, such as FPS or frame time.
#[derive(Component, Debug, Clone)]
pub struct CurveConfig {
    /// ID of the metric this curve represents (must reference a MetricDefinition component)
    pub metric_id: String,
    /// Whether this curve should use autoscaling (None = use graph default)
    pub autoscale: Option<bool>,
    /// Exponential smoothing factor 0.0-1.0 (None = use graph default)
    /// 0.0 = no smoothing, 1.0 = follow new values immediately
    pub smoothing: Option<f32>,
    /// Quantization step for values (None = use graph default)
    /// Values are rounded to nearest multiple of this step
    pub quantize_step: Option<f32>,
}

impl CurveConfig {
    /// Get the metric ID for this curve
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
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

/// Component storing configuration for the performance graph display.
///
/// This component automatically includes all required components for graph rendering
/// using Bevy 0.15's Required Components feature. Simply add this component to
/// an entity and Bevy will automatically attach:
/// - `GraphHandles`: Entity handles for graph UI elements
/// - `HistoryBuffers`: Historical data for curves
/// - `GraphScaleState`: Dynamic Y-axis scaling state
/// - `SampledValues`: Current metric values cache
/// - `Visibility`: UI visibility control
///
/// Curves are defined as separate CurveConfig component entities as children.
#[derive(Component, Debug, Clone)]
#[require(
    GraphHandles,
    HistoryBuffers,
    GraphScaleState,
    super::SampledValues,
    Visibility
)]
pub struct GraphConfig {
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

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            size: Vec2::new(300.0, 80.0),
            label_width: 60.0,
            min_y: 0.0,
            max_y: 30.0,
            thickness: 0.012,
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
        }
    }
}
