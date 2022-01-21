/// Vertex Shader

struct Transform2d {
  r1: vec2<f32>;
  r2: vec2<f32>;
  r3: vec2<f32>;
};

struct Primitive {
  tex_offset: vec2<f32>;
  tex_size: vec2<f32>;
  bounding_min: vec2<f32>;
  bounding_size: vec2<f32>;
  transform: Transform2d;
};

struct VertexInput {
  [[location(0)]] pos: vec2<f32>;
  [[location(1)]] prim_id: u32;
};

struct GlobalUniform {
  r1: vec2<f32>;
  r2: vec2<f32>;
  r3: vec2<f32>;
  atlas_size: vec2<u32>;
};
[[group(0), binding(0)]]
var<uniform> global_uniform: GlobalUniform;

struct PrimitiveInfo {
  primitives: array<Primitive>;
};
[[group(1), binding(0)]]
var<storage> primitive_info: PrimitiveInfo;

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
  [[location(0)]] atlas_tex_coords: vec2<f32>;
  [[location(1)]] atlas_sub_tex_size: vec2<f32>;
  [[location(2)]] atlas_sub_tex_offset: vec2<f32>;
  [[location(3)]] f_atlas_size: vec2<u32>;
};

[[stage(vertex)]]
fn vs_main(model: VertexInput) -> VertexOutput {
  let prim: Primitive = primitive_info.primitives[model.prim_id];
  let t: Transform2d = prim.transform;
  let transform: mat3x2<f32> = mat3x2<f32>(t.r1, t.r2, t.r3);

  let canvas_coord: vec2<f32> = transform * vec3<f32>(model.pos, 1.0);
  let pos2d: vec2<f32> = mat3x2<f32>(global_uniform.r1, global_uniform.r2, global_uniform.r3) * vec3<f32>(canvas_coord, 1.0);

  var out: VertexOutput;
  out.clip_position = vec4<f32>(pos2d, 0.0, 1.0);

  out.atlas_tex_coords = model.pos - prim.bounding_min + prim.tex_offset;
  out.atlas_sub_tex_size = prim.tex_size;
  out.atlas_sub_tex_offset = prim.tex_offset;
  out.f_atlas_size = global_uniform.atlas_size;

  return out;
}

/// Fragment Shader

[[group(0), binding(1)]]
var t_atals: texture_2d<f32>;
[[group(0), binding(3)]]
var s_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  // For now, always use repeat pattern to fill.
  var tex_pos: vec2<f32> = in.atlas_tex_coords - in.atlas_sub_tex_offset;
  tex_pos[0] = tex_pos[0] % in.atlas_sub_tex_size[0];
  tex_pos[1] = tex_pos[1] % in.atlas_sub_tex_size[1];
  tex_pos = tex_pos + in.atlas_sub_tex_offset;

  tex_pos[0] = tex_pos[0] / in.atlas_sub_tex_size[0];
  tex_pos[1] = tex_pos[1] / in.atlas_sub_tex_size[1];
  

  var f_color: vec4<f32> = textureSample(t_atals, s_sampler, in.atlas_tex_coords);
  // rbga fomat texture store in a Bgra8UnormSrgb texture.
  f_color = vec4<f32>(f_color.b, f_color.g, f_color.r, 1.0);

  return f_color;
}
