#version 330

layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in uvec2 path;

out vec2 v_uv;
flat out uvec2 v_path;

void main() {
    gl_Position = vec4(pos, 0.0, 1.0);
    v_uv = uv;
    v_path = path;
}
