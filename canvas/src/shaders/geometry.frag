#version 450

layout(location=0) in vec2 v_tex_coords;
layout(location=1) in vec2 v_text_size;
layout(location=2) in vec2 v_text_offset;
layout(location=0) out vec4 f_color;

layout(set = 0, binding = 1) uniform texture2D t_atals;
layout(set = 0, binding = 2) uniform sampler s_atlas;

void main() {

    // For now, always use repeat pattern to fill.
    vec2 tex_pos = v_tex_coords - v_text_offset;
    tex_pos[0] = mod(tex_pos[0], v_text_size[0]);
    tex_pos[1] = mod(tex_pos[1], v_text_size[1]);
    tex_pos += v_text_offset;

    mat2x3 tex_coord_map = mat2x3(
        tex_pos[0] / v_text_size[0], 0, 0
        ,0, - tex_pos[1] / v_text_size[1], 1
    );
    
    tex_pos = vec3(tex_pos, 1.) * tex_coord_map;

    f_color = texture(sampler2D(t_atals, s_atlas), tex_pos);
}