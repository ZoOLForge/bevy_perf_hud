//! Shared component definitions for the bevy_perf_hud.
//!
//! This module contains component types that are shared across both
//! graph and bar rendering systems.

use bevy::{
    color::Color,
    prelude::{Component, Resource},
};
use std::collections::HashMap;

use crate::constants::*;

/// Component containing handles to all HUD-related entities and materials.
///
/// This component is placed on the root HUD entity and contains references
/// to all the UI entities and materials that make up the performance HUD.
/// Used internally by systems to update HUD appearance and content.
#[derive(Component, Default)]
pub struct HudHandles {
    /// Root entity for the entire HUD UI hierarchy
    pub root: Option<bevy::ecs::entity::Entity>,
    /// Entity for the graph row container (contains labels + graph)
    pub graph_row: Option<bevy::ecs::entity::Entity>,
    /// Entity for the actual graph rendering area
    pub graph_entity: Option<bevy::ecs::entity::Entity>,
    /// Material handle for the graph shader
    pub graph_material: Option<bevy::asset::Handle<crate::MultiLineGraphMaterial>>,
    /// Handles to all graph label entities
    pub graph_labels: Vec<crate::graph_components::GraphLabelHandle>,
    /// Width allocated for graph labels in pixels
    pub graph_label_width: f32,
    /// Entity for the bars container
    pub bars_root: Option<bevy::ecs::entity::Entity>,
    /// Material handles for bar shaders
    pub bar_materials: Vec<bevy::asset::Handle<crate::BarMaterial>>,
    /// Entities for bar label text
    pub bar_labels: Vec<bevy::ecs::entity::Entity>,
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