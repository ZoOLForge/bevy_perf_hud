//! Rendering components and materials for the performance HUD.
//!
//! This module contains all GPU-related code including:
//! - Shader parameter structures
//! - UI material definitions
//! - WGSL shader source code

#![allow(dead_code)] // Struct fields are used by GPU shaders

use bevy::{
    asset::Asset,
    math::Vec4,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
    ui::UiMaterial,
};

use crate::constants::{MAX_CURVES, SAMPLES_VEC4};

// ============================================================================
// SHADER PARAMETER STRUCTURES
// ============================================================================

/// Parameters for the multi-line graph shader.
///
/// This structure contains all the data that needs to be passed to the GPU shader
/// for rendering the performance metrics as time-series graphs. It includes values
/// for multiple curves, UI styling, and rendering configuration.
#[derive(Debug, Clone, ShaderType)]
pub struct MultiLineGraphParams {
    /// 2D array storing all graph values [curve_index][vec4_chunk_index]
    /// Each curve's data is packed into Vec4 chunks for efficient GPU access
    pub values: [[Vec4; SAMPLES_VEC4]; MAX_CURVES],
    /// Number of valid data points currently stored in the values array
    pub length: u32,
    /// Minimum Y-axis value for scaling the graph display
    pub min_y: f32,
    /// Maximum Y-axis value for scaling the graph display
    pub max_y: f32,
    /// Line thickness factor for rendering graph curves (0.0-1.0 normalized)
    pub thickness: f32,
    /// Background color for the graph area (RGBA format)
    pub bg_color: Vec4,
    /// Border color for the graph area (RGBA format)
    pub border_color: Vec4,
    /// Thickness of the graph border in pixels
    pub border_thickness: f32,
    /// Border thickness normalized to UV coordinates (X axis)
    pub border_thickness_uv_x: f32,
    /// Border thickness normalized to UV coordinates (Y axis)
    pub border_thickness_uv_y: f32,
    /// Flag indicating whether to draw the left border (0 = no, 1 = yes)
    pub border_left: u32,
    /// Flag indicating whether to draw the bottom border (0 = no, 1 = yes)
    pub border_bottom: u32,
    /// Flag indicating whether to draw the right border (0 = no, 1 = yes)
    pub border_right: u32,
    /// Flag indicating whether to draw the top border (0 = no, 1 = yes)
    pub border_top: u32,
    /// Array of colors for each curve in the graph (RGBA format)
    pub colors: [Vec4; MAX_CURVES],
    /// Number of curves currently active in the graph
    pub curve_count: u32,
}

impl Default for MultiLineGraphParams {
    fn default() -> Self {
        Self {
            values: [[Vec4::ZERO; SAMPLES_VEC4]; MAX_CURVES],
            length: 0,
            min_y: 0.0,
            max_y: 1.0,
            thickness: 0.01,
            bg_color: Vec4::ZERO,
            border_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_thickness: 2.0,
            border_thickness_uv_x: 0.003,
            border_thickness_uv_y: 0.003,
            border_left: 1,
            border_bottom: 1,
            border_right: 0,
            border_top: 0,
            colors: [Vec4::ZERO; MAX_CURVES],
            curve_count: 0,
        }
    }
}

/// Material definition for rendering multi-line graphs in the performance HUD.
///
/// This material wraps the shader parameters and implements the Bevy UI material
/// interface, allowing it to be used as a UI node with custom rendering behavior.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MultiLineGraphMaterial {
    /// Shader parameters containing all data for graph rendering
    #[uniform(0)]
    pub params: MultiLineGraphParams,
}

impl UiMaterial for MultiLineGraphMaterial {
    /// Returns the fragment shader path for multi-line graph rendering.
    ///
    /// This shader handles the rendering of multiple curves on a single graph,
    /// including value scaling, color application, and border rendering.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/multiline_graph.wgsl".into())
    }
}

/// Parameters for the bar chart shader.
///
/// This structure contains data needed to render a single performance metric
/// as a horizontal progress bar, including value, foreground color, and background color.
#[derive(Debug, Clone, ShaderType)]
pub struct BarParams {
    /// Current normalized value for the bar (0.0-1.0 range)
    pub value: f32,
    /// Red component of the bar's foreground color
    pub r: f32,
    /// Green component of the bar's foreground color
    pub g: f32,
    /// Blue component of the bar's foreground color
    pub b: f32,
    /// Alpha component of the bar's foreground color
    pub a: f32,
    /// Red component of the bar's background color
    pub bg_r: f32,
    /// Green component of the bar's background color
    pub bg_g: f32,
    /// Blue component of the bar's background color
    pub bg_b: f32,
    /// Alpha component of the bar's background color
    pub bg_a: f32,
}

/// Material definition for rendering performance bars in the HUD.
///
/// This material handles the rendering of horizontal progress bars that display
/// current metric values as a percentage of their range.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct BarMaterial {
    /// Shader parameters containing all data for bar rendering
    #[uniform(0)]
    pub params: BarParams,
}

impl UiMaterial for BarMaterial {
    /// Returns the fragment shader path for bar rendering.
    ///
    /// This shader renders a horizontal progress bar with configurable
    /// foreground and background colors based on the current value.
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/bar.wgsl".into())
    }
}
