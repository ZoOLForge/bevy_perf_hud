//! Component definitions for the bevy_perf_hud.
//!
//! This module contains all component types used by the HUD systems to store
//! state directly on entities instead of using global resources.

use bevy::{asset::Handle, ecs::entity::Entity, prelude::Component};
use std::collections::{HashMap, VecDeque};

use crate::{BarMaterial, MultiLineGraphMaterial, MAX_CURVES, MAX_SAMPLES};

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
    /// Entities for individual bar graphics
    pub bar_entities: Vec<Entity>,
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
    /// Root entity for the bars UI hierarchy
    pub root: Option<Entity>,
    /// Entity for the bars container
    pub bars_root: Option<Entity>,
    /// Entities for individual bar graphics
    pub bar_entities: Vec<Entity>,
    /// Material handles for bar shaders
    pub bar_materials: Vec<Handle<BarMaterial>>,
    /// Entities for bar label text
    pub bar_labels: Vec<Entity>,
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
        mode: &crate::config::BarScaleMode,
        fallback_min: f32,
        fallback_max: f32,
        min_limit: Option<f32>,
        max_limit: Option<f32>,
    ) -> (f32, f32) {
        let (target_min, target_max) = match mode {
            crate::config::BarScaleMode::Fixed => (fallback_min, fallback_max),
            crate::config::BarScaleMode::Auto {
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
            crate::config::BarScaleMode::Percentile {
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
