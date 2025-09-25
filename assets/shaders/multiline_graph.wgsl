// fixed graph: length=256, up to 6 curves
// Note: pack 4 samples into one vec4 to satisfy uniform array stride alignment (16 bytes)
// Optimized version: Precalculated constants, improved distance calculations, and reduced redundant operations
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


// Optimized smooth_band function with precalculated constants
fn smooth_band(distance: f32, inner: f32, outer: f32, range_reciprocal: f32) -> f32 {
  let normalized = clamp((outer - distance) * range_reciprocal, 0.0, 1.0);
  return smoothstep(0.0, 1.0, normalized);
}

// Smooth cubic Hermite interpolation (ease-in-out curve) between two points
// This creates a smooth curve instead of linear interpolation
fn smooth_interpolate(v0: f32, v1: f32, t: f32) -> f32 {
  // Use smoothstep for smooth interpolation: 3t² - 2t³
  let smooth_t = t * t * (3.0 - 2.0 * t);
  return mix(v0, v1, smooth_t);
}

// Cubic Hermite spline interpolation (requires 4 points: p-1, p0, p1, p2)
// This provides even smoother curves than simple smoothstep interpolation
fn cubic_hermite(p0: f32, p1: f32, m0: f32, m1: f32, t: f32) -> f32 {
  // Calculate t² and t³ for Hermite basis functions
  let t2 = t * t;
  let t3 = t2 * t;
  
  // Hermite basis functions
  let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;     // For p0
  let h10 = t3 - 2.0 * t2 + t;              // For m0
  let h01 = -2.0 * t3 + 3.0 * t2;           // For p1
  let h11 = t3 - t2;                        // For m1
  
  return h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1;
}

