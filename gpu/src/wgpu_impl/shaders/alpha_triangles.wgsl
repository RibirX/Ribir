@group(0) @binding(0) 
var<uniform> view_size: vec4<u32>;

// An 8x sample provides better quality than a 4x sample in text rendering. 
// https://learn.microsoft.com/en-us/windows/win32/api/d3d11/ne-d3d11-d3d11_standard_multisample_quality_levels 
const sample_pattern: array<vec2f, 8> = array(
  // 4x sample pattern
  // vec2(-6.0, 2.0) / 16.0,
  // vec2(-2.0, -6.0) / 16.0,
  // vec2(2.0, 6.0) / 16.0
  // vec2(6.0, -2.0) / 16.0,

  // 8x sample pattern
  vec2( -7. / 16.,  -1. / 16.),
  vec2( -5. / 16.,  5. / 16.),
  vec2( -3. / 16.,  -5. / 16.),
  vec2( -1. / 16.,  3. / 16.),
  vec2(1. / 16., -3. / 16.),
  vec2( 3. / 16.,  7. / 16.),
  vec2( 5. / 16., 1. / 16.),
  vec2( 7. / 16.,  -7. / 16.)
);
const sample_size: u32 = 8;


@vertex
fn vs_main(@location(0) pos: vec2<f32>, @builtin(instance_index) instance: u32) -> @builtin(position) vec4<f32> {
  
  let size = vec2(f32(view_size.x), f32(view_size.y));
  var sample_pos = pos + sample_pattern[instance % sample_size];
  sample_pos = sample_pos * vec2(2., -2.)  / size + vec2(-1., 1.);
    return vec4<f32>(sample_pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
  let value: f32 = 1.0 / f32(sample_size);
  return vec4(value, value, value, value);
}
