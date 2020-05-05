#version 450

layout(location=0) in vec2 pos;
layout(location=1) in uint prim_id;

layout(location=0) out vec3 v_color;

void main() {
    v_color = vec3(0.5, 0.0, 0.5);
    gl_Position = vec4(pos, 0, 1.0);
}