@fragment
fn fragment(in: VSOut) -> @location(0) vec4<f32> {
  // Pre-calculate constants and clamp once
  let uv0 = clamp(in.uv, vec2<f32>(0.0), vec2<f32>(1.0));
  let uv = vec2<f32>(uv0.x, 1.0 - uv0.y);
  
  // Pre-calculate length and scale factors
  let len = max(P.length, 1u);
  let len_minus_one = len - 1u;
  let len_scale = max(f32(len_minus_one), 1e-6);
  let inv_len_scale = 1.0 / len_scale;
  let y_range = max(P.max_y - P.min_y, 1e-6);
  let inv_y_range = 1.0 / y_range;

  // Pre-calculate thickness values
  let thickness_inner = P.thickness * 0.6;
  let thickness_outer = P.thickness * 1.2;
  let thickness_range_reciprocal = 1.0 / max(thickness_outer - thickness_inner, 1e-6);

  let x = uv.x * f32(len_minus_one);
  let i0 = u32(floor(x));
  let i1 = min(i0 + 1u, len_minus_one);
  let t = fract(x);

  // Pre-calculate indices
  let j0 = i0 / PACK;
  let l0 = i0 % PACK;
  let j1 = i1 / PACK;
  let l1 = i1 % PACK;

  // Pre-calculate common values outside the loop
  let x0 = f32(i0) * inv_len_scale;
  let x1 = f32(i1) * inv_len_scale;

  var best_alpha = 0.0;
  var out_rgb = vec3<f32>(0.0);
  
  // Loop through curves with early exit
  for (var c: u32 = 0u; c < P.curve_count; c = c + 1u) {
    // Get values for interpolation
    let v0 = P.values[c][j0];
    let v1 = P.values[c][j1];
    let y0 = v0[l0];
    let y1 = v1[l1];
    
    // Get neighboring points for smooth interpolation
    var y_minus1 = y0;  // Previous point
    var y_plus1 = y1;   // Next point
    
    // Get y_minus1: check if i0 > 0
    if (i0 > 0) {
      let j_minus1 = (i0 - 1) / PACK;
      let l_minus1 = (i0 - 1) % PACK;
      let v_minus1 = P.values[c][j_minus1];
      y_minus1 = v_minus1[l_minus1];
    }
    
    // Get y_plus1: check if i1 < len_minus_one
    if (i1 < len_minus_one) {
      let j_plus1 = (i1 + 1) / PACK;
      let l_plus1 = (i1 + 1) % PACK;
      let v_plus1 = P.values[c][j_plus1];
      y_plus1 = v_plus1[l_plus1];
    }
    
    // Use cubic Hermite for smooth interpolation
    // Calculate tangents for Hermite spline
    let m0 = 0.5 * (y1 - y_minus1);  // Tangent at point 0
    let m1 = 0.5 * (y_plus1 - y0);   // Tangent at point 1
    
    // Perform cubic Hermite interpolation
    let y = cubic_hermite(y0, y1, m0, m1, t);
    
    // Normalize y values
    let y0n = (y0 - P.min_y) * inv_y_range;
    let y1n = (y1 - P.min_y) * inv_y_range;
    let yn = (y - P.min_y) * inv_y_range;  // Normalized interpolated y value
    
    // Calculate positions for smooth curve
    let p0 = vec2<f32>(x0, y0n);
    let p1 = vec2<f32>(x1, y1n);
    let p = vec2<f32>(uv.x, yn); // Current interpolated point
    
    // Instead of just measuring distance to the straight line segment,
    // measure distance to the smooth curve using the interpolated point
    // For simplicity, we'll use a weighted approach between the two
    let seg = p1 - p0;
    let w = p - p0;
    let tseg = clamp(dot(w, seg) / max(dot(seg, seg), 1e-6), 0.0, 1.0);
    let closest_on_line = p0 + seg * tseg;  // Closest point on the straight line segment
    
    // Calculate distance to the interpolated point
    let d = distance(uv, p);
    
    // Calculate alpha using precalculated reciprocal
    let alpha = smooth_band(d, thickness_inner, thickness_outer, thickness_range_reciprocal);
    let ca = alpha * P.colors[c].a;
    
    if (ca > best_alpha) {
      best_alpha = ca;
      out_rgb = P.colors[c].rgb;
    }
    
    // Early exit optimization
    if (best_alpha >= 0.995) {
      break;
    }
  }
  
  // Final color blending
  var comp_rgb = mix(P.bg_color.rgb, out_rgb, best_alpha);
  var comp_a = 1.0 - (1.0 - P.bg_color.a) * (1.0 - best_alpha);

  // Pre-calculate border constants
  let btx = P.border_thickness_uv_x;
  let bty = P.border_thickness_uv_y;
  let btx_inner = btx * 0.6;
  let btx_outer = btx * 1.2;
  let bty_inner = bty * 0.6;
  let bty_outer = bty * 1.2;
  let btx_range_reciprocal = 1.0 / max(btx_outer - btx_inner, 1e-6);
  let bty_range_reciprocal = 1.0 / max(bty_outer - bty_inner, 1e-6);

  // Border calculations only if needed
  let left_alpha = select(0.0, smooth_band(uv.x, btx_inner, btx_outer, btx_range_reciprocal), P.border_left == 1u);
  let bottom_alpha = select(0.0, smooth_band(uv.y, bty_inner, bty_outer, bty_range_reciprocal), P.border_bottom == 1u);
  let right_alpha = select(0.0, smooth_band(1.0 - uv.x, btx_inner, btx_outer, btx_range_reciprocal), P.border_right == 1u);
  let top_alpha = select(0.0, smooth_band(1.0 - uv.y, bty_inner, bty_outer, bty_range_reciprocal), P.border_top == 1u);
  
  let b_alpha = max(max(left_alpha, bottom_alpha), max(right_alpha, top_alpha)) * P.border_color.a;
  comp_rgb = mix(comp_rgb, P.border_color.rgb, b_alpha);
  comp_a = 1.0 - (1.0 - comp_a) * (1.0 - b_alpha);
  
  return vec4<f32>(comp_rgb, comp_a);
}
