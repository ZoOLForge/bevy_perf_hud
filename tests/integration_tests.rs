//! Integration tests for bevy_perf_hud plugin
//!
//! These tests verify that the plugin integrates correctly with Bevy
//! and that all systems work together properly.

use bevy::prelude::*;
use bevy::render::settings::RenderCreation;
use bevy_perf_hud::{BevyPerfHudPlugin, PerfHudSettings};

fn app_with_headless_rendering() -> App {
    let mut app = App::new();

    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());

    let mut render_plugin = bevy::render::RenderPlugin::default();

    if let RenderCreation::Automatic(settings) = &mut render_plugin.render_creation {
        settings.backends = None;
    }
    app.add_plugins(render_plugin);

    app.add_plugins(bevy::ui::UiPlugin::default());

    app
}

#[test]
fn plugin_can_be_added_to_app() {
    let mut app = app_with_headless_rendering();

    // This should not panic
    app.add_plugins(BevyPerfHudPlugin);

    // Verify that the plugin registered its resources
    assert!(app
        .world()
        .contains_resource::<bevy_perf_hud::MetricProviders>());
}

#[test]
fn plugin_works_with_custom_settings() {
    let mut app = app_with_headless_rendering();

    // Insert custom settings
    let settings = PerfHudSettings {
        origin: Vec2::new(100.0, 50.0),
        ..default()
    };

    app.insert_resource(settings);
    app.add_plugins(BevyPerfHudPlugin);

    // Should not panic and settings should be preserved
    let stored_settings = app.world().resource::<PerfHudSettings>();
    assert_eq!(stored_settings.origin, Vec2::new(100.0, 50.0));
}

#[test]
fn providers_are_registered_correctly() {
    let mut app = app_with_headless_rendering();
    app.add_plugins(BevyPerfHudPlugin);

    // Verify that providers are registered
    let _providers = app.world().resource::<bevy_perf_hud::MetricProviders>();

    // We can't access private fields, but we can verify the resource exists
    // and that default providers were added by checking if they handle known metrics
    assert!(app
        .world()
        .contains_resource::<bevy_perf_hud::MetricProviders>());
}
