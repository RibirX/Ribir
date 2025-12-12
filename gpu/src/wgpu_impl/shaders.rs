//! WGSL Shader code for the GPU implementation.

use crate::DrawPhaseLimits;

pub fn radial_gradient_shader(limits: &DrawPhaseLimits) -> String {
  basic_template(limits.max_mask_layers)
    + &format!(
      r#"
@group(2) @binding(0)
var<uniform> prims: array<Primitive, {}>;

@group(3) @binding(0)
var<uniform> stops: array<StopPair, {}>;
    "#,
      limits.max_radial_gradient_primitives,
      limits.max_gradient_stop_primitives / 2,
    )
    + r#"
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
  stop_start: u32,
  stop_cnt: u32,
  start_center: vec2<f32>,
  end_center: vec2<f32>,
  start_radius: f32,
  end_radius: f32,
  mask_head: i32,
  spread: u32, // 0 for pad, 1 for reflect, 2 for repeat
}


@fragment
fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
    let prim = prims[input.prim_idx];
    let pos = mat3x2(prim.t0, prim.t1, prim.t2) * vec3(input.pos.xy, 1.);

    var alpha = 1.;
    var mask_idx = prim.mask_head;
    loop {
        if mask_idx < 0 { break; }

        let mask = mask_layers[u32(mask_idx)];
        alpha *= mask_sample(mask, input.pos.xy);
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

fn get_stop(idx: u32) -> Stop {
    let pair = stops[idx / 2];
    if idx % 2 == 0 {
        return Stop(unpackUnorm4x8(pair.color1), pair.offset1);
    } else {
        return Stop(unpackUnorm4x8(pair.color2), pair.offset2);
    }
}

fn unpackUnorm4x8(packed: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((packed & 0xff000000) >> 24) / 255.0,
        f32((packed & 0x00ff0000) >> 16) / 255.0,
        f32((packed & 0x0000ff00) >> 8) / 255.0,
        f32((packed & 0x000000ff) >> 0) / 255.0
    );
}"#
}

