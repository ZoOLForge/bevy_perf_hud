//! Bevy Performance HUD Plugin
//!
//! A comprehensive performance monitoring overlay for Bevy applications that displays:
//! - Real-time performance graphs with configurable metrics
//! - System resource usage bars (CPU, memory)
//! - Custom metric tracking with extensible provider system
//! - Configurable visual appearance and positioning


mod bar_components;
mod components;
pub mod constants;
mod graph_components;
mod plugin;
mod providers;
mod render;
mod systems;


pub use bar_components::*;
pub use components::*;
pub use constants::*;
pub use graph_components::*;
pub use plugin::BevyPerfHudPlugin;
pub use providers::*;
pub use render::*;
pub use systems::*;
