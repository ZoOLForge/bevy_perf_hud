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

#[derive(Debug, Clone, ShaderType)]
pub struct MultiLineGraphParams {
    pub values: [[Vec4; SAMPLES_VEC4]; MAX_CURVES],
    pub length: u32,
    pub min_y: f32,
    pub max_y: f32,
    pub thickness: f32,
    pub bg_color: Vec4,
    pub border_color: Vec4,
    pub border_thickness: f32,
    pub border_thickness_uv_x: f32,
    pub border_thickness_uv_y: f32,
    pub border_left: u32,
    pub border_bottom: u32,
    pub border_right: u32,
    pub border_top: u32,
    pub colors: [Vec4; MAX_CURVES],
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

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct MultiLineGraphMaterial {
    #[uniform(0)]
    pub params: MultiLineGraphParams,
}

impl UiMaterial for MultiLineGraphMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/multiline_graph.wgsl".into())
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct BarParams {
    pub value: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub bg_r: f32,
    pub bg_g: f32,
    pub bg_b: f32,
    pub bg_a: f32,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct BarMaterial {
    #[uniform(0)]
    pub params: BarParams,
}

impl UiMaterial for BarMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/bar.wgsl".into())
    }
}
