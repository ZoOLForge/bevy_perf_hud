// Optimized bar shader: Simplified logic, reduced redundant clamps, and pre-calculated values
struct BarParams {
  value: f32,
  r: f32, g: f32, b: f32, a: f32,
  bg_r: f32, bg_g: f32, bg_b: f32, bg_a: f32,
}


@group(1) @binding(0)
var<uniform> B: BarParams;


struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, }


@fragment
fn fragment(in: VSOut) -> @location(0) vec4<f32> {
  // Clamp UV coordinates once and use directly
  let uv_x = clamp(in.uv.x, 0.0, 1.0);
  let value = clamp(B.value, 0.0, 1.0);
  
  // Pre-calculate colors to avoid repeated vec4 construction
  let fill = vec4<f32>(B.r, B.g, B.b, B.a);
  let bg = vec4<f32>(B.bg_r, B.bg_g, B.bg_b, B.bg_a);
  
  // Simplified fill detection - avoid select function when possible
  let is_fill = f32(uv_x <= value);
  return mix(bg, fill, is_fill);
}
