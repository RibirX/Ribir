struct Vertex {
  @location(0) pos: vec2<f32>,
  @location(1) prim_idx: u32,
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

struct Primitive {
  transform: mat3x2<f32>,
  stop_start: u32,
  stop_cnt: u32,
  start_center: vec2<f32>,
  end_center: vec2<f32>,
  start_radius: f32,
  end_radius: f32,
  mask_head: i32,
  spread: u32, // 0 for pad, 1 for reflect, 2 for repeat
}

@group(0) @binding(0) 
var<storage> mask_layers: array<MaskLayer>;

@group(1) @binding(0)
var<storage> stops: array<StopPair>;

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

    let res = calc_offset(pos.x, pos.y, prim.start_center.x, prim.start_center.y, prim.start_radius, prim.end_center.x, prim.end_center.y, prim.end_radius);

    if res[0] < 0. || (prim.start_radius != prim.end_radius && res[1] < (prim.start_radius / (prim.start_radius - prim.end_radius))) {
        return vec4<f32>(1., 1., 1., alpha);
    }
    var offset = res[1];
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

    var prev = get_stop(prim.stop_start);
    var next = get_stop(prim.stop_start + 1);
    for (var i = 2u; i < prim.stop_cnt && next.offset < offset; i++) {
        prev = next;
        next = get_stop(prim.stop_start + i);
    }

    offset = max(prev.offset, min(next.offset, offset));
    let weight1 = (next.offset - offset) / (next.offset - prev.offset);
    let weight2 = 1. - weight1;
    return (prev.color * weight1 + next.color * weight2) * vec4<f32>(1., 1., 1., alpha);
}
// input the center and radius of the circles, return the tag of resolvable (1. mean resolvable and -1. unresolvable) and the offset if tag is resolvable.
fn calc_offset(x: f32, y: f32, x_0: f32, y_0: f32, r_0: f32, x_1: f32, y_1: f32, r_1: f32) -> vec2<f32> {
    // see definition at https://html.spec.whatwg.org/multipage/canvas.html#dom-context-2d-createradialgradient
    // with offset ω, Radial gradients must be rendered by following these steps:
    //     1. If x0 = x1 and y0 = y1 and r0 = r1, then the radial gradient must paint nothing. Return.
    //         Let x(ω) = (x1-x0)ω + x0
    //         Let y(ω) = (y1-y0)ω + y0
    //         Let r(ω) = (r1-r0)ω + r0
    //     2. Let the color at ω be the color at that position on the gradient (with the colors coming from the interpolation
    //        and extrapolation described above).
    //     3. For all values of ω where r(ω) > 0, starting with the value of ω nearest to positive infinity and ending with 
    //        the value of ω nearest to negative infinity, draw the circumference of the circle with radius r(ω) at position
    //        (x(ω), y(ω)), with the color at ω, but only painting on the parts of the bitmap that have not yet been painted
    //        on by earlier circles in this step for this rendering of the gradient.

    //     so the offset ω meet the following equation: (x(ω) - x)^2 + (y(ω) - y)^2 = r(ω)^2. 
    //     we sovle the equation and get the offset ω with the min r.
    //     define: 
    //         dx_0 = x - x_0;
    //         dx_1_0 = x_1 - x_0;
    //         dy_0 = y - y_0;
    //         dy_1_0 = y_1 - y_0;
    //         dr_1_0 = r_1 - r_0;
    //     the (x(ω) - x)^2 + (y(ω) - y)^2 = r(ω)^2 can be rewrite as:
    //         (dx_1_0^2 + dy_1_0^2 - dr_1_0^2) * ω^2 - 2 * (dx_1_0 * dx_0 + dy_1_0 * dy_0 + dr_1_0 * r_0) * ω + (dx_0^2 + dy_0^2 - r_0^2) = 0
    //     the ω can be solve by the quadratic formula:
    //         ω = (-b ± sqrt(b^2 - 4ac)) / 2a
    //         where a = dx_1_0^2 + dy_1_0^2 - dr_1_0^2
    //             b = -2 * (dx_1_0 * dx_0 + dy_1_0 * dy_0 + dr_1_0 * r_0)
    //             c = dx_0^2 + dy_0^2 - r_0^2

    let dx_0 = x - x_0;
    let dx_1_0 = x_1 - x_0;
    let dy_0 = y - y_0;
    let dy_1_0 = y_1 - y_0;
    let dr_1_0 = r_1 - r_0;
    let a = dx_1_0 * dx_1_0 + dy_1_0 * dy_1_0 - dr_1_0 * dr_1_0;
    let b = -2. * (dx_1_0 * dx_0 + dy_1_0 * dy_0 + dr_1_0 * r_0);
    let c = dx_0 * dx_0 + dy_0 * dy_0 - r_0 * r_0;

    let delta = b * b - 4. * a * c;

    if abs(a) < 0.1 {
        if abs(b) < 0.1 {
            return vec2(-1., 0.);
        } else {
            return vec2(1., -c / b);
        }
    } else if delta < 0. {
        return vec2(-1., 0.);
    }

    let sqrt_delta = sqrt(delta);
    let _2a = 2. * a;
    let w1 = (-b + sqrt_delta) / _2a;
    let w2 = (-b - sqrt_delta) / _2a;

    return vec2(1., max(w1, w2));
}