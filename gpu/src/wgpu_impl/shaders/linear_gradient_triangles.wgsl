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


struct MaskLayer {
  transform: mat3x2<f32>,
  min: vec2<f32>,
  max: vec2<f32>,
  mask_tex_idx: u32,
  prev_mask_idx: i32,
}

struct Stop {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
    offset: f32,
}

struct Primitive {
  transform: mat3x2<f32>,
  stop_start: i32,
  stop_cnt: i32,
  start_position: vec2<f32>,
  end_position: vec2<f32>,
  mask_head: i32,
  spread: u32, // 0 for pad, 1 for reflect, 2 for repeat
}

@group(0) @binding(0) 
var<storage> mask_layers: array<MaskLayer>;

@group(1) @binding(0)
var<storage> stops: array<Stop>;

@group(2) @binding(0)
var<storage> prims: array<Primitive>;

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

@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    let prim = prims[input.prim_idx];
    let pos = prim.transform * vec3(input.pos.xy, 1.);
    var alpha = 1.;
    var mask_idx = prim.mask_head;
    loop {
        if mask_idx < 0 {
            break;
        }
        let mask = mask_layers[u32(mask_idx)];

        var mask_pos = mask.transform * vec3(input.pos.xy, 1.);
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

    if prim.spread == 0u {
        // pad
        offset = min(1., max(0., offset));
    } else if prim.spread == 1u {
        //reflect
        offset = 1. - abs(fract(offset / 2.) - 0.5) * 2.;
    } else {
        //repeat
        offset = fract(offset);
    }

    var prev = stops[prim.stop_start];
    var next = stops[prim.stop_start + 1];
    for (var i = 2; i < prim.stop_cnt && next.offset < offset; i++) {
        prev = next;
        next = stops[prim.stop_start + i];
    }

    offset = max(prev.offset, min(next.offset, offset));
    let weight1 = (next.offset - offset) / (next.offset - prev.offset);
    let weight2 = 1. - weight1;
    let prev_color = vec4<f32>(prev.red, prev.green, prev.blue, prev.alpha);
    let next_color = vec4<f32>(next.red, next.green, next.blue, next.alpha);
    return (prev_color * weight1 + next_color * weight2) * vec4<f32>(1., 1., 1., alpha);
}
