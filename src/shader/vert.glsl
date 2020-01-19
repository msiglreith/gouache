#version 330

layout(location = 0) in vec2 pos;
layout(location = 1) in vec4 col;
layout(location = 2) in vec2 uv;
layout(location = 3) in uvec2 path;

out vec4 v_col;
out vec2 v_uv;
flat out uvec2 v_path;

void main() {
    gl_Position = vec4(pos, 0.0, 1.0);
    v_col = col;
    v_uv = uv;
    v_path = path;
}
