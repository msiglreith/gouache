#version 330

uniform sampler2D paths;

in vec4 v_col;
in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    vec2 ddx = dFdx(v_uv);
    vec2 ddy = dFdy(v_uv);
    vec2 footprint = sqrt(ddx * ddx + ddy * ddy);

    vec2 p0 = vec2(0.0, 0.0);
    float alpha = 0.0;
    for (uint i = v_path.x; i < v_path.y; ) {
        vec4 segment = texelFetch(paths, ivec2(int(i), 0), 0);
        vec2 p1 = segment.xy;
        vec2 p2 = segment.zw;

        if (p1 == vec2(0.0, 0.0) && p2.y > 0.5) {
            i += uint(p2.x * 65536.0);
        } else {
            i += 1u;
        }

        vec2 y_footprint = v_uv.y + vec2(-0.5 * footprint.y, 0.5 * footprint.y);
        vec2 y_window = clamp(vec2(p2.y, p0.y), y_footprint.x, y_footprint.y);
        float y_overlap = (y_window.y - y_window.x) / footprint.y;

        if (p1.x != 0.0 && y_overlap != 0.0 && max(p0.x, p2.x) > v_uv.x - 0.5 * footprint.x) {
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

        p0 = p2;
    }

    float brightness = (v_col.r + v_col.g + v_col.b) / (3.0 * v_col.a);
    alpha = clamp(abs(alpha), 0.0, 1.0);
    f_col = mix(1.0 - (1.0 - alpha) * (1.0 - alpha), alpha * alpha, sqrt(brightness)) * v_col;
}
