//! Core plugin implementation for the bevy_perf_hud.
//!
//! This module contains the main [`BevyPerfHudPlugin`] and its setup logic.

use bevy::{
    app::{App, Plugin, Update},
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::IntoScheduleConfigs,
    ui::UiMaterialPlugin,
};

use crate::{
    initialize_bars_ui, sample_diagnostics, update_bars, update_graph, BarMaterial,
    MetricProviders, MetricRegistry, MultiLineGraphMaterial, ProviderRegistry, PerfMetricProvider,
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
            // Initialize provider registry for display configuration
            .init_resource::<ProviderRegistry>()
            // Initialize metric registry for metric definitions
            .init_resource::<MetricRegistry>()
            // Register systems for HUD lifecycle
            .add_systems(
                Update,
                (
                    // Bar UI initialization runs first to create child entities
                    initialize_bars_ui,
                    // Independent graph and bars systems
                    (sample_diagnostics, update_graph).chain(),
                    (sample_diagnostics, update_bars).chain(),
                ),
            ); // Update loop

        // Register default metric providers (FPS, frame time, entity count, system info)
        app.world_mut()
            .resource_mut::<MetricProviders>()
            .ensure_default_entries();

        // Cache display configurations from default providers
        {
            use crate::providers::{
                FpsMetricProvider, FrameTimeMetricProvider, EntityCountMetricProvider,
                SystemCpuUsageMetricProvider, SystemMemUsageMetricProvider,
                ProcessCpuUsageMetricProvider, ProcessMemUsageMetricProvider,
                ProviderDisplayConfig,
            };

            let world = app.world_mut();
            let mut provider_registry = world.resource_mut::<ProviderRegistry>();

            // Cache display config for each default provider
            let providers: Vec<Box<dyn PerfMetricProvider>> = vec![
                Box::new(FpsMetricProvider),
                Box::new(FrameTimeMetricProvider),
                Box::new(EntityCountMetricProvider),
                Box::new(SystemCpuUsageMetricProvider),
                Box::new(SystemMemUsageMetricProvider),
                Box::new(ProcessCpuUsageMetricProvider),
                Box::new(ProcessMemUsageMetricProvider),
            ];

            for provider in providers {
                let metric_id = provider.metric_id().to_owned();
                let display_config = ProviderDisplayConfig {
                    label: provider.label(),
                    unit: provider.unit(),
                    precision: provider.precision(),
                    color: provider.color(),
                };
                provider_registry.cache_display_config(metric_id, display_config);
            }
        }

        // Register default metric definitions
        app.world_mut()
            .resource_mut::<MetricRegistry>()
            .register_defaults();
    }
}
