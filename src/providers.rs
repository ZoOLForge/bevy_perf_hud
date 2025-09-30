//! Metric provider system for extensible performance monitoring.
//!
//! This module contains the trait-based system that allows the HUD to display
//! both built-in and custom performance metrics.

use bevy::{
    app::App,
    color::Color,
    diagnostic::{
        DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    prelude::{Resource, Component, Query, Res},
    ecs::world::World,
};
use std::{
    any::TypeId,
    collections::HashMap,
};

use crate::{constants::*, components::SampledValues};

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

    /// Returns the display label for this metric.
    /// If None, the metric_id will be used as the label.
    fn label(&self) -> Option<String> {
        None
    }

    /// Returns the unit string to display after values (e.g., "ms", "fps", "%").
    fn unit(&self) -> Option<String> {
        None
    }

    /// Returns the number of decimal places to display in values.
    fn precision(&self) -> u32 {
        1
    }

    /// Returns the color for this metric's visualization (curve/bar).
    fn color(&self) -> Color {
        Color::srgb(1.0, 1.0, 1.0)
    }
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

/// Generic component wrapper for performance metric providers.
///
/// This component stores a specific provider type directly without boxing,
/// allowing for compile-time type safety and better performance through
/// avoidance of dynamic dispatch.
#[derive(Component)]
pub struct ProviderComponent<P: PerfMetricProvider> {
    /// The actual metric provider instance
    pub provider: P,
    /// Cached metric ID for quick lookups
    pub metric_id: String,
}

impl<P: PerfMetricProvider> ProviderComponent<P> {
    /// Create a new provider component from a provider instance
    pub fn new(provider: P) -> Self {
        let metric_id = provider.metric_id().to_owned();
        Self { provider, metric_id }
    }

    /// Get the metric ID for this provider
    pub fn metric_id(&self) -> &str {
        &self.metric_id
    }

    /// Get a mutable reference to the provider for sampling
    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    /// Get an immutable reference to the provider
    pub fn provider(&self) -> &P {
        &self.provider
    }
}

/// Metadata about a registered provider type.
///
/// Used to track which provider types have been registered and their
/// associated information for the generic sampling system.
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    /// Type ID of the provider for runtime identification
    pub type_id: TypeId,
    /// Example metric ID from this provider type (for debugging)
    pub sample_metric_id: String,
}

/// Display configuration extracted from a provider.
#[derive(Debug, Clone)]
pub struct ProviderDisplayConfig {
    pub label: Option<String>,
    pub unit: Option<String>,
    pub precision: u32,
    pub color: Color,
}

/// Resource managing the registry of provider types and their metadata.
///
/// This resource tracks which provider types have been registered in the
/// generic system, allowing for proper initialization and querying of
/// provider components.
#[derive(Resource, Default)]
pub struct ProviderRegistry {
    /// Map from TypeId to provider metadata
    registered_types: HashMap<TypeId, ProviderMetadata>,
    /// Map from metric ID to TypeId for reverse lookups
    metric_to_type: HashMap<String, TypeId>,
    /// Cached display configuration from providers
    display_configs: HashMap<String, ProviderDisplayConfig>,
}

impl ProviderRegistry {
    /// Register a provider type with the registry
    pub fn register<P: PerfMetricProvider + 'static>(&mut self, sample_metric_id: String) {
        let type_id = TypeId::of::<P>();
        let metadata = ProviderMetadata {
            type_id,
            sample_metric_id: sample_metric_id.clone(),
        };

