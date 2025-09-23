// fixed graph: length=256, up to 6 curves
// Note: pack 4 samples into one vec4 to satisfy uniform array stride alignment (16 bytes)
const SAMPLES: u32 = 256u;
const PACK: u32 = 4u;
const SAMPLES_V4: u32 = SAMPLES / PACK; // 64

struct MultiLineGraphParams {
  values: array<array<vec4<f32>, SAMPLES_V4>, 6u>,
  length: u32,
  min_y: f32,
  max_y: f32,
  thickness: f32,
  bg_color: vec4<f32>,
  border_color: vec4<f32>,
  border_thickness: f32,
  border_thickness_uv_x: f32,
  border_thickness_uv_y: f32,
  border_left: u32,
  border_bottom: u32,
  border_right: u32,
  border_top: u32,
  colors: array<vec4<f32>, 6u>,
  curve_count: u32,
}


@group(1) @binding(0)
var<uniform> P: MultiLineGraphParams;


struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, }


fn lane(v: vec4<f32>, i: u32) -> f32 {
  if (i == 0u) { return v.x; }
  if (i == 1u) { return v.y; }
  if (i == 2u) { return v.z; }
  return v.w;
}

@fragment
fn fragment(in: VSOut) -> @location(0) vec4<f32> {
  // Flip Y so that y=0 is bottom, y=1 is top (UI uv is usually top-left origin)
  let uv0 = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
  let uv = vec2<f32>(uv0.x, 1.0 - uv0.y);
  let len = max(P.length, 1u);
  let x = uv.x * f32(len - 1u);
  let i0 = u32(floor(x));
  let i1 = min(i0 + 1u, len - 1u);
  let t = fract(x);


  var best_alpha = 0.0;
  var out_rgb = vec3<f32>(0.0);
  for (var c: u32 = 0u; c < P.curve_count; c = c + 1u) {
    let j0 = i0 / PACK;
    let l0 = i0 % PACK;
    let j1 = i1 / PACK;
    let l1 = i1 % PACK;
    let v0 = P.values[c][j0];
    let v1 = P.values[c][j1];
    let y0 = lane(v0, l0);
    let y1 = lane(v1, l1);
    let y = mix(y0, y1, t);
    let y0n = (y0 - P.min_y) / max(P.max_y - P.min_y, 1e-6);
    let y1n = (y1 - P.min_y) / max(P.max_y - P.min_y, 1e-6);
    let x0 = f32(i0) / max(f32(len - 1u), 1e-6);
    let x1 = f32(i1) / max(f32(len - 1u), 1e-6);
    let p0 = vec2<f32>(x0, y0n);
    let p1 = vec2<f32>(x1, y1n);
    let seg = p1 - p0;
    let w = uv - p0;
    let tseg = clamp(dot(w, seg) / max(dot(seg, seg), 1e-6), 0.0, 1.0);
    let closest = p0 + seg * tseg;
    let d = distance(uv, closest);
    let alpha = smoothstep(P.thickness*1.2, P.thickness*0.6, d);
    let ca = alpha * P.colors[c].a;
    if (ca > best_alpha) {
      best_alpha = ca;
      out_rgb = P.colors[c].rgb;
    }
  }
  var comp_rgb = mix(P.bg_color.rgb, out_rgb, best_alpha);
  var comp_a = 1.0 - (1.0 - P.bg_color.a) * (1.0 - best_alpha);

  // Border overlay (left x=0, bottom y=0)
  let btx = P.border_thickness_uv_x;
  let bty = P.border_thickness_uv_y;
  let left_alpha = select(0.0, smoothstep(btx*1.2, btx*0.6, uv.x), P.border_left == 1u);
  let bottom_alpha = select(0.0, smoothstep(bty*1.2, bty*0.6, uv.y), P.border_bottom == 1u);
  let right_alpha = select(0.0, smoothstep(btx*1.2, btx*0.6, 1.0 - uv.x), P.border_right == 1u);
  let top_alpha = select(0.0, smoothstep(bty*1.2, bty*0.6, 1.0 - uv.y), P.border_top == 1u);
  let b_alpha = max(max(left_alpha, bottom_alpha), max(right_alpha, top_alpha)) * P.border_color.a;
  comp_rgb = mix(comp_rgb, P.border_color.rgb, b_alpha);
  comp_a = 1.0 - (1.0 - comp_a) * (1.0 - b_alpha);
  return vec4<f32>(comp_rgb, comp_a);
}
