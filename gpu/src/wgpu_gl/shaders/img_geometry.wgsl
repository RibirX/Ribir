/// Vertex Shader

struct Transform2d {
  r1: vec2<f32>;
  r2: vec2<f32>;
  r3: vec2<f32>;
};

struct Primitive {
  offset: vec2<u32>;
  factor: vec2<f32>;
  transform: Transform2d;
};

struct VertexInput {
  [[location(0)]] pos: vec2<f32>;
  [[location(1)]] prim_id: u32;
};


[[group(0), binding(0)]]
var<uniform> global_uniform: Transform2d;

struct PrimitiveInfo {
  primitives: array<Primitive>;
};

[[group(1), binding(0)]]
var<storage> primitive_info: PrimitiveInfo;
[[group(1), binding(1)]]
var texture: texture_2d<f32>;
[[group(1), binding(2)]]
var s_sampler: sampler;

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
  [[location(0)]] texture_offset: vec2<f32>;
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

  // todo
  
  return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
  // todo
  return vec4<f32>(0.2, 0.2, 0.4, 0.5);
}
