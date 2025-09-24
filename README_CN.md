# bevy_perf_hud

高性能监控 HUD 插件，专为 Bevy 应用打造，可在运行时展示帧率、帧时间、系统与进程资源等指标，并提供可扩展的自定义度量体系。

## 特性 Highlights

- 可配置的性能图表与进度条，支持多曲线、多指标显示。
- 内建 FPS、帧时间、实体数量、系统/进程 CPU 与内存利用率等常用指标。
- 提供平滑、量化与自动缩放设置，便于在不同负载下保持可读性。
- 支持自定义 `PerfMetricProvider` 拓展自有监控数据，并通过插件一键注册。
- 兼容桌面与 Web (wasm32) 目标，可在 Dev / Release / Web-Release 不同 profile 下工作。

## 安装 Installation

在 `Cargo.toml` 中加入依赖：

```toml
[dependencies]
bevy = { version = "0.16", default-features = false, features = ["bevy_winit", "bevy_ui", "bevy_render"] }
bevy_perf_hud = "0.1"
```

> 提示：如果你依赖 `DefaultPlugins`，请确认启用了 `bevy_diagnostic` 与 `bevy_ui` 相关特性，以便 HUD 正常工作。

## 快速上手 Quick Start

```rust
use bevy::prelude::*;
use bevy_perf_hud::BevyPerfHudPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin) // 添加性能 HUD 插件 / Add the performance HUD plugin
        .run();
}
```

默认设置会在窗口右上角绘制图表与资源条。若想修改位置或显示内容，可插入 `PerfHudSettings` 资源：

```rust
use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, PerfHudSettings};

fn main() {
    App::new()
        .insert_resource(PerfHudSettings {
            origin: Vec2::new(32.0, 32.0),
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
```

## 主要指标 Metrics

| 指标 ID               | 说明 Description                                                  |
|---------------------|-----------------------------------------------------------------|
| `fps`               | 当前每秒帧数 (取整) / Frames per second (floored)                       |
| `frame_time_ms`     | 平滑帧时间 (ms) / Smoothed frame time in milliseconds                |
| `entity_count`      | 当前 `World` 中实体数量 / Active entity count                          |
| `system/cpu_usage`  | 系统 CPU 使用率 (%) / Overall system CPU usage (%)                   |
| `system/mem_usage`  | 系统内存使用率 (%) / Overall system memory usage (%)                   |
| `process/cpu_usage` | 当前进程 CPU 使用率 (%) / CPU usage for the running process (%)        |
| `process/mem_usage` | 当前进程内存占用 (MiB) / Memory footprint for the running process (MiB) |

## 自定义指标 Custom Metrics

实现 `PerfMetricProvider` 并通过扩展 trait 注册即可扩展 HUD：

```rust
use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext};

#[derive(Default)]
struct NetworkLagProvider(f32);

impl PerfMetricProvider for NetworkLagProvider {
    fn metric_id(&self) -> &str { "net/lag_ms" }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        self.0 = (self.0 + 1.0) % 120.0;
        Some(self.0) // 返回最新测量值 / Return the latest measurement
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_perf_hud::BevyPerfHudPlugin)
        .add_perf_metric_provider(NetworkLagProvider::default())
        .run();
}
```

## 示例 Examples

仓库提供了两段示例代码：

- `examples/simple.rs`：带有 3D 场景与快捷键 (空格生成几何体、F1 切换 HUD 模式)。
- `examples/custom_metric.rs`：演示如何注册额外的自定义指标。

运行方式：

```bash
cargo run --example simple
cargo run --example custom_metric
```

## 许可证 License

本项目采用双许可协议：MIT 或 Apache-2.0。你可以在 `LICENSE-MIT` 与 `LICENSE-APACHE`（如果存在）中找到完整文本，亦可在
`Cargo.toml` 中查看许可声明。

## 致谢 Acknowledgements

- [Bevy Engine](https://bevyengine.org/) 提供现代化的 ECS 游戏引擎基础。
- HUD 指标基于 `bevy_diagnostic` 以及 `SystemInformationDiagnosticsPlugin` 的数据收集能力。

欢迎 Issue、PR 或讨论，帮助我们持续改进性能可视化体验！
