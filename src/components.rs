//! Component definitions for the bevy_perf_hud.
//!
//! This module contains all component types used by the HUD systems to store
//! state directly on entities instead of using global resources.

use bevy::prelude::Visibility;
use bevy::{asset::Handle, ecs::entity::Entity, prelude::{Component, Resource}, color::Color, math::Vec2};
use std::collections::{HashMap, VecDeque};

use crate::{BarMaterial, MultiLineGraphMaterial, MAX_CURVES, MAX_SAMPLES, constants::*};

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

/// Component containing handles to all HUD-related entities and materials.
///
/// This component is placed on the root HUD entity and contains references
/// to all the UI entities and materials that make up the performance HUD.
/// Used internally by systems to update HUD appearance and content.
#[derive(Component, Default)]
pub struct HudHandles {
    /// Root entity for the entire HUD UI hierarchy
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
    /// Entity for the bars container
    pub bars_root: Option<Entity>,
    /// Material handles for bar shaders
    pub bar_materials: Vec<Handle<BarMaterial>>,
    /// Entities for bar label text
    pub bar_labels: Vec<Entity>,
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

/// Component containing handles to bars-related entities and materials.
///
/// This component is placed on bars entities and contains references
/// to all the UI entities and materials that make up the performance bars.
/// Used internally by the bars update system.
#[derive(Component, Default)]
pub struct BarsHandles {
    /// Entity for the bars container
    pub bars_root: Option<Entity>,
    /// Entities for bar label text
    pub bar_labels: Vec<Entity>,
}

/// Component storing material handles for bar rendering.
///
/// This component contains the material handles used to render performance bars.
/// It's separate from BarsHandles to allow more granular querying and updating.
#[derive(Component, Default)]
pub struct BarMaterials {
    /// Material handles for bar shaders
    pub materials: Vec<Handle<BarMaterial>>,
}

/// Container component for bar layout configuration and management.
///
/// This component automatically includes all required components for bar rendering
/// using Bevy 0.15's Required Components feature. Simply add this component to
/// an entity and Bevy will automatically attach:
/// - `BarsHandles`: Entity handles for bars UI elements
/// - `BarMaterials`: Material handles for bar shaders
/// - `SampledValues`: Current metric values cache
/// - `BarScaleStates`: Dynamic scaling state for bars
#[derive(Component)]
#[require(BarsHandles, BarMaterials, SampledValues, BarScaleStates, Visibility)]
pub struct BarsContainer {
    /// Number of columns in the bar grid layout
    pub column_count: usize,
    /// Total width of the bar container in pixels
    pub width: f32,
    /// Height of each bar row in pixels
    pub row_height: f32,
}

impl Default for BarsContainer {
    fn default() -> Self {
        Self {
            column_count: 2,
            width: 300.0,
            row_height: 24.0,
        }
    }
}

impl BarMaterials {
    /// Create new BarMaterials with empty materials list
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
        }
    }
    
    /// Push a new material handle to the list
    pub fn push(&mut self, material: Handle<BarMaterial>) {
        self.materials.push(material);
    }
    
    /// Get a material handle by index
    pub fn get(&self, index: usize) -> Option<&Handle<BarMaterial>> {
        self.materials.get(index)
    }
    
    /// Get a mutable reference to a material handle by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Handle<BarMaterial>> {
        self.materials.get_mut(index)
    }
    
    /// Get the number of materials
    pub fn len(&self) -> usize {
        self.materials.len()
    }
    
    /// Check if there are no materials
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}

impl std::ops::Index<usize> for BarMaterials {
    type Output = Handle<BarMaterial>;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.materials[index]
    }
}

impl std::ops::IndexMut<usize> for BarMaterials {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.materials[index]
    }
}

/// Component storing the most recent sampled values for all performance metrics.
///
/// This acts as a cache of current metric values, updated each frame by the
/// metric sampling system and consumed by the HUD rendering systems.
#[derive(Component, Default)]
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

