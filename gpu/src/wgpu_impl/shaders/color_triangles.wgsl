struct Vertex {
  @location(0) pos: vec2<f32>,
  @location(1) color: vec4<f32>,
  @location(2) mask_head: i32
};

struct FragInput {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) mask_head: i32,
}
 

@vertex
fn vs_main(v: Vertex) -> FragInput {
    var input: FragInput;
    // convert from gpu-backend coords(0..1) to wgpu corrds(-1..1)
    let pos = v.pos * vec2(2., -2.) + vec2(-1., 1.);
    input.pos = vec4<f32>(pos, 0.0, 1.0);
    input.mask_head = v.mask_head;
    input.color = v.color;
    return input;
}


struct MaskLayer {
  transform: mat3x2<f32>,
  min: vec2<f32>,
  max: vec2<f32>,
  mask_tex_idx: u32,
  prev_mask_idx: i32,
}

@group(0) @binding(0) 
var<storage> mask_layers: array<MaskLayer>;

@group(1) @binding(0)
var textures: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var samplers: binding_array<sampler>;


@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    var color = input.color;
    var mask_idx = input.mask_head;
    loop {
        if mask_idx < 0 {
            break;
        }
        let mask = mask_layers[u32(mask_idx)];

        var mask_pos = mask.transform * vec3(input.pos.xy, 1.);
        if any(mask_pos < mask.min) || any(mask.max < mask_pos) {
            color.a = 0.;
            break;
        }

        let mask_tex_idx = mask.mask_tex_idx;
        let texture = textures[mask_tex_idx];
        let s_sampler = samplers[mask_tex_idx];

        let tex_size = textureDimensions(texture);
        mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
        let alpha = textureSampleLevel(texture, s_sampler, mask_pos, 0.).r;
        if alpha == 0. {
            color.a = 0.;
            break;
        } else {
            color = color * vec4<f32>(1., 1., 1., alpha);
        }
        mask_idx = mask.prev_mask_idx;
    }
    return color;
}
