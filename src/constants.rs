//! Constants used throughout the bevy_perf_hud plugin.
//!
//! This module centralizes all compile-time constants, magic numbers,
//! and string identifiers used across the performance HUD system.

/// Maximum number of samples to store in the history buffer for graph rendering
pub const MAX_SAMPLES: usize = 256;

/// Maximum number of curves that can be displayed simultaneously in a graph
pub const MAX_CURVES: usize = 6;

/// Number of Vec4 elements needed to pack all samples for shader
pub const SAMPLES_VEC4: usize = MAX_SAMPLES / 4;

/// Metric ID for system-wide CPU usage percentage
pub const SYSTEM_CPU_USAGE_ID: &str = "system/cpu_usage";

/// Metric ID for system-wide memory usage percentage
pub const SYSTEM_MEM_USAGE_ID: &str = "system/mem_usage";

/// Metric ID for process-specific CPU usage percentage
pub const PROCESS_CPU_USAGE_ID: &str = "process/cpu_usage";

/// Metric ID for process-specific memory usage in bytes
pub const PROCESS_MEM_USAGE_ID: &str = "process/mem_usage";