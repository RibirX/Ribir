#version 450

layout(location=0) in vec2 pos;
layout(location=1) in uint prim_id;
layout(location=0) out vec3 v_color;

layout(set=0, binding=0) 
uniform CoordinateConvertMatrix {
     float m11;
     float m12;
     float m21;
     float m22;
     float m31;
     float m32;
};


void main() {
    mat3x2 axis_map = mat3x2(
        m11, m12,
        m21, m22,
        m31, m32
    );

    v_color = vec3(0.5, 0.0, 0.5);
    vec2 mapped = axis_map * vec3( pos, 1);
    gl_Position = vec4(mapped, 0, 1.0);

}