pub fn linear_gradient_shader(limits: &DrawPhaseLimits) -> String {
  basic_template(limits.max_mask_layers)
    + &format!(
      r#"
@group(2) @binding(0)
var<uniform> prims: array<Primitive, {}>;

@group(3) @binding(0)
var<uniform> stops: array<StopPair, {}>;"#,
      limits.max_linear_gradient_primitives,
      limits.max_gradient_stop_primitives / 2,
    )
    + r#"
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

fn get_stop(idx: u32) -> Stop {
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
    let pos = mat3x2(prim.t0, prim.t1, prim.t2) * vec3(input.pos.xy, 1.);

    var alpha = 1.;
    var mask_idx = prim.mask_head_and_spread >> 16;
    loop {
        if mask_idx < 0 { break; }

        let mask = mask_layers[u32(mask_idx)];
        alpha *= mask_sample(mask, input.pos.xy);
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
"#
}

pub fn color_triangles_shader(max_mask_layers: usize) -> String {
  basic_template(max_mask_layers)
    + r#"
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
  
  @fragment
  fn fs_main(input: FragInput) -> @location(0) vec4<f32> {
      var color = input.color;
      var mask_idx = input.mask_head;
      var alpha = 1.;
      let pos = input.pos.xy;
      loop {
          if mask_idx < 0 { break; }
  
          let mask = mask_layers[u32(mask_idx)];
          alpha *= mask_sample(mask, pos);
          mask_idx = mask.prev_mask_idx;
      }
  
      color.a *= alpha;
      return color;
  }
 "#
}

pub fn img_triangles_shader(limits: &DrawPhaseLimits) -> String {
  basic_template(limits.max_mask_layers)
    + &format!(
      "
      @group(2) @binding(0) 
      var<uniform> primtives: array<ImgPrimitive, {}>;",
      limits.max_image_primitives
    )
    + r#"
  struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) prim_idx: u32,
  }
  
  // Since a the different alignment between WebGPU and WebGL, we not use 
  // mat3x2<f32> in the struct, but use vec2<f32> instead. Then, we compose it.
  struct ImgPrimitive {
    /// Transform a vertex position to a image texture position.
    t0: vec2<f32>,
    t1: vec2<f32>,
    t2: vec2<f32>,
    /// The origin of the image placed in texture.
    img_start: vec2<f32>,
    /// The size of the image image.
    img_size: vec2<f32>,
    
    /// This is a mix field,
    /// - the high 16 bits is the index of head mask layer, as a i16 type.
    /// - the low 16 bits is the index of texture, as a u16 type.
    mask_head_and_tex_idx: i32,

    /// 1 for premultiplied alpha, 0 for non-premultiplied alpha
    is_premultiplied: i32,

    /// base color
    base_color: vec4<f32>,

    /// color matrix of [f32; 4 * 4]
    color_matrix: mat4x4<f32>,
  }
  
  struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) @interpolate(flat) prim_idx: u32
  }
  
  @vertex
  fn vs_main(v: VertexInput) -> VertexOutput {
      var o: VertexOutput;
      // convert from gpu-backend coords(0..1) to wgpu corrds(-1..1)
      let pos = v.pos * vec2(2., -2.) + vec2(-1., 1.);
      o.pos = vec4<f32>(pos, 1., 1.);
      o.prim_idx = v.prim_idx;
  
      return o;
  }
  
  
  @fragment
  fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
      let prim = primtives[f.prim_idx];
      let pos = mat3x2(prim.t0, prim.t1, prim.t2) * f.pos.xyz;
      var img_pos = pos.xy % prim.img_size + prim.img_start;
      var color = img_sample(prim, img_pos);
  
      var mask_idx = prim.mask_head_and_tex_idx >> 16 ;
      var alpha = 1.0;
      loop {
          if mask_idx < 0 { break; }
  
          let mask = mask_layers[u32(mask_idx)];
          alpha *= mask_sample(mask, f.pos.xy);
          mask_idx = mask.prev_mask_idx;
      }
      if prim.is_premultiplied == 1 && color.a > 0.0 {
        color = vec4<f32>(color.rgb / color.a, color.a);
      }
      color.a = color.a * alpha;
      return color * prim.color_matrix + prim.base_color;
  }
  
  fn img_sample(prim: ImgPrimitive, pos: vec2<f32>) -> vec4<f32> {
      switch abs(prim.mask_head_and_tex_idx & 0x0000FFFF) {
        case 0: { return img_tex_smaple(tex_0, prim, pos); }
        case 1: { return img_tex_smaple(tex_1, prim, pos); }
        case 2: { return img_tex_smaple(tex_2, prim, pos); }
        case 3: { return img_tex_smaple(tex_3, prim, pos); }
        case 4: { return img_tex_smaple(tex_4, prim, pos); }
        case 5: { return img_tex_smaple(tex_5, prim, pos); }
        case 6: { return img_tex_smaple(tex_6, prim, pos); }
        case 7: { return img_tex_smaple(tex_7, prim, pos); }
        // should not happen, use a red color to indicate error
        default: { return vec4<f32>(1., 0., 0., 1.); }
    };
  }
  
  fn img_tex_smaple(tex: texture_2d<f32>, prim: ImgPrimitive, pos: vec2<f32>) -> vec4<f32> {
      let img_tex_size = textureDimensions(tex);
      let sample_pos = pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));
      return textureSampleLevel(tex, s_sampler, sample_pos, 0.);
  }
  "#
}

