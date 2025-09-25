//! Bevy Performance HUD Plugin
//!
//! A comprehensive performance monitoring overlay for Bevy applications that displays:
//! - Real-time performance graphs with configurable metrics
//! - System resource usage bars (CPU, memory)
//! - Custom metric tracking with extensible provider system
//! - Configurable visual appearance and positioning


mod constants;
mod config;
mod plugin;
mod providers;
mod render;
mod resources;
mod systems;

pub use constants::*;
pub use config::*;
pub use plugin::BevyPerfHudPlugin;
pub use providers::*;
pub use render::*;
pub use resources::*;
pub use systems::*;