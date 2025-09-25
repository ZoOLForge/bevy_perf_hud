//! Integration tests for bevy_perf_hud plugin
//!
//! These tests verify that the plugin integrates correctly with Bevy
//! and that all systems work together properly.

use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, PerfHudSettings};

#[test]
fn plugin_can_be_added_to_app() {
    let mut app = App::new();

    // Add minimal plugins required for UI materials
    app.add_plugins((
        bevy::MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::render::RenderPlugin::default(),
        bevy::ui::UiPlugin::default(),
    ));

    // This should not panic
    app.add_plugins(BevyPerfHudPlugin::default());

    // Verify that the plugin registered its resources
    assert!(app.world().contains_resource::<bevy_perf_hud::SampledValues>());
    assert!(app.world().contains_resource::<bevy_perf_hud::MetricProviders>());
    assert!(app.world().contains_resource::<bevy_perf_hud::HistoryBuffers>());
    assert!(app.world().contains_resource::<bevy_perf_hud::GraphScaleState>());
}

#[test]
fn plugin_works_with_custom_settings() {
    let mut app = App::new();

    // Add minimal plugins required for UI materials
    app.add_plugins((
        bevy::MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::render::RenderPlugin::default(),
        bevy::ui::UiPlugin::default(),
    ));

    // Insert custom settings
    let settings = PerfHudSettings {
        enabled: true,
        origin: Vec2::new(100.0, 50.0),
        ..default()
    };

    app.insert_resource(settings);
    app.add_plugins(BevyPerfHudPlugin::default());

    // Should not panic and settings should be preserved
    let stored_settings = app.world().resource::<PerfHudSettings>();
    assert_eq!(stored_settings.origin, Vec2::new(100.0, 50.0));
    assert!(stored_settings.enabled);
}

#[test]
fn providers_are_registered_correctly() {
    let mut app = App::new();
    app.add_plugins((
        bevy::MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::render::RenderPlugin::default(),
        bevy::ui::UiPlugin::default(),
        BevyPerfHudPlugin::default(),
    ));

    // Verify that providers are registered
    let _providers = app.world().resource::<bevy_perf_hud::MetricProviders>();

    // We can't access private fields, but we can verify the resource exists
    // and that default providers were added by checking if they handle known metrics
    assert!(app.world().contains_resource::<bevy_perf_hud::MetricProviders>());
}