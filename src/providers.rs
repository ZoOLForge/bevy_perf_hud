//! Metric provider system for extensible performance monitoring.
//!
//! This module contains the trait-based system that allows the HUD to display
//! both built-in and custom performance metrics.

use bevy::{
    app::App,
    diagnostic::{
        DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::Resource,
};

use crate::constants::*;

/// Context passed to metric providers during sampling.
///
/// Contains references to Bevy's diagnostic systems and other resources
/// that providers might need to calculate their metric values.
#[derive(Clone, Copy)]
pub struct MetricSampleContext<'a> {
    /// Reference to Bevy's diagnostics store for built-in metrics
    pub diagnostics: Option<&'a DiagnosticsStore>,
}

/// Trait for implementing custom performance metric providers.
///
/// This trait allows you to create custom metrics that can be displayed
/// in the performance HUD alongside built-in metrics like FPS and frame time.
///
/// # Example
/// ```rust
/// use bevy_perf_hud::{PerfMetricProvider, MetricSampleContext};
///
/// struct CustomMetricProvider {
///     counter: f32,
/// }
///
/// impl PerfMetricProvider for CustomMetricProvider {
///     fn metric_id(&self) -> &str {
///         "custom_metric"
///     }
///
///     fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
///         self.counter += 1.0;
///         Some(self.counter)
///     }
/// }
/// ```
pub trait PerfMetricProvider: Send + Sync + 'static {
    /// Returns the unique identifier for this metric.
    /// Must match the ID used in metric definitions.
    fn metric_id(&self) -> &str;

    /// Sample the current value of this metric.
    ///
    /// # Arguments
    /// * `ctx` - Context containing diagnostic data and other resources
    ///
    /// # Returns
    /// The current metric value, or None if unavailable
    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32>;
}

/// Resource managing the registry of all metric providers.
///
/// This resource maintains a collection of all metric providers (both built-in
/// and custom) and handles the sampling process during each frame update.
#[derive(Resource, Default)]
pub struct MetricProviders {
    /// Collection of all registered metric providers
    providers: Vec<Box<dyn PerfMetricProvider>>,
}

impl MetricProviders {
    /// Register a new metric provider.
    ///
    /// # Arguments
    /// * `provider` - The provider implementation to register
    pub fn add_provider<P: PerfMetricProvider>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }

    /// Check if a provider with the given metric ID is already registered.
    ///
    /// # Arguments
    /// * `id` - The metric ID to check for
    ///
    /// # Returns
    /// true if a provider for this metric exists, false otherwise
    pub fn contains(&self, id: &str) -> bool {
        self.providers.iter().any(|p| p.metric_id() == id)
    }

    /// Register all built-in metric providers if they haven't been added yet.
    ///
    /// This is called automatically by the plugin to ensure standard metrics
    /// (FPS, frame time, entity count, system resources) are available.
    pub fn ensure_default_entries(&mut self) {
        self.ensure_provider(FpsMetricProvider);
        self.ensure_provider(FrameTimeMetricProvider);
        self.ensure_provider(EntityCountMetricProvider);
        self.ensure_provider(SystemCpuUsageMetricProvider);
        self.ensure_provider(SystemMemUsageMetricProvider);
        self.ensure_provider(ProcessCpuUsageMetricProvider);
        self.ensure_provider(ProcessMemUsageMetricProvider);
    }

    /// Get a mutable iterator over all registered providers.
    ///
    /// Used internally by the sampling system to update metric values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn PerfMetricProvider> {
        self.providers.iter_mut().map(|p| p.as_mut())
    }

    fn ensure_provider<P: PerfMetricProvider>(&mut self, provider: P) {
        let id = provider.metric_id().to_owned();
        if !self.contains(&id) {
            self.providers.push(Box::new(provider));
        }
    }
}

