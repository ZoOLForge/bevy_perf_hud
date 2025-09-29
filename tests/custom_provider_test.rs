use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext, MetricDefinition, MetricRegistry};

#[derive(Default, Clone)]
struct TestMetricProvider {
    counter: u32,
}

impl PerfMetricProvider for TestMetricProvider {
    fn metric_id(&self) -> &str { "test/metric" }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        self.counter += 1;
        Some(self.counter as f32)
    }
}

fn setup_test_metric(
    mut commands: Commands,
    mut metric_registry: ResMut<MetricRegistry>,
) {
    let test_metric = MetricDefinition {
        id: "test/metric".into(),
        label: Some("Test Metric".into()),
        unit: Some("#".into()),
        precision: 0,
        color: Color::srgb(1.0, 0.0, 0.0),
    };

    metric_registry.register(test_metric.clone());
    commands.spawn(test_metric);
}

#[test]
fn test_custom_provider_integration() {
    App::new()
        .add_plugins((
            bevy::MinimalPlugins,
            bevy_perf_hud::BevyPerfHudPlugin
        ))
        .add_perf_metric_provider(TestMetricProvider::default())
        .add_systems(Startup, setup_test_metric)
        .update();
    
    // The test passes if no panic occurs
}