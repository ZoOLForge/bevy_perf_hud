//! Runtime resources for the bevy_perf_hud.
//!
//! This module contains all runtime state resources used by the HUD systems.

use bevy::{asset::Handle, ecs::entity::Entity, prelude::Resource};
use std::collections::HashMap;

use crate::{BarMaterial, MultiLineGraphMaterial, MAX_CURVES, MAX_SAMPLES};

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
