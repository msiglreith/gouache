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
    vec2 y_footprint = v_uv.y + vec2(-0.5 * footprint.y, 0.5 * footprint.y);

    uvec2 lane = 2u * uvec2(65536.0 * texelFetch(paths, ivec2(int(v_path.x + 16u * clamp(uint(v_uv.x * 2.0), 0, 1) + clamp(uint(y_footprint.x * 16.0), 0, 15)), 0), 0).xy);
    float flip = v_uv.x < 0.5 ? -1.0 : 1.0;

    float alpha = 0.0;
    vec3 t1 = texelFetch(paths, ivec2(int(v_path.x + 32u + lane.x), 0), 0).xyz;
    vec3 t2 = texelFetch(paths, ivec2(int(v_path.x + 33u + lane.x), 0), 0).xyz;
    for (uint i = v_path.x + 32u + lane.x; i < v_path.x + 32u + lane.y; i += 2u) {
        vec2 p1 = t1.xy;
        vec2 p2 = vec2(t1.z, t2.x);
        vec2 p3 = t2.yz;

        vec2 y_window = clamp(vec2(p3.y, p1.y), y_footprint.x, y_footprint.y);
        float y_overlap = (y_window.y - y_window.x) / footprint.y;

        if (min(p1.y, p3.y) > y_footprint.y) { break; }

        t1 = texelFetch(paths, ivec2(int(i + 2u), 0), 0).xyz;
        t2 = texelFetch(paths, ivec2(int(i + 3u), 0), 0).xyz;

        if (y_overlap != 0.0) {
            float a = p1.y - 2.0 * p2.y + p3.y;
            float b = p2.y - p1.y;
            float c = p1.y - 0.5 * (y_window.x + y_window.y);
            float q = -(b + (b < 0.0 ? -1.0 : 1.0) * sqrt(max(b * b - a * c, 0.0)));
            float ta = q / a;
            float tb = c / q;
            float t = (0.0 <= ta && ta <= 1.0) ? ta : tb;
            float x = mix(mix(p1.x, p2.x, t), mix(p2.x, p3.x, t), t);

            vec2 tangent = mix(p2 - p1, p3 - p2, t);
            float f = ((x - v_uv.x) * abs(tangent.y)) / length(footprint * tangent.yx);
            float x_overlap = clamp(0.5 + flip * f, 0.0, 1.0);

            alpha += x_overlap * y_overlap;
        }
    }

    float brightness = (v_col.r + v_col.g + v_col.b) / (3.0 * v_col.a);
    alpha = clamp(abs(alpha), 0.0, 1.0);
    f_col = mix(1.0 - (1.0 - alpha) * (1.0 - alpha), alpha * alpha, sqrt(brightness)) * v_col;
}
