@group(0) @binding(0) 
var<uniform> view_size: vec4<u32>;

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @builtin(instance_index) instance: u32) -> @builtin(position) vec4<f32> {
  // An 8x sample provides better quality than a 4x sample in text rendering. 
  // Text rendering often prioritizes horizontal resolution due to LCD subpixel rendering. 
  // High-DPI displays without subpixel rendering already provide sufficient quality, 
  // so I prefer not to differentiate between text and other path rendering. 
  // I'm experimenting with a 3x2 sample pattern, which slightly outperforms the 4x sample in horizontal resolution.

  // I learned about the sample pattern from 
  // https://learn.microsoft.com/en-us/windows/win32/api/d3d11/ne-d3d11-d3d11_standard_multisample_quality_levels 
  // And attempted to generate a 3x2 sample pattern from the 4x and 8x sample patterns, following these rules:
  // - Divide into a 3x2 area, ensuring each area has a sample.
  // - Divide the x-axis and y-axis into 6 parts, ensuring each part has a sample.
  // - Use 1/18 as a unit for the x-axis and 1/12 as a unit for the y-axis.
  //   This makes the sample in the x-axis wider than the y-axis.

  // Preliminary tests show that for small text, the 3x2 sample pattern is better 
  // than the 4x sample pattern and not inferior to the 8x sample pattern. 
  // This is an experiment and hasn't been extensively tested. 
  // If we encounter problems or if it performs worse than the 4x sample pattern,
  //  we can easily revert to the 4x sample pattern.
  var sample_pattern = array(
    // 4x sample pattern
    // vec2(-6.0, 2.0) / 16.0,
    // vec2(-2.0, -6.0) / 16.0,
    // vec2(2.0, 6.0) / 16.0
    // vec2(6.0, -2.0) / 16.0,

    // 8x sample pattern
    // vec2( -7.,  -1.) / 16.,
    // vec2( -5.,  5.) / 16.,
    // vec2( -3.,  -5.) / 16.,
    // vec2( -1.,  3.) / 16.,
    // vec2(1., -3.) / 16.,
    // vec2( 3.,  7.) / 16.,
    // vec2( 5., 1.) / 16.,
    // vec2( 7.,  -7.) / 16.

    // 3x2 sample pattern
    vec2(-8., -1.) / vec2(18., 12.),
    vec2(-5., 5.) / vec2(18., 12.),
    vec2(-2., -3.) / vec2(18., 12.),
    vec2(2., 3.) / vec2(18., 12.),
    vec2(5., 1.) / vec2(18., 12.),
    vec2(8., -5) / vec2(18., 12.)
  );

    let size = vec2(f32(view_size.x), f32(view_size.y));
    var sample_pos = pos + sample_pattern[instance % 6];
    sample_pos = sample_pos * vec2(2., -2.)  / size + vec2(-1., 1.);
    return vec4<f32>(sample_pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
  let value: f32 = 1.0 / 6.0;
  return vec4(value, value, value, value);
}