        self.registered_types.insert(type_id, metadata);
        self.metric_to_type.insert(sample_metric_id, type_id);
    }

    /// Cache display configuration from a provider
    pub fn cache_display_config(&mut self, metric_id: String, config: ProviderDisplayConfig) {
        self.display_configs.insert(metric_id, config);
    }

    /// Get cached display configuration for a metric
    pub fn get_display_config(&self, metric_id: &str) -> Option<&ProviderDisplayConfig> {
        self.display_configs.get(metric_id)
    }

    /// Check if a provider type is registered
    pub fn is_registered<P: PerfMetricProvider + 'static>(&self) -> bool {
        self.registered_types.contains_key(&TypeId::of::<P>())
    }

    /// Get metadata for a provider type
    pub fn get_metadata<P: PerfMetricProvider + 'static>(&self) -> Option<&ProviderMetadata> {
        self.registered_types.get(&TypeId::of::<P>())
    }

    /// Get provider type ID from metric ID
    pub fn get_type_for_metric(&self, metric_id: &str) -> Option<TypeId> {
        self.metric_to_type.get(metric_id).copied()
    }

    /// Get all registered type IDs
    pub fn get_registered_types(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.registered_types.keys().copied()
    }

    /// Clear the registry (useful for testing)
    pub fn clear(&mut self) {
        self.registered_types.clear();
        self.metric_to_type.clear();
    }

    /// Ensure all default provider types are registered and spawned.
    ///
    /// This function spawns entities with ProviderComponent for all built-in
    /// provider types and registers them in the registry. It should be called
    /// by the plugin during initialization.
    pub fn ensure_default_provider_entities(&mut self, world: &mut World) {
        // Helper macro to spawn provider and cache its display config
        macro_rules! register_provider {
            ($provider:expr, $type:ty) => {
                let provider = $provider;
                let metric_id = provider.metric_id().to_owned();

                // Cache display config
                self.cache_display_config(metric_id.clone(), ProviderDisplayConfig {
                    label: provider.label(),
                    unit: provider.unit(),
                    precision: provider.precision(),
                    color: provider.color(),
                });

                // Spawn provider component
                world.spawn(ProviderComponent::new(provider));

                // Register type
                self.register::<$type>(metric_id);
            };
        }

        register_provider!(FpsMetricProvider::default(), FpsMetricProvider);
        register_provider!(FrameTimeMetricProvider::default(), FrameTimeMetricProvider);
        register_provider!(EntityCountMetricProvider::default(), EntityCountMetricProvider);
        register_provider!(SystemCpuUsageMetricProvider::default(), SystemCpuUsageMetricProvider);
        register_provider!(SystemMemUsageMetricProvider::default(), SystemMemUsageMetricProvider);
        register_provider!(ProcessCpuUsageMetricProvider::default(), ProcessCpuUsageMetricProvider);
        register_provider!(ProcessMemUsageMetricProvider::default(), ProcessMemUsageMetricProvider);
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
/// #[derive(Default, Clone)]
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
    fn add_perf_metric_provider<P: PerfMetricProvider + Clone + 'static>(&mut self, provider: P) -> &mut Self;
}

impl PerfHudAppExt for App {
    fn add_perf_metric_provider<P: PerfMetricProvider + Clone + 'static>(&mut self, provider: P) -> &mut Self {
        // Store provider using the new generic component system
        let metric_id = provider.metric_id().to_owned();

        // Extract display configuration from provider
        let display_config = ProviderDisplayConfig {
            label: provider.label(),
            unit: provider.unit(),
            precision: provider.precision(),
            color: provider.color(),
        };

        let provider_component = ProviderComponent::new(provider.clone());

        // Spawn an entity with the provider component
        self.world_mut().spawn(provider_component);

        // Register the provider type and cache display config
        self.init_resource::<ProviderRegistry>();
        let mut registry = self.world_mut().resource_mut::<ProviderRegistry>();
        registry.register::<P>(metric_id.clone());
        registry.cache_display_config(metric_id, display_config);
        drop(registry);

        // Add the sampling system for this provider type
        self.add_systems(
            bevy::app::Update,
            sample_provider_type::<P>
        );

        // Keep backward compatibility with the old system for now
        // Store the provider in the legacy system as well
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
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("FPS:".into())
    }

    fn unit(&self) -> Option<String> {
        Some("fps".into())
    }

    fn precision(&self) -> u32 {
        0
    }

    fn color(&self) -> Color {
        Color::srgb(1.0, 1.0, 1.0)
    }
}

/// Built-in metric provider for frame time in milliseconds.
///
/// Provides the smoothed frame time duration from Bevy's diagnostics,
/// converted to milliseconds and floored to the nearest integer.
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("FT:".into())
    }

    fn unit(&self) -> Option<String> {
        Some("ms".into())
    }

    fn precision(&self) -> u32 {
        1
    }

    fn color(&self) -> Color {
        Color::srgb(0.4, 0.4, 0.4)
    }
}

/// Built-in metric provider for the total number of entities.
///
/// Provides the current entity count from Bevy's entity diagnostics.
/// Useful for monitoring memory usage and performance impact of entities.
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("Entities".into())
    }

    fn precision(&self) -> u32 {
        0
    }

    fn color(&self) -> Color {
        Color::srgb(0.56, 0.93, 0.56)
    }
}

