#version 140
in vec2 position;
out vec4 out_color;

uniform float width;
uniform float height;

uniform float point_0_x;
uniform float point_0_y;
uniform float point_1_x;
uniform float point_1_y;
uniform float point_2_x;
uniform float point_2_y;
uniform float point_3_x;
uniform float point_3_y;

vec4 to_linear(vec4 srgb)
{
    bvec4 cutoff = lessThan(srgb, vec4(0.04045));
    vec4 higher = pow((srgb + vec4(0.055))/vec4(1.055), vec4(2.4));
    vec4 lower = srgb/vec4(12.92);

    return vec4(mix(higher, lower, cutoff).rgb, srgb.a);
}
float det_of_line(vec2 a, vec2 b, vec2 point)
{
    return (point.x-a.x)*(b.y-a.y) - (point.y-a.y)*(b.x-a.x);
}
void main()
{
    float minima_x = min(min(point_0_x, point_1_x), min(point_2_x, point_3_x));
    float minima_y = min(min(point_0_y, point_1_y), min(point_2_y, point_3_y));
    
    float x = position.x * width;
    float y = position.y * height;
    
    float x_checker = floor((x - minima_x) / 8.0);
    float y_checker = floor((y - minima_y) / 8.0);
    float checker = mod(x_checker + y_checker, 2.0);
    
    vec3 color = mix(vec3(0.8), vec3(1.0), checker);
    
    vec2 point_0 = vec2(point_0_x, point_0_y);
    vec2 point_1 = vec2(point_1_x, point_1_y);
    vec2 point_2 = vec2(point_2_x, point_2_y);
    vec2 point_3 = vec2(point_3_x, point_3_y);
    
    vec2 point = vec2(x, y);
    
    float det = max (
        max(det_of_line(point_0, point_1, point), det_of_line(point_1, point_3, point)),
        max(det_of_line(point_3, point_2, point), det_of_line(point_2, point_0, point))
    );
    
    if (det <= 0.0)
        out_color = to_linear(vec4(color, 1.0));
    else
        out_color = vec4(0.0);
}