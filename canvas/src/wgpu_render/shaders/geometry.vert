#version 450
struct Transform2d {
    vec2 r1;
    vec2 r2;
    vec2 r3;
};

struct Primitive {
  uvec2 tex_offset;
  uvec2 tex_size;
  vec2 bounding_min;
  vec2 bounding_size;
  Transform2d transform;
};


layout(location=0) in vec2 pos;
layout(location=1) in vec2 font_tex_pos;
layout(location=2) in uint prim_id;

layout(set=0, binding=0) 
uniform global_uniform {
    vec2 r1;
    vec2 r2;
    vec2 r3;
    uvec2 atlas_size;
};

layout(set=1, binding=0) 
buffer primitive_info {
    Primitive primitives[];
};

layout(location=0) out vec2 atlas_tex_coords;
layout(location=1) out vec2 atlas_sub_tex_size;
layout(location=2) out vec2 atlas_sub_tex_offset;
layout(location=3) out vec2 f_atlas_size;
layout(location=4) out vec2 f_font_tex_pos;


void main() {
    Primitive prim = primitives[prim_id];
    Transform2d t = prim.transform;
    mat3x2 transform = mat3x2(t.r1, t.r2, t.r3);

    vec2 canvas_coord = transform * vec3(pos, 1);
    vec2 pos2d = mat3x2(r1, r2, r3) * vec3(canvas_coord, 1);
    gl_Position = vec4(pos2d, 0, 1.0);

    atlas_tex_coords = pos - prim.bounding_min + prim.tex_offset;
    atlas_sub_tex_size = prim.tex_size;
    atlas_sub_tex_offset = prim.tex_offset;
    f_atlas_size = atlas_size;
    f_font_tex_pos = font_tex_pos;
}