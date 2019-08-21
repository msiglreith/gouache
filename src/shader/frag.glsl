#version 330

uniform sampler2D curves;

in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    vec2 footprint = vec2(length(vec2(dFdx(v_uv.x), dFdy(v_uv.x))), length(vec2(dFdx(v_uv.y), dFdy(v_uv.y))));

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
            float dxdy = tangent.x / tangent.y;
            float x_overlap = clamp((0.5 * footprint.x + (x - v_uv.x) / max(1.0, abs(dxdy) * (footprint.y / footprint.x))) / footprint.x, 0.0, 1.0);

            alpha += x_overlap * y_overlap;
        }
    }

    float brightness = 0.0;
    alpha = clamp(abs(alpha), 0.0, 1.0);
    f_col = mix((1.0 - (1.0 - alpha) * (1.0 - alpha)), alpha * alpha, sqrt(brightness)) * vec4(brightness, brightness, brightness, 1.0);
}
