/// Vertex Shader

struct Transform2d {
  r1: vec2<f32>;
  r2: vec2<f32>;
  r3: vec2<f32>;
};

struct Primitive {
  texture_rect: vec2<u32>;
  factor: vec2<f32>;
  transform: Transform2d;
  opacity: f32;
  dummy: f32;
};

struct Uniform {
  matrix: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> coord_matrix: Uniform;
[[group(0), binding(1)]]
var texture: texture_2d<f32>;
[[group(0), binding(2)]]
var s_sampler: sampler;

struct PrimitiveInfo {
  primitives: array<Primitive>;
};

[[group(1), binding(0)]]
var<storage> primitive_info: PrimitiveInfo;

struct VertexOutput {
  [[builtin(position)]] clip_position: vec4<f32>;
  [[location(0)]] tex_pos: vec2<f32>;
  [[location(1)]] tex_size: vec2<f32>;
  [[location(2)]] opacity: f32;
};

[[stage(vertex)]]
fn vs_main([[location(0)]] pos: vec2<f32>, [[location(1)]] prim_id: u32) -> VertexOutput {
  let prim: Primitive = primitive_info.primitives[prim_id];
  let t: Transform2d = prim.transform;
  let transform: mat3x2<f32> = mat3x2<f32>(t.r1, t.r2, t.r3);
  let canvas_coord: vec2<f32> = transform * vec3<f32>(pos, 1.0);

  var out: VertexOutput;
  out.clip_position = coord_matrix.matrix * vec4<f32>(canvas_coord, 0.0, 1.0);
  
  let u16_bits = 16u;
  let u16_mask = 0x0000FFFFu;
  let x = f32(prim.texture_rect[0] & u16_mask);
  let y = f32(prim.texture_rect[0] >> u16_bits);
  let width = f32(prim.texture_rect[1] & u16_mask);
  let height = f32(prim.texture_rect[1] >> u16_bits);
  out.tex_pos = pos * prim.factor + vec2<f32>(x,y);
  out.tex_size = vec2<f32>(width, height);
  out.opacity = prim.opacity;
  
  return out;
}

[[stage(fragment)]]
fn fs_main([[location(0)]] tex_pos: vec2<f32>, [[location(1)]] tex_size: vec2<f32>, [[location(2)]] opacity: f32) -> [[location(0)]] vec4<f32> {
  let pos = tex_pos % tex_size; 
  let size = vec2<f32>(textureDimensions(texture));
  let coord = pos / size;
  var rgba = textureSample(texture, s_sampler, coord);
  rgba[3] = rgba[3] * opacity;
  return rgba;
}
