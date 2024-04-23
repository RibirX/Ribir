struct VertexInput {
  @location(0) pos: vec2<f32>,
  @location(1) prim_idx: u32,
}

struct ImgPrimitive {
  /// Transform a vertex position to a image texture position.
  transform: mat3x2<f32>,
  /// The origin of the image placed in texture.
  img_start: vec2<f32>,
  /// The size of the image image.
  img_size: vec2<f32>,
  /// The index of texture, `load_color_primitives` method provide all textures
  /// a draw phase need.
  img_tex_idx: u32,
  /// The index of the head mask layer.
  mask_head: i32,
  /// extra alpha apply to current vertex
  opacity: f32,
  /// keep align to 8 bytes.
  dummy: u32,
}

struct VertexOutput {
  @builtin(position) pos: vec4<f32>,
  @location(0) @interpolate(flat) prim_idx: u32
}

@group(0) @binding(0) 
var<storage> mask_layers: array<MaskLayer>;

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var o: VertexOutput;
    // convert from gpu-backend coords(0..1) to wgpu corrds(-1..1)
    let pos = v.pos * vec2(2., -2.) + vec2(-1., 1.);
    o.pos = vec4<f32>(pos, 1., 1.);
    o.prim_idx = v.prim_idx;

    return o;
}

struct MaskLayer {
  transform: mat3x2<f32>,
  min: vec2<f32>,
  max: vec2<f32>,
  mask_tex_idx: u32,
  prev_mask_idx: i32,
}



@group(1) @binding(0) 
var<storage> primtives: array<ImgPrimitive>;

@group(2) @binding(0)
var s_sampler: sampler;
@group(2) @binding(1)
var tex_0: texture_2d<f32>;
@group(2) @binding(2)
var tex_1: texture_2d<f32>;
@group(2) @binding(3)
var tex_2: texture_2d<f32>;
@group(2) @binding(4)
var tex_3: texture_2d<f32>;
@group(2) @binding(5)
var tex_4: texture_2d<f32>;
@group(2) @binding(6)
var tex_5: texture_2d<f32>;
@group(2) @binding(7)
var tex_6: texture_2d<f32>;
@group(2) @binding(8)
var tex_7: texture_2d<f32>;


@fragment
fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
    let prim = primtives[f.prim_idx];
    var color: vec4<f32>;
    let pos = prim.transform * f.pos.xyz;
    var img_pos = pos.xy % prim.img_size + prim.img_start;
    switch prim.img_tex_idx {
        case 0u: {
            let img_tex_size = textureDimensions(tex_0);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_0, s_sampler, img_pos);
        }
        case 1u: {
            let img_tex_size = textureDimensions(tex_1);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_1, s_sampler, img_pos);
        }
        case 2u: {
            let img_tex_size = textureDimensions(tex_2);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_2, s_sampler, img_pos);
        }
        case 3u: {
            let img_tex_size = textureDimensions(tex_3);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_3, s_sampler, img_pos);
        }
        case 4u: {
            let img_tex_size = textureDimensions(tex_4);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_4, s_sampler, img_pos);
        }
        case 5u: {
            let img_tex_size = textureDimensions(tex_5);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_5, s_sampler, img_pos);
        }
        case 6u: {
            let img_tex_size = textureDimensions(tex_6);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_6, s_sampler, img_pos);
        }
        case 7u: {
            let img_tex_size = textureDimensions(tex_7);
            img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
            color = textureSample(tex_7, s_sampler, img_pos);
        }
        default: { color = vec4<f32>(1., 0., 0., 1.); }
      };


    var mask_idx = prim.mask_head;
    loop {
        if mask_idx < 0 {
            break;
        }

        let mask = mask_layers[u32(mask_idx)];
        var mask_pos = mask.transform * vec3(f.pos.xy, 1.0);
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

    color.a = color.a * prim.opacity;
    return color;
}