/// Built-in metric provider for system-wide CPU usage percentage.
///
/// Provides the overall CPU usage across all cores and processes,
/// as reported by Bevy's system information diagnostics.
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("SysCPU".into())
    }

    fn unit(&self) -> Option<String> {
        Some("%".into())
    }

    fn precision(&self) -> u32 {
        1
    }

    fn color(&self) -> Color {
        Color::srgb(0.96, 0.76, 0.18)
    }
}

/// Built-in metric provider for system-wide memory usage percentage.
///
/// Provides the overall memory usage as a percentage of total system RAM,
/// as reported by Bevy's system information diagnostics.
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("SysMem".into())
    }

    fn unit(&self) -> Option<String> {
        Some("%".into())
    }

    fn precision(&self) -> u32 {
        1
    }

    fn color(&self) -> Color {
        Color::srgb(0.28, 0.56, 0.89)
    }
}

/// Built-in metric provider for process-specific CPU usage percentage.
///
/// Provides the CPU usage of the current Bevy application process,
/// as reported by Bevy's system information diagnostics.
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("ProcCPU".into())
    }

    fn unit(&self) -> Option<String> {
        Some("%".into())
    }

    fn precision(&self) -> u32 {
        1
    }

    fn color(&self) -> Color {
        Color::srgb(1.0, 0.64, 0.0)
    }
}

/// Built-in metric provider for process-specific memory usage in bytes.
///
/// Provides the memory usage of the current Bevy application process,
/// as reported by Bevy's system information diagnostics.
#[derive(Default, Clone)]
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

    fn label(&self) -> Option<String> {
        Some("ProcMem".into())
    }

    fn unit(&self) -> Option<String> {
        Some("MB".into())
    }

    fn precision(&self) -> u32 {
        0
    }

    fn color(&self) -> Color {
        Color::srgb(0.53, 0.81, 0.92)
    }
}

/// Generic sampling system for a specific provider type.
///
/// This system queries all entities with a specific ProviderComponent<P> type
/// and samples them using the compile-time known provider type, avoiding
/// dynamic dispatch overhead.
pub fn sample_provider_type<P: PerfMetricProvider + 'static>(
    diagnostics: Option<Res<DiagnosticsStore>>,
    mut sampled_values_query: Query<&mut SampledValues>,
    mut provider_query: Query<&mut ProviderComponent<P>>,
) {
    let Ok(mut samples) = sampled_values_query.single_mut() else {
        return;
    };

    let ctx = MetricSampleContext {
        diagnostics: diagnostics.as_deref(),
    };

    // Sample all providers of this specific type
    for mut provider_component in provider_query.iter_mut() {
        if let Some(value) = provider_component.provider_mut().sample(ctx) {
            samples.set(&provider_component.metric_id, value);
        }
    }
}

/// Register all built-in provider sampling systems with the given app.
///
/// This function adds all the built-in provider sampling systems to the Bevy app
/// to enable the new generic sampling approach. Each system handles one specific
/// provider type with compile-time type safety.
pub fn register_builtin_sampling_systems(app: &mut App) {
    app.add_systems(
        bevy::app::Update,
        (
            sample_provider_type::<FpsMetricProvider>,
            sample_provider_type::<FrameTimeMetricProvider>,
            sample_provider_type::<EntityCountMetricProvider>,
            sample_provider_type::<SystemCpuUsageMetricProvider>,
            sample_provider_type::<SystemMemUsageMetricProvider>,
            sample_provider_type::<ProcessCpuUsageMetricProvider>,
            sample_provider_type::<ProcessMemUsageMetricProvider>,
        )
    );
}

/// Helper trait to register a provider type and its sampling system.
///
/// This trait provides a convenient way to register both the provider component
/// and its corresponding sampling system with the Bevy app.
pub trait PerfHudGenericAppExt {
    /// Register a provider type and its sampling system.
    ///
    /// This method should be used instead of add_perf_metric_provider when
    /// you want to use the new generic system exclusively.
    fn register_perf_provider_type<P: PerfMetricProvider + Clone + 'static>(&mut self) -> &mut Self;
}

impl PerfHudGenericAppExt for App {
    fn register_perf_provider_type<P: PerfMetricProvider + Clone + 'static>(&mut self) -> &mut Self {
        // Add the sampling system for this provider type
        self.add_systems(
            bevy::app::Update,
            sample_provider_type::<P>
        );

        // Initialize the provider registry
        self.init_resource::<ProviderRegistry>();

        self
    }
}
