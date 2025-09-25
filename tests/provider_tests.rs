//! Unit tests for individual metric providers
//!
//! These tests verify that each metric provider correctly samples
//! and processes performance data.

use bevy::diagnostic::DiagnosticsStore;
use bevy_perf_hud::{
    FpsMetricProvider, FrameTimeMetricProvider, EntityCountMetricProvider,
    MetricSampleContext, PerfMetricProvider,
};

#[test]
fn fps_provider_has_correct_id() {
    let provider = FpsMetricProvider::default();
    assert_eq!(provider.metric_id(), "fps");
}

#[test]
fn frame_time_provider_has_correct_id() {
    let provider = FrameTimeMetricProvider::default();
    assert_eq!(provider.metric_id(), "frame_time_ms");
}

#[test]
fn entity_count_provider_has_correct_id() {
    let provider = EntityCountMetricProvider::default();
    assert_eq!(provider.metric_id(), "entity_count");
}

#[test]
fn providers_handle_missing_diagnostics_gracefully() {
    let mut fps_provider = FpsMetricProvider::default();
    let mut frame_time_provider = FrameTimeMetricProvider::default();
    let mut entity_count_provider = EntityCountMetricProvider::default();

    let ctx = MetricSampleContext { diagnostics: None };

    // Providers should return None when diagnostics are unavailable
    assert_eq!(fps_provider.sample(ctx), None);
    assert_eq!(frame_time_provider.sample(ctx), None);
    assert_eq!(entity_count_provider.sample(ctx), None);
}

#[test]
fn providers_handle_empty_diagnostics_gracefully() {
    let mut fps_provider = FpsMetricProvider::default();
    let mut frame_time_provider = FrameTimeMetricProvider::default();
    let mut entity_count_provider = EntityCountMetricProvider::default();

    let diagnostics = DiagnosticsStore::default();
    let ctx = MetricSampleContext {
        diagnostics: Some(&diagnostics),
    };

    // Providers should return None when specific metrics are unavailable
    assert_eq!(fps_provider.sample(ctx), None);
    assert_eq!(frame_time_provider.sample(ctx), None);
    assert_eq!(entity_count_provider.sample(ctx), None);
}