/// State for tracking dynamic bar scaling
#[derive(Debug, Clone, Component)]
pub struct BarScaleState {
    /// Current minimum value for normalization
    pub current_min: f32,
    /// Current maximum value for normalization
    pub current_max: f32,
    /// Historical values for auto/percentile calculation
    pub history: VecDeque<f32>,
    /// Maximum number of samples to keep in history
    pub max_samples: usize,
}

impl Default for BarScaleState {
    fn default() -> Self {
        Self {
            current_min: 0.0,
            current_max: 1.0,
            history: VecDeque::new(),
            max_samples: 120, // ~2 seconds at 60fps
        }
    }
}

impl BarScaleState {
    /// Create a new scale state with specified history size
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples,
            ..Default::default()
        }
    }

    /// Add a new sample to the history
    pub fn add_sample(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }

        self.history.push_back(value);

        // Keep only the most recent samples
        while self.history.len() > self.max_samples {
            self.history.pop_front();
        }
    }

    /// Calculate the range based on the configured scale mode
    pub fn calculate_range(
        &mut self,
        mode: &BarScaleMode,
        fallback_min: f32,
        fallback_max: f32,
        min_limit: Option<f32>,
        max_limit: Option<f32>,
    ) -> (f32, f32) {
        let (target_min, target_max) = match mode {
            BarScaleMode::Fixed => (fallback_min, fallback_max),
            BarScaleMode::Auto {
                smoothing,
                min_span,
                margin_frac,
            } => self.calculate_auto_range(
                *smoothing,
                *min_span,
                *margin_frac,
                fallback_min,
                fallback_max,
            ),
            BarScaleMode::Percentile {
                lower,
                upper,
                sample_count,
            } => self.calculate_percentile_range(
                *lower,
                *upper,
                *sample_count,
                fallback_min,
                fallback_max,
            ),
        };

        // Apply hard limits if specified
        let final_min = match min_limit {
            Some(limit) => target_min.max(limit),
            None => target_min,
        };
        let final_max = match max_limit {
            Some(limit) => target_max.min(limit),
            None => target_max,
        };

        // Ensure valid range, but respect hard limits
        let final_max = if final_max < final_min {
            // If limits conflict, adjust final_min down to final_max
            self.current_min = final_max - 1e-6;
            final_max
        } else {
            final_max.max(final_min + 1e-6)
        };
        let final_min = if final_max < final_min {
            final_max - 1e-6
        } else {
            final_min
        };

        self.current_min = final_min;
        self.current_max = final_max;

        (final_min, final_max)
    }

    /// Calculate automatic range based on data statistics
    fn calculate_auto_range(
        &self,
        smoothing: f32,
        min_span: f32,
        margin_frac: f32,
        fallback_min: f32,
        fallback_max: f32,
    ) -> (f32, f32) {
        if self.history.is_empty() {
            return (fallback_min, fallback_max);
        }

        // Calculate data range
        let mut data_min = f32::INFINITY;
        let mut data_max = f32::NEG_INFINITY;

        for &value in &self.history {
            data_min = data_min.min(value);
            data_max = data_max.max(value);
        }

        if !data_min.is_finite() || !data_max.is_finite() {
            return (fallback_min, fallback_max);
        }

        // Ensure minimum span
        let span = (data_max - data_min).max(min_span.max(1e-3));
        if data_max - data_min < span {
            let mid = 0.5 * (data_max + data_min);
            data_min = mid - 0.5 * span;
            data_max = mid + 0.5 * span;
        }

        // Add margins
        let margin = span * margin_frac.clamp(0.0, 0.45);
        let target_min = data_min - margin;
        let target_max = data_max + margin;

        // Apply smoothing
        let smoothing = smoothing.clamp(0.0, 1.0);
        if self.current_max <= self.current_min {
            // First time, use target values directly
            (target_min, target_max)
        } else {
            // Smooth transition from current to target
            let new_min = self.current_min + (target_min - self.current_min) * (1.0 - smoothing);
            let new_max = self.current_max + (target_max - self.current_max) * (1.0 - smoothing);
            (new_min, new_max)
        }
    }

    /// Calculate range based on percentiles of recent data
    fn calculate_percentile_range(
        &self,
        lower_percentile: f32,
        upper_percentile: f32,
        sample_count: usize,
        fallback_min: f32,
        fallback_max: f32,
    ) -> (f32, f32) {
        let samples_to_use = sample_count.min(self.history.len());
        if samples_to_use < 2 {
            return (fallback_min, fallback_max);
        }

        // Get most recent samples
        let mut recent_values: Vec<f32> = self
            .history
            .iter()
            .rev()
            .take(samples_to_use)
            .copied()
            .collect();

        recent_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let lower_idx = ((lower_percentile / 100.0) * (recent_values.len() - 1) as f32) as usize;
        let upper_idx = ((upper_percentile / 100.0) * (recent_values.len() - 1) as f32) as usize;

        let p_min = recent_values[lower_idx.min(recent_values.len() - 1)];
        let p_max = recent_values[upper_idx.min(recent_values.len() - 1)];

        (p_min, p_max.max(p_min + 1e-6))
    }

    /// Get the current normalization range
    pub fn get_current_range(&self) -> (f32, f32) {
        (self.current_min, self.current_max)
    }

    /// Normalize a value using the current range
    pub fn normalize_value(&self, value: f32) -> f32 {
        if self.current_max <= self.current_min {
            return 0.0;
        }

        ((value - self.current_min) / (self.current_max - self.current_min)).clamp(0.0, 1.0)
    }

    /// Clear the history (useful when switching modes)
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.current_min = 0.0;
        self.current_max = 1.0;
    }

    /// Get the number of samples in history
    pub fn sample_count(&self) -> usize {
        self.history.len()
    }

    /// Check if we have enough samples for reliable range calculation
    pub fn has_sufficient_data(&self, min_required: usize) -> bool {
        self.history.len() >= min_required
    }
}

