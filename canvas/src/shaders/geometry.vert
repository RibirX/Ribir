#version 450
struct Transform2d {
    float m11;
     float m12;
     float m21;
     float m22;
     float m31;
     float m32;
};

struct Primitive {
  vec2 tex_offset;
  vec2 tex_size;
  Transform2d transform;
};


layout(location=0) in vec2 pos;
layout(location=1) in uint prim_id;

layout(std140, set=0, binding=0) 
uniform global_uniform {
    Transform2d canvas_2d_two_wgpu;
    vec2 texture_atlas_size;
};

layout(std140, set=1, binding=1) 
buffer texture_info {
    Primitive primitives[];
};

layout(location=0) out vec3 v_color;


mat3x2 transform2d_to_max(Transform2d t) {
    return mat3x2(
        t.m11, t.m12,
        t.m21, t.m22,
        t.m31, t.m32
    );
}

void main() {
    Primitive prim = primitives[prim_id];

    // vertext position calc
    mat3x2 axis_map = transform2d_to_max(canvas_2d_two_wgpu);
    mat3x2 transform = transform2d_to_max(prim.transform);
    vec2 mapped = transform * vec3(axis_map *  vec3(pos, 1), 1);
    gl_Position = vec4(mapped, 0, 1.0);

    // sampler pick 
    v_color = vec3(0.5, 0.0, 0.5);

}