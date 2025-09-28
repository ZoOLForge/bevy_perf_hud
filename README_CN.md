# bevy_perf_hud

[![CI](https://github.com/ZoOLForge/bevy_perf_hud/workflows/CI/badge.svg)](https://github.com/ZoOLForge/bevy_perf_hud/actions)
[![Crates.io](https://img.shields.io/crates/v/bevy_perf_hud)](https://crates.io/crates/bevy_perf_hud)
[![Downloads](https://img.shields.io/crates/d/bevy_perf_hud)](https://crates.io/crates/bevy_perf_hud)
[![Documentation](https://docs.rs/bevy_perf_hud/badge.svg)](https://docs.rs/bevy_perf_hud)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/ZoOLForge/bevy_perf_hud#license)
[![Discord](https://img.shields.io/discord/1319490473060073532?label=Discord&logo=discord&logoColor=white)](https://discord.gg/jwyXfjUP)

![Sep-24-2025 18-37-55](https://github.com/ZoOLForge/bevy_perf_hud/raw/main/media/Sep-24-2025%2018-37-55.gif)

![条形图缩放模式演示](https://github.com/ZoOLForge/bevy_perf_hud/raw/main/media/Sep-25-2025%2019-26-06.gif)

一个可配置的性能抬头显示器（HUD）插件，专为 Bevy 应用打造。在运行时可视化帧率、实体数量和资源使用情况，并可扩展自定义指标。

## 目录

- [特性](#特性)
- [安装](#安装)
- [快速上手](#快速上手)
- [高级配置](#高级配置)
- [内置指标](#内置指标)
- [自定义指标](#自定义指标)
- [示例](#示例)
- [性能影响](#性能影响)
- [故障排除](#故障排除)
- [获取帮助](#获取帮助)
- [支持的版本](#支持版本)
- [许可证](#许可证)
- [致谢](#致谢)

## 特性

- 灵活的 HUD 布局，支持多曲线图表和资源条。
- 内建 FPS、帧时间、实体数量、系统/进程 CPU 与内存使用情况的指标提供器。
- 精细的控制选项，包括平滑处理、量化、自动缩放和外观。
- 可扩展的 `PerfMetricProvider` 特性，用于自定义指标并显示在内置指标旁边。

## 安装

**最小支持 Rust 版本 (MSRV)**: 1.76.0

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
bevy = { version = "0.16", default-features = false, features = [
    "bevy_winit",
    "bevy_ui",
    "bevy_render",
    "bevy_diagnostic",
    "sysinfo_plugin",
] }
bevy_perf_hud = "0.1"
```

### 功能标志

| 功能      | 描述                 | 默认 |
|-----------|----------------------|------|
| `default` | 启用所有标准功能     | ✓    |

### 要求

- **Bevy 功能**: HUD 需要 `bevy_ui`、`bevy_diagnostic` 和 `bevy_render` 功能
- **系统指标**: 添加 `sysinfo_plugin` 功能以进行 CPU/内存监控
- **平台支持**: Windows、macOS、Linux（系统指标在某些平台上可能功能有限）

> **提示**: 如果你使用 `DefaultPlugins`，所需功能已经启用。没有 `sysinfo_plugin`，
> 系统/进程 CPU 和内存提供器将被静默跳过。

## 快速上手

```rust
use bevy::prelude::*;
use bevy_perf_hud::BevyPerfHudPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
```

默认情况下，HUD 会显示在右上角附近。要重新定位或自定义布局，请在添加插件之前插入 `PerfHudSettings` 资源：

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

## 高级配置

`PerfHudSettings` 提供了额外的调节选项来定制 HUD：

- `graph`: 调整画布大小、曲线平滑度、量化以及决定哪些指标出现在时间序列图表中。
- `bars`: 控制资源条是否渲染，设置每项指标的最小/最大边界，并决定何时显示数值。
- `enabled` / `origin`: 全局切换 HUD，并在屏幕上任意锚定。

示例：扩展图表，平滑 FPS 曲线，缩小系统 CPU 条范围。

```rust
use bevy::prelude::*;
use bevy_perf_hud::{BevyPerfHudPlugin, PerfHudSettings};

fn main() {
    App::new()
        .insert_resource({
            let mut settings = PerfHudSettings::default();
            settings.origin = Vec2::new(24.0, 24.0);
            settings.graph.size = Vec2::new(360.0, 120.0);
            if let Some(fps_curve) = settings
                .graph
                .curves
                .iter_mut()
                .find(|curve| curve.metric.id == "fps")
            {
                fps_curve.smoothing = Some(0.15);
            }
            // 修改实体数量条使用自动缩放
            if let Some(entity_bar) = settings
                .bars
                .bars
                .iter_mut()
                .find(|bar| bar.metric.id == "entity_count")
            {
                entity_bar.scale_mode = bevy_perf_hud::BarScaleMode::Auto {
                    smoothing: 0.8,     // 平滑范围过渡
                    min_span: 100.0,    // 最小范围100个实体
                    margin_frac: 0.2,   // 20%增长余量
                };
                entity_bar.show_value = Some(true);
            }
            settings
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(BevyPerfHudPlugin)
        .run();
}
```

### 性能条缩放模式

性能条可以使用不同的缩放模式来动态调整其范围：

#### 固定模式（默认）
使用静态 `min_value` 和 `max_value` - 传统行为，具有可预测的稳定范围。

```rust
use bevy_perf_hud::{BarConfig, BarScaleMode, MetricDefinition};

BarConfig {
    metric: MetricDefinition { /* ... */ },
    min_value: 0.0,
    max_value: 100.0,
    scale_mode: BarScaleMode::Fixed, // 直接使用 min_value/max_value
    // ...
}
```

#### 自动模式
基于历史数据自动调整范围，具有平滑过渡：

```rust
BarConfig {
    metric: MetricDefinition { /* ... */ },
    min_value: 0.0,   // 无数据时的后备值
    max_value: 100.0, // 无数据时的后备值
    scale_mode: BarScaleMode::Auto {
        smoothing: 0.8,     // 范围变化平滑度（0.0=瞬间，1.0=从不变化）
        min_span: 10.0,     // 最小范围跨度，防止除零
        margin_frac: 0.15,  // 数据上下边距分数（0.0-0.5）
    },
    min_limit: Some(0.0),    // 硬性最小边界（可选）
    max_limit: Some(500.0),  // 硬性最大边界（可选）
    // ...
}
```

#### 百分位模式
使用最近样本的百分位数 - 适合有尖峰的数据，如延迟：

```rust
BarConfig {
    metric: MetricDefinition { /* ... */ },
    min_value: 0.0,   // 样本不足时的后备值
    max_value: 100.0, // 样本不足时的后备值
    scale_mode: BarScaleMode::Percentile {
        lower: 5.0,        // P5百分位数作为最小值（忽略底部5%）
        upper: 95.0,       // P95百分位数作为最大值（忽略顶部5%）
        sample_count: 60,  // 要分析的最近样本数量
    },
    min_limit: Some(0.0),    // 硬边界防止极端异常值
    max_limit: Some(1000.0),
    // ...
}
```

**使用场景：**
- **固定**：CPU/内存百分比、已知限制的FPS
- **自动**：实体数量、MB单位的内存使用等变化指标
- **百分位**：网络延迟、帧尖峰、任何有异常值的指标

## 内置指标

| 指标 ID           | 说明                                            |
|-------------------|-------------------------------------------------|
| `fps`             | 每秒帧数（向下取整为整数）。                     |
| `frame_time_ms`   | 平滑后的帧时间（毫秒）。                         |
| `entity_count`    | `World` 中的活跃实体数量。                       |
| `system/cpu_usage`| 整体系统 CPU 使用率百分比。                      |
| `system/mem_usage`| 整体系统内存使用率百分比。                       |
| `process/cpu_usage`| 运行进程的 CPU 使用率。                          |
| `process/mem_usage`| 运行进程的内存占用（MiB）。                      |

## 自定义指标

实现 `PerfMetricProvider` 特性并使用 `PerfHudAppExt` 辅助工具注册：

### 基础示例

```rust
use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext};

#[derive(Default)]
struct NetworkLagProvider(f32);

impl PerfMetricProvider for NetworkLagProvider {
    fn metric_id(&self) -> &str { "net/lag_ms" }

    fn sample(&mut self, _ctx: MetricSampleContext) -> Option<f32> {
        // 模拟网络延迟测量
        self.0 = (self.0 + 1.0) % 120.0;
        Some(self.0)
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

### 高级示例

这是一个更现实的示例，用于跟踪多个游戏指标：

```rust
use bevy::prelude::*;
use bevy_perf_hud::{PerfHudAppExt, PerfMetricProvider, MetricSampleContext, PerfHudSettings};
use std::collections::VecDeque;

// 跟踪活跃玩家连接
#[derive(Resource, Default)]
struct GameStats {
    active_players: u32,
    packets_per_second: VecDeque<u32>,
    last_update: f64,
}

#[derive(Default)]
struct PlayerCountProvider;

impl PerfMetricProvider for PlayerCountProvider {
    fn metric_id(&self) -> &str { "game/players" }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        ctx.world.get_resource::<GameStats>()
            .map(|stats| stats.active_players as f32)
    }
}

#[derive(Default)]
struct NetworkThroughputProvider;

impl PerfMetricProvider for NetworkThroughputProvider {
    fn metric_id(&self) -> &str { "net/packets_sec" }

    fn sample(&mut self, ctx: MetricSampleContext) -> Option<f32> {
        ctx.world.get_resource::<GameStats>()
            .and_then(|stats| stats.packets_per_second.back().copied())
            .map(|pps| pps as f32)
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameStats::default())
        .insert_resource({
            let mut settings = PerfHudSettings::default();
            // 将我们的自定义指标添加到 HUD 显示
            settings.graph.curves.push(bevy_perf_hud::GraphCurveSettings {
                metric: bevy_perf_hud::MetricSettings {
                    id: "game/players".to_string(),
                    ..default()
                },
                color: Color::srgb(0.2, 0.8, 0.2),
                ..default()
            });
            settings.bars.bars.push(bevy_perf_hud::BarSettings {
                metric: bevy_perf_hud::MetricSettings {
                    id: "net/packets_sec".to_string(),
                    ..default()
                },
                max_value: 1000.0,
                ..default()
            });
            settings
        })
        .add_plugins(bevy_perf_hud::BevyPerfHudPlugin)
        .add_perf_metric_provider(PlayerCountProvider)
        .add_perf_metric_provider(NetworkThroughputProvider)
        .add_systems(Update, update_game_stats)
        .run();
}

// 更新自定义指标数据的系统
fn update_game_stats(
    mut stats: ResMut<GameStats>,
    time: Res<Time>,
    // 你的实际游戏系统将提供真实数据
) {
    let now = time.elapsed_secs_f64();
    if now - stats.last_update > 1.0 {
        // 模拟一些真实的游戏数据
        stats.active_players = (20.0 + 10.0 * (now * 0.1).sin()) as u32;

        let pps = (500.0 + 200.0 * (now * 0.05).cos()) as u32;
        stats.packets_per_second.push_back(pps);
        if stats.packets_per_second.len() > 60 {
            stats.packets_per_second.pop_front();
        }

        stats.last_update = now;
    }
}
```

### 自定义指标指南

- **唯一 ID**: 使用描述性、层次化的名称，如 `"game/players"` 或 `"net/latency_ms"`
- **性能**: 保持 `sample()` 实现快速 - 它们每帧都会被调用
- **可选值**: 当数据不可用时返回 `None` 而不是占位值
- **单位**: 在指标 ID 中包含单位以保持清晰（`_ms`, `_mb`, `_percent`）

## 示例

仓库提供了几个可运行的示例：

- `examples/simple.rs`: 带有 3D 场景与键盘快捷键（空格键生成方块，F1 切换 HUD 模式）。
- `examples/custom_metric.rs`: 演示注册额外的指标提供器与自动缩放。
- `examples/bar_scaling_modes.rs`: 展示所有三种性能条缩放模式（固定、自动、百分位）的实际应用。

运行方式：

```bash
cargo run --example simple
cargo run --example custom_metric
cargo run --example bar_scaling_modes
```

## 性能影响

性能 HUD 的设计旨在对你的应用程序产生最小影响：

- **CPU 使用率**: 典型应用程序约增加 0.1-0.5% 开销
- **内存使用**: 存储历史数据和 UI 组件约需 2-4MB
- **渲染成本**: UI 渲染通常在帧时间上增加 <0.1ms

**优化建议**:

- 在图设置中减少 `history_samples` 以降低内存使用
- 通过从曲线/条配置中删除来禁用未使用的指标
- 对于采样代价高昂的自定义指标，使用更大的 `update_interval`
- 考虑使用功能标志在发布版本中禁用 HUD

## 故障排除

### 常见问题

**HUD 不出现**:

- 确保在你的 Bevy 依赖中启用了 `bevy_ui` 功能
- 检查你是否使用了 `DefaultPlugins` 或手动添加了所需的 UI 插件
- 验证 HUD 没有定位在窗口边界外

**缺少系统指标（CPU/内存）**:

- 在你的 Bevy 依赖中添加 `sysinfo_plugin` 功能
- 没有此功能，系统/进程指标将被静默跳过

**具有大量实体时性能下降**:

- `entity_count` 指标在具有 100k+ 实体时可能影响性能
- 考虑从 HUD 配置中移除此指标以用于非常大的世界

**自定义指标不更新**:

- 确保你的 `PerfMetricProvider::sample()` 方法返回 `Some(value)`
- 检查提供器是否已通过 `add_perf_metric_provider()` 正确注册
- 验证指标 ID 是唯一的，不与内置指标冲突

### 性能调试

如果 HUD 本身导致性能问题：

```rust
// 要禁用 HUD，只需移除插件或组件
// 不再使用全局 enabled 字段
```

## 获取帮助

- **问题**: 在 [GitHub Issues](https://github.com/ZoOLForge/bevy_perf_hud/issues) 上报告错误或请求功能
- **讨论**: 在 [GitHub Discussions](https://github.com/ZoOLForge/bevy_perf_hud/discussions) 上提问
- **Discord**: 加入我们的 [Discord 服务器](https://discord.gg/jwyXfjUP) 获取实时帮助
- **文档**: 详细 API 文档可在 [docs.rs](https://docs.rs/bevy_perf_hud) 上找到

报告问题时，请包含：

- 你的 Bevy 版本
- 操作系统及版本
- 重现问题的最简代码示例
- 控制台输出或错误消息

## 支持版本

| bevy | bevy_perf_hud |
|------|---------------|
| 0.16 | 0.1.3         |

## 许可证

采用 MIT 许可证或 Apache 许可证 2.0 双重许可。

## 致谢

- [Bevy Engine](https://bevyengine.org/) 提供 ECS/游戏引擎基础。
- `bevy_diagnostic` 和 `SystemInformationDiagnosticsPlugin` 提供驱动 HUD 的指标。

Looking for the English documentation? See [`README.md`](README.md).
