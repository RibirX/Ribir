struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) tex_pos: vec2<f32>,
  @location(1) mask_pos: vec2<f32>
}

@vertex
fn vs_main(
    @location(0) pos: vec2<f32>,
    @location(1) tex: vec2<f32>,
    @location(2) mask: vec2<f32>
) -> VertexOutput {
    var output: VertexOutput;
    output.pos = vec4<f32>(pos, 0.0, 1.0);
    output.tex_pos = tex;
    output.mask_pos = mask;
    return output;
}

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;
@group(0) @binding(2)
var mask: texture_2d<f32>;
@group(0) @binding(3)
var mask_smapler: sampler;


@fragment
fn fs_main(
    @location(0) tex_pos: vec2<f32>,
    @location(1) mask_pos: vec2<f32>
) -> @location(0) vec4<f32> {
    let color = textureSample(texture, tex_sampler, tex_pos);
    let alpha = textureSample(mask, mask_smapler, mask_pos).r;
    let mask = vec4<f32>(alpha, alpha, alpha, alpha);
    return color * mask;
}