pub fn filter_triangles_shader(limits: &DrawPhaseLimits) -> String {
  let filter_shader = r#"
  // Since a the different alignment between WebGPU and WebGL, we not use 
  // mat3x2<f32> in the struct, but use vec2<f32> instead. Then, we compose it.
  struct FilterPrimitive {
    /// The origin of the image placed in texture.
    /// Used to locate the original image position in the texture.
    sample_offset: vec2<f32>,
    
    /// Filter effect offset for drop-shadow etc.
    /// This offset is applied during filter convolution sampling.
    offset: vec2<f32>,

    /// The origin of the mask layer in the texture.
    mask_offset: vec2<f32>,

    /// The size of the filter kernel.
    kernel_size: vec2<u32>,

    /// the  index of head mask layer.
    mask_head: i32,

    /// The composite mode for filter application.
    /// - 0 = Replace: filter result completely replaces the original content
    /// - 1 = ExcludeSource: filter result is only applied where original alpha is 0
    composite: i32,

    /// for align
    dummy: vec2<f32>,

    /// base color
    base_color: vec4<f32>,

    /// color matrix of [f32; 4 * 4]
    color_matrix: mat4x4<f32>,

    kernel_matrix: array<vec4<f32>,"#
    .to_string()
    + &format!("{}", limits.max_filter_matrix_len / 4)
    + r#">,
  }

  @group(2) @binding(0) 
  var<uniform> filter_primitive: FilterPrimitive;

  @group(3) @binding(0)
  var original_tex: texture_2d<f32>;

  struct VertexInput {
    @location(0) pos: vec2<f32>,
  }

  struct VertexOutput {
    @builtin(position) pos: vec4<f32>
  }
  
  @vertex
  fn vs_main(v: VertexInput) -> VertexOutput {
      var o: VertexOutput;
      // convert from gpu-backend coords(0..1) to wgpu corrds(-1..1)
      let pos = v.pos * vec2(2., -2.) + vec2(-1., 1.);
      o.pos = vec4<f32>(pos, 1., 1.);
      return o;
  }
  
  @fragment
  fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
    var base = f.pos.xy;
    let alpha = sample_mask(filter_primitive, base);
    
    // Sample original color at current position (without offset) for composite operations.
    // This is critical for drop-shadow: we need to check if the original content exists
    // at the current output position, not at the offset position.
    base += filter_primitive.sample_offset;
    let origin = tex_sample(original_tex, base);

    let tex_size = textureDimensions(original_tex);

    // The movement shifts the final effect by filter_primitive.offset, which means we should
    // sample with an offset of -filter_primitive.offset.
    base -= filter_primitive.offset;

    let kernel_size = filter_primitive.kernel_size;
    let x_radius = f32(kernel_size.x >> 1);
    let y_radius = f32(kernel_size.y >> 1);
    var sum = vec4<f32>(0., 0., 0., 0.);
    for (var i: u32 = 0; i< kernel_size.x; i++) {
      for (var j: u32 = 0; j < kernel_size.y; j++) {
        let pos = base + vec2<f32>(f32(i) - x_radius, f32(j) - y_radius);
        let index = j * kernel_size.x + i;
        let weight = filter_primitive.kernel_matrix[index / 4][index % 4];

        // Filter convolution sampling uses sample_offset.
        // - sample_offset: locates the original image in the texture
        // For output pixel at (x, y), we sample from (x + sample_offset) for the filter.

        if pos.x < 0. || pos.x > f32(tex_size.x) || pos.y < 0. || pos.y > f32(tex_size.y) {
          continue;
        }
        let color = tex_sample(original_tex, pos);

        sum = sum + (color * weight);
      }
    }
    
    if alpha < 0.5 {
      // return origin color when
      //  - the alpha is 0., means it is out of the filter area
      //  - the 0. < alpha < 1., means it is in the edge of the filter area,
      //    here we return the origin color when it < 0.5, in the out-side of edge.
      return origin;
    }
    
    let filtered = sum * filter_primitive.color_matrix + filter_primitive.base_color;
    
    // Apply composite mode
    // composite == 0: Replace mode - return filtered result (existing behavior)
    // composite == 1: ExcludeSource mode - return original color if original alpha > 0
    // For drop-shadow: where original content exists (alpha > 0), show original;
    // where original is transparent, show the offset filtered result (shadow).
    if filter_primitive.composite == 1 && origin.a > 0.0 {
      // ExcludeSource: keep original where alpha > 0
      return origin;
    }
    
    return filtered;
  }

  fn sample_mask( prim: FilterPrimitive, pos: vec2<f32>) -> f32 {
    var mask_idx = prim.mask_head;
    var alpha = 1.0;
    let mask_pos = pos + prim.mask_offset;
    
    loop {
        if mask_idx < 0 { break; }

        let mask = mask_layers[u32(mask_idx)];
        alpha *= mask_sample(mask, mask_pos);
        mask_idx = mask.prev_mask_idx;
    }

    return alpha;
  }
  
  fn tex_sample(tex: texture_2d<f32>, pos: vec2<f32>) -> vec4<f32> {
      let tex_size = textureDimensions(tex);
      let sample_pos = pos / vec2<f32>(f32(tex_size.x), f32(tex_size.y));
      return textureSampleLevel(tex, s_sampler, sample_pos, 0.);
  }
  "#;
  basic_template(limits.max_mask_layers) + &filter_shader
}

