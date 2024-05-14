struct Vertex {
  @location(0) pos: vec2<f32>,
  @location(1) @interpolate(flat) prim_idx: u32,
};

struct FragInput {
  @builtin(position) pos: vec4<f32>,
  @location(0) @interpolate(flat) prim_idx: u32,
}

@vertex
fn vs_main(v: Vertex) -> FragInput {
    var input: FragInput;
    // convert from gpu-backend coords(0..1) to wgpu corrds(-1..1)
    let pos = v.pos * vec2(2., -2.) + vec2(-1., 1.);
    input.pos = vec4<f32>(pos, 0.0, 1.0);
    input.prim_idx = v.prim_idx;
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
  mask_tex_idx: u32,
  prev_mask_idx: i32,
}

// A pair of stops. This arrangement aligns the stops with 16 bytes, minimizing excessive padding.
struct StopPair {
    color1: u32,
    offset1: f32,
    color2: u32,
    offset2: f32,
}

struct Stop {
    color: vec4<f32>,
    offset: f32,
}

// Since a the different alignment between WebGPU and WebGL, we not use 
// mat3x2<f32> in the struct, but use vec2<f32> instead. Then, we compose it.
struct Primitive {
  t0: vec2<f32>,
  t1: vec2<f32>,
  t2: vec2<f32>,
  start_position: vec2<f32>,
  end_position: vec2<f32>,
  // A value mixed stop_start(u16) and stop_cnt(u16)
  stop: u32,
  // A value mixed mask_head(i16) and spread(u16)
  mask_head_and_spread: i32
}

fn mask_matrix(mask: MaskLayer) -> mat3x2<f32> {
  return mat3x2(mask.t0, mask.t1, mask.t2);
}

fn prim_matrix(img: Primitive) -> mat3x2<f32> {
  return mat3x2(img.t0, img.t1, img.t2);
}


@group(0) @binding(0) 
var<uniform> mask_layers: array<MaskLayer, 1365>;

@group(1) @binding(0)
var<uniform> stops: array<StopPair, 256>;

@group(2) @binding(0)
var<uniform> prims: array<Primitive, 512>;

@group(3) @binding(0)
var s_sampler: sampler;
@group(3) @binding(1)
var tex_0: texture_2d<f32>;
@group(3) @binding(2)
var tex_1: texture_2d<f32>;
@group(3) @binding(3)
var tex_2: texture_2d<f32>;
@group(3) @binding(4)
var tex_3: texture_2d<f32>;
@group(3) @binding(5)
var tex_4: texture_2d<f32>;
@group(3) @binding(6)
var tex_5: texture_2d<f32>;
@group(3) @binding(7)
var tex_6: texture_2d<f32>;
@group(3) @binding(8)
var tex_7: texture_2d<f32>;


fn calc_offset(x: f32, y: f32, x_0: f32, y_0: f32, x_1: f32, y_1: f32) -> f32 {
    let dx_0 = x - x_0;
    let dy_0 = y - y_0;
    let dx_1_0 = x_1 - x_0;
    let dy_1_0 = y_1 - y_0;

    return (dx_0 * dx_1_0 + dy_0 * dy_1_0) / (dx_1_0 * dx_1_0 + dy_1_0 * dy_1_0);
}

fn unpackUnorm4x8(packed: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((packed & 0xff000000) >> 24) / 255.0,
        f32((packed & 0x00ff0000) >> 16) / 255.0,
        f32((packed & 0x0000ff00) >> 8) / 255.0,
        f32((packed & 0x000000ff) >> 0) / 255.0
    );
}

fn get_stop(idx: u32)  -> Stop {
    let pair = stops[idx / 2];
    if idx % 2 == 0 {
        return Stop(unpackUnorm4x8(pair.color1), pair.offset1);
    } else {
        return Stop(unpackUnorm4x8(pair.color2), pair.offset2);
    }
}

@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    let prim = prims[input.prim_idx];
    let pos = prim_matrix(prim) * vec3(input.pos.xy, 1.);
    var alpha = 1.;
    var mask_idx = prim.mask_head_and_spread >> 16;

    loop {
        if mask_idx < 0 {
            break;
        }
        let mask = mask_layers[u32(mask_idx)];

        var mask_pos = mask_matrix(mask) * vec3(input.pos.xy, 1.);
        if any(mask_pos < mask.min) || any(mask.max < mask_pos) {
            alpha = 0.;
            break;
        }

        switch mask.mask_tex_idx {
            case 0u: {
                let tex_size = textureDimensions(tex_0);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_0, s_sampler, mask_pos, 0.).r;
            }
            case 1u: {
                let tex_size = textureDimensions(tex_1);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_1, s_sampler, mask_pos, 0.).r;
            }
            case 2u: {
                let tex_size = textureDimensions(tex_2);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_2, s_sampler, mask_pos, 0.).r;
            }
            case 3u: {
                let tex_size = textureDimensions(tex_3);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_3, s_sampler, mask_pos, 0.).r;
            }
            case 4u: {
                let tex_size = textureDimensions(tex_4);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_4, s_sampler, mask_pos, 0.).r;
            }
            case 5u: {
                let tex_size = textureDimensions(tex_5);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_5, s_sampler, mask_pos, 0.).r;
            }
            case 6u: {
                let tex_size = textureDimensions(tex_6);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = alpha * textureSampleLevel(tex_6, s_sampler, mask_pos, 0.).r;
            }
            case 7u: {
                let tex_size = textureDimensions(tex_7);
                mask_pos = mask_pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
                alpha = textureSampleLevel(tex_7, s_sampler, mask_pos, 0.).r;
            }
            default: { alpha = 0.; }
        };

        if alpha == 0. {
            break;
        }
        mask_idx = mask.prev_mask_idx;
    }

    if prim.start_position.x == prim.end_position.x && prim.start_position.y == prim.end_position.y {
        return vec4<f32>(1., 1., 1., alpha);
    }
    var offset = calc_offset(pos.x, pos.y, prim.start_position.x, prim.start_position.y, prim.end_position.x, prim.end_position.y);
    
    let spread = abs(prim.mask_head_and_spread & 0x0000ffff);
    if spread == 0 {
        // pad
        offset = min(1., max(0., offset));
    } else if spread == 1 {
        //reflect
        offset = 1. - abs(fract(offset / 2.) - 0.5) * 2.;
    } else {
        //repeat
        offset = fract(offset);
    }

    let stop_start = prim.stop >> 16;
    let stop_cnt = prim.stop & 0x0000ffff;
    var prev = get_stop(stop_start);
    var next = get_stop(stop_start + 1);
    for (var i = 2u; i < stop_cnt && next.offset < offset; i++) {
        prev = next;
        next = get_stop(stop_start + i);
    }

    offset = max(prev.offset, min(next.offset, offset));
    let weight1 = (next.offset - offset) / (next.offset - prev.offset);
    let weight2 = 1. - weight1;
    return (prev.color * weight1 + next.color * weight2) * vec4<f32>(1., 1., 1., alpha);
}
