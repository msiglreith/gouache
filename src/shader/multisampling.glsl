#version 330

uniform sampler2D curves;

in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    vec2 x_offset = dFdx(v_uv);
    vec2 y_offset = dFdy(v_uv);

    float alpha = 0.0;
    for (uint i = v_path.x; i < v_path.y; i++) {
        uint count = 0u;
        for (int sx = 0; sx < 16; sx++) {
            for (int sy = 0; sy < 16; sy++) {
                vec2 uv = v_uv + (float(sx) / 16.0 - 0.5) * x_offset + (float(sy) / 16.0 - 0.5) * y_offset;

                vec3 a = texelFetch(curves, ivec2(2u * i, 0), 0).xyz;
                vec3 b = texelFetch(curves, ivec2(2u * i + 1u, 0), 0).xyz;
                vec2 p1 = a.xy;
                vec2 p2 = vec2(a.z, b.x);
                vec2 p3 = b.yz;
                vec2 e1 = 2.0 * (p2 - p1);
                vec2 e2 = p3 - 2.0 * p2 + p1;
                vec2 p = uv - p1;
                mat2 m = mat2(e2.y, -e2.x, -e1.y, e2.y) / (e1.x * e2.y - e1.y * e2.x);
                vec2 q = m * p;

                if (q.x * q.x < q.y) { count++; }
            }
        }
        alpha += float(count) / 256.0;
    }
    f_col = vec4(alpha);
}
