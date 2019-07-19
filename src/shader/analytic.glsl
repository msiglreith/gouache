#version 330

uniform sampler2D curves;

in vec2 v_uv;
flat in uvec2 v_path;

out vec4 f_col;

void main() {
    vec2 footprint = vec2(length(vec2(dFdx(v_uv.x), dFdy(v_uv.x))), length(vec2(dFdx(v_uv.y), dFdy(v_uv.y))));

    float alpha = 0.0;
    for (uint i = v_path.x; i < v_path.y; i++) {
        /* fetch control points */
        vec3 t1 = texelFetch(curves, ivec2(2u * i, 0), 0).xyz;
        vec3 t2 = texelFetch(curves, ivec2(2u * i + 1u, 0), 0).xyz;
        vec2 p0 = t1.xy;
        vec2 p1 = vec2(t1.z, t2.x);
        vec2 p2 = t2.yz;

        /* implicitize bezier curve */
        vec2 p = v_uv - p0;
        vec2 a = p0 - 2.0 * p1 + p2;
        vec2 b = 2.0 * p1 - 2.0 * p0;
        float x = a.x * p.y - a.y * p.x;
        float y = b.y * p.x - b.x * p.y;
        float det = a.x * b.y - b.x * a.y;
        float f = -sign(p2.y - p0.y) * sign(x) * (x * x - det * y);

        /* normalize to distance function */
        vec2 grad = vec2(-2.0 * x * a.y - det * b.y, 2.0 * x * a.x + det * b.x);
        float mag = length(grad * footprint);
        if (mag != 0.0) { f /= mag; }

        vec2 x_window = v_uv.x + vec2(-0.5 * footprint.x, 0.5 * footprint.x);
        float left = min(p0.x, p2.x);
        float x_overlap = abs(clamp(p2.x, x_window.x, x_window.y) - clamp(p0.x, x_window.x, x_window.y)) / footprint.x;
        float x_inside  = abs(min(left, x_window.x) - min(left, x_window.y)) / footprint.x;
        vec2 y_window = v_uv.y + vec2(-0.5 * footprint.y, 0.5 * footprint.y);
        float y_overlap = (clamp(p2.y, y_window.x, y_window.y) - clamp(p0.y, y_window.x, y_window.y)) / footprint.y;

        alpha += y_overlap * (x_inside + x_overlap * smoothstep(0.0, 1.0, 0.5 + 0.8862269 * f));
    }

    f_col = vec4(abs(alpha));
}
