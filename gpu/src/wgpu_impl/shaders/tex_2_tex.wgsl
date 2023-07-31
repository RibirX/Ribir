struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) tex_pos: vec2<f32>,
}

@vertex
fn vs_main(@location(0) input_pos: vec2<f32>, @location(1) tex: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    let pos = input_pos * vec2(2., -2.) + vec2(-1., 1.);
    output.pos = vec4<f32>(pos, 0.0, 1.0);
    output.tex_pos = tex;
    return output;
}

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;


@fragment
fn fs_main(@location(0) tex_pos: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(texture, tex_sampler, tex_pos);
}