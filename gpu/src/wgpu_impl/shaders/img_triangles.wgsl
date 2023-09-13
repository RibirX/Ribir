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
  @location(0) prim_idx: u32
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
var textures: binding_array<texture_2d<f32>>;
@group(2) @binding(1)
var samplers: binding_array<sampler>;

@fragment
fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
    let prim = primtives[f.prim_idx];
    let img_tex = textures[prim.img_tex_idx];
    let img_smapler = samplers[prim.img_tex_idx];

    let pos = prim.transform * f.pos.xyz;
    var img_pos = pos.xy % prim.img_size + prim.img_start;
    let img_tex_size = textureDimensions(img_tex);
    img_pos = img_pos / vec2<f32>(f32(img_tex_size.x), f32(img_tex_size.y));

    var color = textureSample(img_tex, img_smapler, img_pos);
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

        let mask_tex_idx = mask.mask_tex_idx;
        let mask_tex = textures[mask_tex_idx];
        let mask_sampler = samplers[mask_tex_idx];
        let mask_tex_size = textureDimensions(mask_tex);
        mask_pos = mask_pos / vec2<f32>(f32(mask_tex_size.x), f32(mask_tex_size.y));
        let alpha = textureSampleLevel(mask_tex, mask_sampler, mask_pos, 0.).r;
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

