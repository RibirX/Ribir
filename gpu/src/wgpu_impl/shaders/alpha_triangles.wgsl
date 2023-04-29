struct VertexInput {
  @location(0) pos: vec2<f32>,
  @location(1) value: f32,
}


struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) value: f32,
}


@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let pos = input.pos * vec2(2., -2.) + vec2(-1., 1.);
    var output: VertexOutput;
    output.pos = vec4<f32>(pos, 0.0, 1.0);
    output.value = input.value;
    return output;
}

@fragment
fn fs_main(@location(0) value: f32) -> @location(0) vec4<f32> {
    return vec4(value, value, value, value);
}
