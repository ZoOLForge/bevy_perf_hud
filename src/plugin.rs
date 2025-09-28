//! Core plugin implementation for the bevy_perf_hud.
//!
//! This module contains the main [`BevyPerfHudPlugin`] and its setup logic.

use bevy::{
    app::{App, Plugin, Startup, Update},
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::IntoScheduleConfigs,
    ui::UiMaterialPlugin,
};

use crate::{
    sample_diagnostics, setup_hud, update_graph_and_bars, BarMaterial,
    MetricProviders, MultiLineGraphMaterial,
};

/// Main plugin for the Bevy Performance HUD.
///
/// This plugin sets up all the necessary resources, systems, and materials
/// for rendering a real-time performance monitoring overlay in Bevy applications.
///
/// # Example
///
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_perf_hud::BevyPerfHudPlugin;
///
/// let mut app = App::new();
/// app.add_plugins(DefaultPlugins);
/// app.add_plugins(BevyPerfHudPlugin::default());
/// app.run();
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
            // Initialize metric providers resource (this is still needed as global config)
            .init_resource::<MetricProviders>() // Registry of metric sources
            // Register systems for HUD lifecycle
            .add_systems(Startup, setup_hud) // Create HUD entities on startup
            .add_systems(
                Update,
                (
                    (sample_diagnostics, update_graph_and_bars).chain(),
                ),
            ); // Update loop

        // Register default metric providers (FPS, frame time, entity count, system info)
        app.world_mut()
            .resource_mut::<MetricProviders>()
            .ensure_default_entries();
    }
}
