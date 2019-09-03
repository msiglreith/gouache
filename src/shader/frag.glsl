#version 330

uniform sampler2D curves;

in vec4 v_col;
in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    vec2 ddx = dFdx(v_uv);
    vec2 ddy = dFdy(v_uv);
    vec2 footprint = sqrt(ddx * ddx + ddy * ddy);

    float alpha = 0.0;
    for (uint i = v_path.x; i < v_path.y; i++) {
        vec3 t1 = texelFetch(curves, ivec2(2u * i, 0), 0).xyz;
        vec3 t2 = texelFetch(curves, ivec2(2u * i + 1u, 0), 0).xyz;
        vec2 p0 = t1.xy;
        vec2 p1 = vec2(t1.z, t2.x);
        vec2 p2 = t2.yz;

        vec2 y_footprint = v_uv.y + vec2(-0.5 * footprint.y, 0.5 * footprint.y);
        vec2 y_window = clamp(vec2(p2.y, p0.y), y_footprint.x, y_footprint.y);
        float y_overlap = (y_window.y - y_window.x) / footprint.y;

        if (y_overlap != 0.0 && max(p0.x, p2.x) > v_uv.x - 0.5 * footprint.x) {
            float a = p0.y - 2.0 * p1.y + p2.y;
            float b = p1.y - p0.y;
            float c = p0.y - 0.5 * (y_window.x + y_window.y);
            float q = -(b + (b < 0.0 ? -1.0 : 1.0) * sqrt(max(b * b - a * c, 0.0)));
            float ta = q / a;
            float tb = c / q;
            float t = (0.0 <= ta && ta <= 1.0) ? ta : tb;
            float x = mix(mix(p0.x, p1.x, t), mix(p1.x, p2.x, t), t);

            vec2 tangent = mix(p1 - p0, p2 - p1, t);
            float f = ((x - v_uv.x) * abs(tangent.y)) / length(footprint * tangent.yx);
            float x_overlap = clamp(0.5 + f, 0.0, 1.0);

            alpha += x_overlap * y_overlap;
        }
    }

    float brightness = (v_col.r + v_col.g + v_col.b) / (3.0 * v_col.a);
    alpha = clamp(abs(alpha), 0.0, 1.0);
    f_col = mix(1.0 - (1.0 - alpha) * (1.0 - alpha), alpha * alpha, sqrt(brightness)) * v_col;
}
