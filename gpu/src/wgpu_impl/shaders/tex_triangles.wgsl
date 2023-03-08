struct Primitive {
  /// the transform for the sampler.
  transform: mat3x2<f32>,
  /// The origin of content box.
  content_origin: vec2<f32>,
  /// The mask position in its texture.
  mask_offset: vec2<f32>,
  /// The origin of the brush placed in texture.
  brush_origin: vec2<f32>,
  /// The size of the brush image.
  brush_size: vec2<f32>,
  /// The high 16-bits is the index of brush texture and the low 16-bits is the mask texture index.
  brush_and_mask_idx: u32,
  /// extra alpha apply to current vertex
  opacity: f32,
};

struct FragInput {
  @builtin(position) pos: vec4<f32>,
  @location(0) mask_idx: u32,
  @location(1) mask_pos: vec2<f32>,
  @location(2) brush_idx: u32,
  @location(3) brush_tex_pos: vec2<f32>,
  @location(4) opacity: f32,
  @location(5) brush_size: vec2<f32>,
  @location(6) brush_origin: vec2<f32>,
}


@group(0) @binding(0)
var<uniform> coord_matrix: mat4x4<f32>;

@group(2) @binding(0)
var<storage> primitives: array<Primitive>;

const mask_idx_mask: u32 = 0x0000FFFFu;

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) prim_id: u32) -> FragInput {
    let prim: Primitive = primitives[prim_id];
    var input: FragInput;
    input.pos = coord_matrix * vec4<f32>(pos, 0.0, 1.0);
    input.opacity = prim.opacity;
    input.mask_idx = prim.brush_and_mask_idx >> 16u;
    input.brush_idx = prim.brush_and_mask_idx & mask_idx_mask;
    input.mask_pos = pos + prim.mask_offset;
    input.brush_tex_pos = prim.transform * vec3(pos - prim.content_origin, 1.0);
    input.brush_origin = prim.brush_origin;
    input.brush_size = prim.brush_size;

    return input;
}

@group(1) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var samplers: binding_array<sampler>;

@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    let texture = textures[input.brush_idx];
    let tex_sampler = samplers[input.brush_idx];
    let tex_size = vec2<f32>(textureDimensions(texture));
    let brush_tex_pos = (input.brush_tex_pos % input.brush_size + input.brush_origin) / tex_size;
    var color = textureSample(texture, tex_sampler, brush_tex_pos);

    let mask = textures[input.mask_idx];
    let mask_tex_size = vec2<f32>(textureDimensions(mask));
    let mask_sampler = samplers[input.mask_idx];
    let mask_pos = input.mask_pos / mask_tex_size;
    let alpha = textureSample(mask, mask_sampler, mask_pos).r;
    color.a = color.a * alpha * input.opacity;
    return color;
}
