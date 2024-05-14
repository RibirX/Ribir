struct Vertex {
  @location(0) pos: vec2<f32>,
  @location(1) color: vec4<f32>,
  @location(2) mask_head: i32
};

struct FragInput {
  @builtin(position) pos: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) @interpolate(flat) mask_head: i32,
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


// Since a the different alignment between WebGPU and WebGL, we not use 
// mat3x2<f32> in the struct, but use vec2<f32> instead. Then, we compose it.
struct MaskLayer {
  t0: vec2<f32>,
  t1: vec2<f32>,
  t2: vec2<f32>,
  min: vec2<f32>,
  max: vec2<f32>,
  @align(4)
  mask_tex_idx: u32,
  @align(4)
  prev_mask_idx: i32,
}

fn mask_matrix(mask: MaskLayer) -> mat3x2<f32> {
  return mat3x2(
    mask.t0, 
    mask.t1, 
    mask.t2
  );
}

@group(0) @binding(0) 
var<uniform> mask_layers: array<MaskLayer, 1365>;

@group(1) @binding(0)
var s_sampler: sampler;
@group(1) @binding(1)
var tex_0: texture_2d<f32>;
@group(1) @binding(2)
var tex_1: texture_2d<f32>;
@group(1) @binding(3)
var tex_2: texture_2d<f32>;
@group(1) @binding(4)
var tex_3: texture_2d<f32>;
@group(1) @binding(5)
var tex_4: texture_2d<f32>;
@group(1) @binding(6)
var tex_5: texture_2d<f32>;
@group(1) @binding(7)
var tex_6: texture_2d<f32>;
@group(1) @binding(8)
var tex_7: texture_2d<f32>;


@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    var color = input.color;
    var mask_idx = input.mask_head;
    loop {
        if mask_idx < 0 {
            break;
        }
        let mask = mask_layers[u32(mask_idx)];

        var mask_pos = mask_matrix(mask) * vec3(input.pos.xy, 1.);
        if any(mask_pos < mask.min) || any(mask.max < mask_pos) {
            color.a = 0.;
            break;
        }

        var alpha = 0.;
        switch mask.mask_tex_idx {
            case 0u: {
                let tex_size = textureDimensions(tex_0);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_0, s_sampler, mask_pos, 0.).r;
            }
            case 1u: {
                let tex_size = textureDimensions(tex_1);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_1, s_sampler, mask_pos, 0.).r;
            }
            case 2u: {
                let tex_size = textureDimensions(tex_2);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_2, s_sampler, mask_pos, 0.).r;
            }
            case 3u: {
                let tex_size = textureDimensions(tex_3);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_3, s_sampler, mask_pos, 0.).r;
            }
            case 4u: {
                let tex_size = textureDimensions(tex_4);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_4, s_sampler, mask_pos, 0.).r;
            }
            case 5u: {
                let tex_size = textureDimensions(tex_5);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_5, s_sampler, mask_pos, 0.).r;
            }
            case 6u: {
                let tex_size = textureDimensions(tex_6);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_6, s_sampler, mask_pos, 0.).r;
            }
            case 7u: {
                let tex_size = textureDimensions(tex_7);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_7, s_sampler, mask_pos, 0.).r;
            }
            default: { alpha = 0.; }
        };

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