// ============================================================================
// Configuration Types (formerly from config.rs and hud_settings_components.rs)
// ============================================================================

/// Definition of a performance metric for display purposes.
///
/// This structure defines how a metric should be presented in the HUD,
/// including its visual appearance and formatting options.
#[derive(Debug, Clone, Component)]
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

/// Configuration for a single curve (line) in a performance graph.
///
/// Each curve represents one metric tracked over time, such as FPS or frame time.
#[derive(Debug, Clone)]
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
#[derive(Component, Debug, Clone)]
pub struct BarConfig {
    /// ID of the metric this bar represents (must reference a MetricDefinition component)
    pub metric_id: String,
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
    /// Background color for this bar (supports transparency)
    pub bg_color: Color,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            metric_id: "default".to_owned(),
            show_value: Some(true),
            min_value: 0.0,
            max_value: 100.0,
            scale_mode: BarScaleMode::Fixed,
            min_limit: None,
            max_limit: None,
            bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
        }
    }
}

impl BarConfig {
    /// Get the metric ID for this bar
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
    
    /// Create a fixed mode bar configuration - traditional static range
    /// 
    /// This mode uses a fixed min/max range for normalization, which is 
    /// ideal for metrics with known bounds like percentages (0-100%).
    /// 
    /// # Arguments
    /// * `metric_id` - The ID of the metric this bar represents
    /// * `min_value` - Minimum value (0% fill)
    /// * `max_value` - Maximum value (100% fill)
    /// 
    /// # Example
    /// ```
    /// let bar_config = BarConfig::fixed_mode("cpu_usage", 0.0, 100.0);
    /// ```
    pub fn fixed_mode(metric_id: impl Into<String>, min_value: f32, max_value: f32) -> Self {
        Self {
            metric_id: metric_id.into(),
            show_value: Some(true),
            min_value,
            max_value,
            scale_mode: BarScaleMode::Fixed,
            min_limit: None,
            max_limit: None,
            bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
        }
    }
    
