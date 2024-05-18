@vertex
fn vs_main(@location(0) pos: vec2<f32>) ->  @builtin(position)  vec4<f32> {
    return vec4(pos * vec2(2., -2.) + vec2(-1., 1.), 0., 1.);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(0., 0., 0., 0.);
}