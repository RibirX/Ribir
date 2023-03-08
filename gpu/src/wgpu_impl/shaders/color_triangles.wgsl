struct Primitive {
  rgba: vec4<f32>,
  mask_offset: vec2<f32>,
  mask_tex_id: u32,
  _dummy: u32,
};

struct FragInput {
  @builtin(position) pos: vec4<f32>,
  @location(0) tex_idx: u32,
  @location(1) mask_pos: vec2<f32>,
  @location(2) color: vec4<f32>,
}
 
@group(0) @binding(0)
var<uniform> coord_matrix: mat4x4<f32>;

@group(2) @binding(0)
var<storage> primitives: array<Primitive>;

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) prim_idx: u32) -> FragInput {
    let prim = primitives[prim_idx];
    var input: FragInput;
    input.pos = coord_matrix * vec4<f32>(pos, 0.0, 1.0);
    input.mask_pos = pos + prim.mask_offset;
    input.tex_idx = prim.mask_tex_id;
    input.color = prim.rgba;

    return input;
}


@group(1) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var samplers: binding_array<sampler>;


@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    let texture = textures[input.tex_idx];
    let s_sampler = samplers[input.tex_idx];
    var color = input.color;
    let tex_size = vec2<f32>(textureDimensions(texture));
    let pos = input.mask_pos / tex_size;
    let mask = textureSample(texture, s_sampler, pos).r;
    color.a = color.a * mask;
    return color;
}
                    