    /// Create an auto mode bar configuration - adapts to data range with smoothing
    /// 
    /// This mode automatically adjusts the range based on historical data,
    /// with smoothing to prevent rapid fluctuations. Good for metrics like
    /// entity counts that vary significantly over time.
    /// 
    /// # Arguments
    /// * `metric_id` - The ID of the metric this bar represents
    /// * `fallback_min` - Fallback minimum value if no data
    /// * `fallback_max` - Fallback maximum value if no data
    /// 
    /// # Example
    /// ```
    /// let bar_config = BarConfig::auto_mode("entity_count", 0.0, 10000.0);
    /// ```
    pub fn auto_mode(metric_id: impl Into<String>, fallback_min: f32, fallback_max: f32) -> Self {
        Self {
            metric_id: metric_id.into(),
            show_value: Some(true),
            min_value: fallback_min,
            max_value: fallback_max,
            scale_mode: BarScaleMode::Auto {
                smoothing: 0.8,   // Moderate smoothing
                min_span: 50.0,   // Minimum range span
                margin_frac: 0.1, // 10% margin
            },
            min_limit: None,
            max_limit: None,
            bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
        }
    }
    
    /// Create a percentile mode bar configuration - uses P5 to P95 range
    /// 
    /// This mode uses percentiles of recent data to determine the range,
    /// which is excellent for handling spiky metrics like latency where
    /// you want to ignore outliers.
    /// 
    /// # Arguments
    /// * `metric_id` - The ID of the metric this bar represents
    /// * `fallback_min` - Fallback minimum value if insufficient data
    /// * `fallback_max` - Fallback maximum value if insufficient data
    /// 
    /// # Example
    /// ```
    /// let bar_config = BarConfig::percentile_mode("network_latency", 0.0, 200.0);
    /// ```
    pub fn percentile_mode(metric_id: impl Into<String>, fallback_min: f32, fallback_max: f32) -> Self {
        Self {
            metric_id: metric_id.into(),
            show_value: Some(true),
            min_value: fallback_min,
            max_value: fallback_max,
            scale_mode: BarScaleMode::Percentile {
                lower: 5.0,       // P5 percentile
                upper: 95.0,      // P95 percentile
                sample_count: 60, // Last 60 samples
            },
            min_limit: None,
            max_limit: None,
            bg_color: Color::srgba(0.12, 0.12, 0.12, 0.6),
        }
    }
}

impl CurveConfig {
    /// Get the metric ID for this curve
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }
}

/// A resource that manages the mapping between metric IDs and their definitions
#[derive(Resource, Default)]
pub struct MetricRegistry {
    metrics: HashMap<String, MetricDefinition>,
}

impl MetricRegistry {
    /// Register a metric definition
    pub fn register(&mut self, metric: MetricDefinition) {
        self.metrics.insert(metric.id.clone(), metric);
    }

    /// Get a metric definition by ID
    pub fn get(&self, id: &str) -> Option<&MetricDefinition> {
        self.metrics.get(id)
    }

    /// Register default metrics used by the system
    pub fn register_defaults(&mut self) {
        // Frame time metric
        self.register(MetricDefinition {
            id: "frame_time_ms".into(),
            label: Some("FT:".into()),
            unit: Some("ms".into()),
            precision: 1,
            color: Color::srgb(0.4, 0.4, 0.4),
        });

        // FPS metric
        self.register(MetricDefinition {
            id: "fps".into(),
            label: Some("FPS:".into()),
            unit: Some("fps".into()),
            precision: 0,
            color: Color::srgb(1.0, 1.0, 1.0),
        });

        // System CPU usage
        self.register(MetricDefinition {
            id: SYSTEM_CPU_USAGE_ID.to_owned(),
            label: Some("SysCPU".into()),
            unit: Some("%".into()),
            precision: 1,
            color: Color::srgb(0.96, 0.76, 0.18),
        });

        // System memory usage
        self.register(MetricDefinition {
            id: SYSTEM_MEM_USAGE_ID.to_owned(),
            label: Some("SysMem".into()),
            unit: Some("%".into()),
            precision: 1,
            color: Color::srgb(0.28, 0.56, 0.89),
        });

        // Entity count
        self.register(MetricDefinition {
            id: "entity_count".into(),
            label: Some("Ent:".into()),
            unit: None,
            precision: 0,
            color: Color::srgb(0.1, 0.8, 0.4),
        });
    }
}