/// Extension trait for [`App`] to easily register custom metric providers.
///
/// This trait provides a convenient way to add custom metric providers
/// to your Bevy application without needing to manually access resources.
///
/// # Example
/// ```no_run
/// use bevy::prelude::*;
/// use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext};
///
/// #[derive(Default)]
/// struct MyCustomProvider;
///
/// impl PerfMetricProvider for MyCustomProvider {
///     fn metric_id(&self) -> &str {
///         "custom/example_metric"
///     }
///
///     fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
///         Some(42.0)
///     }
/// }
///
/// let mut app = App::new();
/// app.add_plugins(DefaultPlugins);
/// app.add_perf_metric_provider(MyCustomProvider::default());
/// app.run();
/// ```
pub trait PerfHudAppExt {
    /// Add a custom metric provider to the application.
    ///
    /// # Arguments
    /// * `provider` - The metric provider to register
    ///
    /// # Returns
    /// The app instance for method chaining
    fn add_perf_metric_provider<P: PerfMetricProvider>(&mut self, provider: P) -> &mut Self;
}

impl PerfHudAppExt for App {
    fn add_perf_metric_provider<P: PerfMetricProvider>(&mut self, provider: P) -> &mut Self {
        self.init_resource::<MetricProviders>();
        self.world_mut()
            .resource_mut::<MetricProviders>()
            .add_provider(provider);
        self
    }
}

/// Built-in metric provider for frames per second (FPS).
///
/// Provides the current FPS value calculated by Bevy's frame time diagnostics.
/// The value is floored to the nearest integer for display purposes.
#[derive(Default)]
pub struct FpsMetricProvider;

impl PerfMetricProvider for FpsMetricProvider {
    fn metric_id(&self) -> &str {
        "fps"
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)?
            .average()?;
        Some(fps as f32)
    }
}

/// Built-in metric provider for frame time in milliseconds.
///
/// Provides the smoothed frame time duration from Bevy's diagnostics,
/// converted to milliseconds and floored to the nearest integer.
#[derive(Default)]
pub struct FrameTimeMetricProvider;

impl PerfMetricProvider for FrameTimeMetricProvider {
    fn metric_id(&self) -> &str {
        "frame_time_ms"
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let frame_time = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)?
            .smoothed()?;
        Some(frame_time as f32)
    }
}

/// Built-in metric provider for the total number of entities.
///
/// Provides the current entity count from Bevy's entity diagnostics.
/// Useful for monitoring memory usage and performance impact of entities.
#[derive(Default)]
pub struct EntityCountMetricProvider;

impl PerfMetricProvider for EntityCountMetricProvider {
    fn metric_id(&self) -> &str {
        "entity_count"
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let entities = diagnostics
            .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)?
            .value()?;
        Some(entities as f32)
    }
}

/// Built-in metric provider for system-wide CPU usage percentage.
///
/// Provides the overall CPU usage across all cores and processes,
/// as reported by Bevy's system information diagnostics.
#[derive(Default)]
pub struct SystemCpuUsageMetricProvider;

impl PerfMetricProvider for SystemCpuUsageMetricProvider {
    fn metric_id(&self) -> &str {
        SYSTEM_CPU_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}

/// Built-in metric provider for system-wide memory usage percentage.
///
/// Provides the overall memory usage as a percentage of total system RAM,
/// as reported by Bevy's system information diagnostics.
#[derive(Default)]
pub struct SystemMemUsageMetricProvider;

impl PerfMetricProvider for SystemMemUsageMetricProvider {
    fn metric_id(&self) -> &str {
        SYSTEM_MEM_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}

/// Built-in metric provider for process-specific CPU usage percentage.
///
/// Provides the CPU usage of the current Bevy application process,
/// as reported by Bevy's system information diagnostics.
#[derive(Default)]
pub struct ProcessCpuUsageMetricProvider;

impl PerfMetricProvider for ProcessCpuUsageMetricProvider {
    fn metric_id(&self) -> &str {
        PROCESS_CPU_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}

/// Built-in metric provider for process-specific memory usage in bytes.
///
/// Provides the memory usage of the current Bevy application process,
/// as reported by Bevy's system information diagnostics.
#[derive(Default)]
pub struct ProcessMemUsageMetricProvider;

impl PerfMetricProvider for ProcessMemUsageMetricProvider {
    fn metric_id(&self) -> &str {
        PROCESS_MEM_USAGE_ID
    }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        let diagnostics = ctx.diagnostics?;
        let usage = diagnostics
            .get(&SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE)?
            .value()?;
        Some(usage as f32)
    }
}
