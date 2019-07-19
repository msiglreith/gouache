#version 330

uniform sampler2D curves;

in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    mat2 j = mat2(dFdx(v_uv), dFdy(v_uv));

    float alpha = 0.0;
    for (uint i = v_path.x; i < v_path.y; i++) {
        vec3 a = texelFetch(curves, ivec2(2u * i, 0), 0).xyz;
        vec3 b = texelFetch(curves, ivec2(2u * i + 1u, 0), 0).xyz;
        vec2 p1 = a.xy;
        vec2 p2 = vec2(a.z, b.x);
        vec2 p3 = b.yz;
        vec2 e1 = 2.0 * (p2 - p1);
        vec2 e2 = p3 - 2.0 * p2 + p1;
        vec2 p = v_uv - p1;
        mat2 m = mat2(e2.y, -e2.x, -e1.y, e2.y) / (e1.x * e2.y - e1.y * e2.x);
        vec2 q = m * p;
        float f = q.y - q.x * q.x;
        vec2 g = vec2(2.0 * q.x, -1.0) * m * j;
        alpha = smoothstep(0.0, 1.0, 0.5 + 0.8862269 * f / length(g));
    }
    f_col = vec4(alpha);
}
