/// Vertex Shader

struct Transform2d {
  r1: vec2<f32>,
  r2: vec2<f32>,
  r3: vec2<f32>,
};

struct Primitive {
  rgba: array<f32, 4>,
  transform: Transform2d,
  opacity: f32,
  dummy: f32,
};

struct Uniform {
  matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> coord_matrix: Uniform;

struct PrimitiveInfo {
  primitives: array<Primitive>,
};
@group(1) @binding(0)
var<storage> primitive_info: PrimitiveInfo;

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) f_color: vec4<f32>,
};


@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) prim_id: u32) -> VertexOutput {
  let prim: Primitive = primitive_info.primitives[prim_id];
  let t: Transform2d = prim.transform;
  let transform: mat3x2<f32> = mat3x2<f32>(t.r1, t.r2, t.r3);
  let canvas_coord: vec2<f32> = transform * vec3<f32>(pos, 1.0);

  var out: VertexOutput;

  out.clip_position = coord_matrix.matrix * vec4<f32>(canvas_coord, 0.0, 1.0);
  let rgba = prim.rgba;
  out.f_color = vec4<f32>(rgba[0], rgba[1], rgba[2], rgba[3] * prim.opacity );

  return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return in.f_color;
}