/// Configuration for the performance graph (chart) display.
///
/// Controls how performance metrics are visualized as time-series graphs,
/// including appearance, scaling behavior, and which metrics to show.
#[derive(Debug, Clone)]
pub struct GraphSettings {
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



impl GraphSettings {
    /// Convert GraphSettings to MultiLineGraphParams with initial values
    pub fn to_graph_params(&self) -> crate::render::MultiLineGraphParams {
        use crate::render::MultiLineGraphParams;
        use bevy::color::ColorToComponents;

        let mut params = MultiLineGraphParams::default();
        #[allow(clippy::field_reassign_with_default)]
        {
            params.min_y = self.min_y;
            params.max_y = self.max_y;
            params.thickness = self.thickness;
            params.bg_color = self.bg_color.to_linear().to_vec4();
            params.border_color = self.border.color.to_linear().to_vec4();
            params.border_thickness = self.border.thickness; // pixels
            params.border_thickness_uv_x =
                (self.border.thickness / self.size.x).max(0.0001);
            params.border_thickness_uv_y =
                (self.border.thickness / self.size.y).max(0.0001);
            params.border_left = if self.border.left { 1 } else { 0 };
            params.border_bottom = if self.border.bottom { 1 } else { 0 };
            params.border_right = if self.border.right { 1 } else { 0 };
            params.border_top = if self.border.top { 1 } else { 0 };
            params.curve_count = self.curves.len().min(crate::MAX_CURVES) as u32;
            // TODO: Need to redesign this method to accept MetricDefinition components
            // Write curve colors - currently disabled due to refactoring
            // for (i, c) in self.curves.iter().take(crate::MAX_CURVES).enumerate() {
            //     // Need to query MetricDefinition component by c.metric_id
            //     let v = metric_def.color.to_linear().to_vec4();
            //     params.colors[i] = v;
            // }
        }

        params
    }
}

impl Default for GraphSettings {
    fn default() -> Self {
        Self {
            size: Vec2::new(300.0, 80.0),
            label_width: 60.0,
            min_y: 0.0,
            max_y: 30.0,
            thickness: 0.012,
            curves: vec![
                CurveConfig {
                    metric_id: "frame_time_ms".into(),
                    autoscale: None,
                    smoothing: Some(0.25),
                    quantize_step: Some(0.1),
                },
                CurveConfig {
                    metric_id: "fps".into(),
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
        }
    }
}



// ============================================================================
// Component Types (formerly from hud_settings_components.rs)
// ============================================================================

/// Component storing configuration for the performance graph display.
#[derive(Component, Debug, Clone)]
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

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            size: Vec2::new(300.0, 80.0),
            label_width: 60.0,
            min_y: 0.0,
            max_y: 30.0,
            thickness: 0.012,
            curves: vec![
                CurveConfig {
                    metric_id: "frame_time_ms".into(),
                    autoscale: None,
                    smoothing: Some(0.25),
                    quantize_step: Some(0.1),
                },
                CurveConfig {
                    metric_id: "fps".into(),
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
        }
    }
}

/// Component storing configuration for the performance bars display.


/// Component storing dynamic scaling states for all performance bars.
/// Maps from metric ID to its scaling state
#[derive(Component, Default)]
pub struct BarScaleStates {
    /// Map from metric ID to its scaling state
    states: HashMap<String, BarScaleState>,
}

impl BarScaleStates {
    /// Get mutable reference to a bar's scale state, creating it if needed
    pub fn get_or_create(&mut self, metric_id: &str) -> &mut BarScaleState {
        self.states.entry(metric_id.to_owned()).or_default()
    }

    /// Get reference to a bar's scale state if it exists
    pub fn get(&self, metric_id: &str) -> Option<&BarScaleState> {
        self.states.get(metric_id)
    }

    /// Clear all scaling states (useful when configuration changes)
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Remove a specific bar's scaling state
    pub fn remove(&mut self, metric_id: &str) -> Option<BarScaleState> {
        self.states.remove(metric_id)
    }
}