fn basic_template(max_mask_layers: usize) -> String {
  format!(
    r#"
    @group(0) @binding(0) 
    var<uniform> mask_layers: array<MaskLayer, {max_mask_layers}>;
    "#
  ) + r#"
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

fn mask_sample(mask: MaskLayer, pos: vec2<f32>) -> f32 {
    switch mask.mask_tex_idx {
      case 0u: { return mask_tex_sampler(tex_0, mask, pos); }
      case 1u: { return mask_tex_sampler(tex_1, mask, pos); }
      case 2u: { return mask_tex_sampler(tex_2, mask, pos); }
      case 3u: { return mask_tex_sampler(tex_3, mask, pos); }
      case 4u: { return mask_tex_sampler(tex_4, mask, pos); }
      case 5u: { return mask_tex_sampler(tex_5, mask, pos); }
      case 6u: { return mask_tex_sampler(tex_6, mask, pos); }
      case 7u: { return mask_tex_sampler(tex_7, mask, pos); }
      // should not happen
      default: { return 0.; }
  };
}


fn mask_tex_sampler(tex: texture_2d<f32>, mask: MaskLayer, pos: vec2<f32>) -> f32 {
    var mask_pos = mat3x2(mask.t0, mask.t1, mask.t2) * vec3(pos, 1.);
    if any(mask_pos < mask.min) || any(mask.max < mask_pos) {
        return 0.;
    }
    let size = textureDimensions(tex);
    let tex_size = vec2(f32(size.x), f32(size.y));
    return textureSampleLevel(tex, s_sampler, mask_pos / tex_size, 0.).r;
}
"#
}

pub fn texture_triangles_shader(limits: &DrawPhaseLimits) -> String {
  basic_template(limits.max_mask_layers)
    + &format!(
      "
      @group(2) @binding(0) 
      var<uniform> primtives: array<TexturePrimitive, {}>;",
      limits.max_texture_primitives
    )
    + r#"
    struct VertexInput {
        @location(0) pos: vec2<f32>,
        @location(1) prim_idx: u32,
    }
    struct VertexOutput {
        @builtin(position) pos: vec4<f32>,
        @location(0) @interpolate(flat) prim_idx: u32,
    }

    struct TexturePrimitive {
        t0: vec2<f32>,
        t1: vec2<f32>,
        t2: vec2<f32>,
        mask_head: i32,
        opacity: f32,
        is_premultiplied: u32,
        pad1: u32,
        pad2: vec2<u32>,
    }

    @vertex
    fn vs_main(input: VertexInput) -> VertexOutput {
        var output: VertexOutput;
        let pos = input.pos * vec2(2., -2.) + vec2(-1., 1.);
        output.pos = vec4<f32>(pos, 0.0, 1.0);
        output.prim_idx = input.prim_idx;
        return output;
    }

    @group(2) @binding(1)
    var input_tex: texture_2d<f32>;

    @fragment
    fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
        let prim = primtives[input.prim_idx];
        
        let pos = mat3x2(prim.t0, prim.t1, prim.t2) * input.pos.xyz;
        
        // Sampling from the single bound input_tex
        let size = textureDimensions(input_tex);
        let sample_pos = pos.xy / vec2<f32>(f32(size.x), f32(size.y));
        var color = textureSampleLevel(input_tex, s_sampler, sample_pos, 0.);
        
        var alpha = 1.0;
        var mask_idx = prim.mask_head;

        loop {
            if (mask_idx < 0) { break; }
            let mask = mask_layers[u32(mask_idx)];
            alpha *= mask_sample(mask, input.pos.xy);
            mask_idx = mask.prev_mask_idx;
        }

        if (prim.is_premultiplied == 1u && color.a > 0.0) {
            color = vec4<f32>(color.rgb / color.a, color.a);
        }
        
        return vec4<f32>(color.rgb, color.a * prim.opacity);
    }

  "#
}
