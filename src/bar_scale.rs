//! Bar scaling logic for dynamic range adjustment
//!
//! This module provides automatic range calculation for performance bars,
//! allowing them to adapt their scale based on historical data rather than
//! using fixed min/max values.

use crate::config::BarScaleMode;
use std::collections::VecDeque;

/// State for tracking dynamic bar scaling
#[derive(Debug, Clone)]
pub struct BarScaleState {
    /// Current minimum value for normalization
    pub current_min: f32,
    /// Current maximum value for normalization
    pub current_max: f32,
    /// Historical values for auto/percentile calculation
    pub history: VecDeque<f32>,
    /// Maximum number of samples to keep in history
    pub max_samples: usize,
}

impl Default for BarScaleState {
    fn default() -> Self {
        Self {
            current_min: 0.0,
            current_max: 1.0,
            history: VecDeque::new(),
            max_samples: 120, // ~2 seconds at 60fps
        }
    }
}

impl BarScaleState {
    /// Create a new scale state with specified history size
    pub fn new(max_samples: usize) -> Self {
        Self {
            max_samples,
            ..Default::default()
        }
    }

    /// Add a new sample to the history
    pub fn add_sample(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }

        self.history.push_back(value);

        // Keep only the most recent samples
        while self.history.len() > self.max_samples {
            self.history.pop_front();
        }
    }

    /// Calculate the range based on the configured scale mode
    pub fn calculate_range(
        &mut self,
        mode: &BarScaleMode,
        fallback_min: f32,
        fallback_max: f32,
        min_limit: Option<f32>,
        max_limit: Option<f32>,
    ) -> (f32, f32) {
        let (target_min, target_max) = match mode {
            BarScaleMode::Fixed => (fallback_min, fallback_max),
            BarScaleMode::Auto {
                smoothing,
                min_span,
                margin_frac,
            } => self.calculate_auto_range(
                *smoothing,
                *min_span,
                *margin_frac,
                fallback_min,
                fallback_max,
            ),
            BarScaleMode::Percentile {
                lower,
                upper,
                sample_count,
            } => self.calculate_percentile_range(
                *lower,
                *upper,
                *sample_count,
                fallback_min,
                fallback_max,
            ),
        };

        // Apply hard limits if specified
        let final_min = match min_limit {
            Some(limit) => target_min.max(limit),
            None => target_min,
        };
        let final_max = match max_limit {
            Some(limit) => target_max.min(limit),
            None => target_max,
        };

        // Ensure valid range, but respect hard limits
        let final_max = if final_max < final_min {
            // If limits conflict, adjust final_min down to final_max
            self.current_min = final_max - 1e-6;
            final_max
        } else {
            final_max.max(final_min + 1e-6)
        };
        let final_min = if final_max < final_min {
            final_max - 1e-6
        } else {
            final_min
        };

        self.current_min = final_min;
        self.current_max = final_max;

        (final_min, final_max)
    }

    /// Calculate automatic range based on data statistics
    fn calculate_auto_range(
        &self,
        smoothing: f32,
        min_span: f32,
        margin_frac: f32,
        fallback_min: f32,
        fallback_max: f32,
    ) -> (f32, f32) {
        if self.history.is_empty() {
            return (fallback_min, fallback_max);
        }

        // Calculate data range
        let mut data_min = f32::INFINITY;
        let mut data_max = f32::NEG_INFINITY;

        for &value in &self.history {
            data_min = data_min.min(value);
            data_max = data_max.max(value);
        }

        if !data_min.is_finite() || !data_max.is_finite() {
            return (fallback_min, fallback_max);
        }

        // Ensure minimum span
        let span = (data_max - data_min).max(min_span.max(1e-3));
        if data_max - data_min < span {
            let mid = 0.5 * (data_max + data_min);
            data_min = mid - 0.5 * span;
            data_max = mid + 0.5 * span;
        }

        // Add margins
        let margin = span * margin_frac.clamp(0.0, 0.45);
        let target_min = data_min - margin;
        let target_max = data_max + margin;

        // Apply smoothing
        let smoothing = smoothing.clamp(0.0, 1.0);
        if self.current_max <= self.current_min {
            // First time, use target values directly
            (target_min, target_max)
        } else {
            // Smooth transition from current to target
            let new_min = self.current_min + (target_min - self.current_min) * (1.0 - smoothing);
            let new_max = self.current_max + (target_max - self.current_max) * (1.0 - smoothing);
            (new_min, new_max)
        }
    }

    /// Calculate range based on percentiles of recent data
    fn calculate_percentile_range(
        &self,
        lower_percentile: f32,
        upper_percentile: f32,
        sample_count: usize,
        fallback_min: f32,
        fallback_max: f32,
    ) -> (f32, f32) {
        let samples_to_use = sample_count.min(self.history.len());
        if samples_to_use < 2 {
            return (fallback_min, fallback_max);
        }

        // Get most recent samples
        let mut recent_values: Vec<f32> = self
            .history
            .iter()
            .rev()
            .take(samples_to_use)
            .copied()
            .collect();

        recent_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let lower_idx = ((lower_percentile / 100.0) * (recent_values.len() - 1) as f32) as usize;
        let upper_idx = ((upper_percentile / 100.0) * (recent_values.len() - 1) as f32) as usize;

        let p_min = recent_values[lower_idx.min(recent_values.len() - 1)];
        let p_max = recent_values[upper_idx.min(recent_values.len() - 1)];

        (p_min, p_max.max(p_min + 1e-6))
    }

    /// Get the current normalization range
    pub fn get_current_range(&self) -> (f32, f32) {
        (self.current_min, self.current_max)
    }

    /// Normalize a value using the current range
    pub fn normalize_value(&self, value: f32) -> f32 {
        if self.current_max <= self.current_min {
            return 0.0;
        }

        ((value - self.current_min) / (self.current_max - self.current_min)).clamp(0.0, 1.0)
    }

    /// Clear the history (useful when switching modes)
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.current_min = 0.0;
        self.current_max = 1.0;
    }

    /// Get the number of samples in history
    pub fn sample_count(&self) -> usize {
        self.history.len()
    }

    /// Check if we have enough samples for reliable range calculation
    pub fn has_sufficient_data(&self, min_required: usize) -> bool {
        self.history.len() >= min_required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_mode() {
        let mut state = BarScaleState::default();
        state.add_sample(50.0);

        let (min, max) = state.calculate_range(&BarScaleMode::Fixed, 0.0, 100.0, None, None);

        assert_eq!(min, 0.0);
        assert_eq!(max, 100.0);
    }

    #[test]
    fn test_auto_mode() {
        let mut state = BarScaleState::default();
        for value in [10.0, 20.0, 30.0, 40.0, 50.0] {
            state.add_sample(value);
        }

        let (min, max) = state.calculate_range(
            &BarScaleMode::Auto {
                smoothing: 0.0,
                min_span: 1.0,
                margin_frac: 0.1,
            },
            0.0,
            100.0,
            None,
            None,
        );

        // Should be around data range (10-50) with 10% margins
        assert!(min < 10.0);
        assert!(max > 50.0);
        assert!(min >= 6.0); // 10 - 40*0.1 = 6
        assert!(max <= 54.0); // 50 + 40*0.1 = 54
    }

    #[test]
    fn test_percentile_mode() {
        let mut state = BarScaleState::default();
        for value in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0] {
            state.add_sample(value);
        }

        let (min, max) = state.calculate_range(
            &BarScaleMode::Percentile {
                lower: 10.0, // P10
                upper: 90.0, // P90
                sample_count: 10,
            },
            0.0,
            200.0,
            None,
            None,
        );

        // P10 should be around 1-2, P90 should be around 9-10 (ignoring the outlier 100)
        assert!((1.0..=3.0).contains(&min));
        assert!((8.0..=15.0).contains(&max));
    }

    #[test]
    fn test_limits() {
        let mut state = BarScaleState::default();
        state.add_sample(200.0);

        let (min, max) = state.calculate_range(
            &BarScaleMode::Auto {
                smoothing: 0.0,
                min_span: 1.0,
                margin_frac: 0.1,
            },
            0.0,
            100.0,
            Some(0.0),   // min_limit
            Some(150.0), // max_limit
        );

        assert!(min >= 0.0);
        assert!(max <= 150.0);
    }
}
