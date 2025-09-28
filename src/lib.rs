//! Bevy Performance HUD Plugin
//!
//! A comprehensive performance monitoring overlay for Bevy applications that displays:
//! - Real-time performance graphs with configurable metrics
//! - System resource usage bars (CPU, memory)
//! - Custom metric tracking with extensible provider system
//! - Configurable visual appearance and positioning

mod bar_scale;
mod components;
mod config;
mod constants;
pub mod hud_settings_components;
pub use hud_settings_components::{HudOrigin, GraphConfig, BarsConfig, BarConfig, BarScaleMode, MetricDefinition, CurveConfig};
mod plugin;
mod providers;
mod render;
mod systems;

pub use bar_scale::*;
pub use components::*;

pub use constants::*;

pub use plugin::BevyPerfHudPlugin;
pub use providers::*;
pub use render::*;
pub use systems::*;

// Re-export new component types
pub use components::{BarsHandles, GraphHandles};
