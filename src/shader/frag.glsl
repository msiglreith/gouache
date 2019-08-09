#version 330

uniform sampler2D curves;

in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    float brightness = 0.0;
    float scale = mix(1.0, 1.4142135624, brightness);
    vec2 footprint = scale * vec2(length(vec2(dFdx(v_uv.x), dFdy(v_uv.x))), length(vec2(dFdx(v_uv.y), dFdy(v_uv.y))));

    float alpha = 0.0;
    for (uint i = v_path.x; i < v_path.y; i++) {
        vec3 t1 = texelFetch(curves, ivec2(2u * i, 0), 0).xyz;
        vec3 t2 = texelFetch(curves, ivec2(2u * i + 1u, 0), 0).xyz;
        vec2 p1 = t1.xy;
        vec2 p2 = vec2(t1.z, t2.x);
        vec2 p3 = t2.yz;

        vec2 y_window = clamp(vec2(p1.y - v_uv.y, p3.y - v_uv.y) / footprint.y, -0.5, 0.5);
        float g = y_window.y - y_window.x;

        vec2 x_window = clamp(vec2(min(p1.x, p3.x) - v_uv.x, max(p1.x, p3.x) - v_uv.x) / footprint.x, -0.5, 0.5);

        if (abs(g) > 0 && x_window.y > -0.5) {
            vec2 e1 = 2.0 * (p2 - p1);
            vec2 e2 = p3 - (p1 + e1);
            vec2 p = v_uv - p1;
            mat2 m = mat2(e2.y, -e1.y, -e2.x, e1.x);
            float det = e2.y * e1.x - e2.x * e1.y;
            vec2 q = m * p;

            float f;
            vec2 grad;
            if (abs(det) < 1e-5) {
                vec2 e3 = p3 - p1;
                f = p.y * e3.x - p.x * e3.y;
                grad = vec2(-e3.y, e3.x);
            } else {
                f = sign(q.x) * (det * q.y - q.x * q.x);
                grad = vec2(-2.0 * q.x, det) * m;
            }
            float df = length(grad * footprint);
            if (df != 0) { f /= df; }

            alpha += g * (0.5 + x_window.x + (x_window.y - x_window.x) * smoothstep(0.0, 1.0, 0.5 + sign(g) * f));
        }
    }

    f_col = clamp(mix(sqrt(abs(alpha)), alpha * alpha, brightness), 0.0, 1.0) * vec4(brightness, brightness, brightness, 1.0);
}
