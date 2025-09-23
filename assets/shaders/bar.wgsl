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
let uv = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
let is_fill = select(0.0, 1.0, uv.x <= clamp(B.value, 0.0, 1.0));
let fill = vec4<f32>(B.r, B.g, B.b, B.a);
let bg = vec4<f32>(B.bg_r, B.bg_g, B.bg_b, B.bg_a);
return mix(bg, fill, is_fill);
}
