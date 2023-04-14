#version 140

in vec2 vertex;
out vec4 out_color;

uniform float width;
uniform float height;
uniform float hue;

uniform float ring_size;
uniform float box_size;

#define PI 3.1415926538

vec4 to_srgb(vec4 linear)
{
    bvec4 cutoff = lessThan(linear, vec4(0.0031308));
    vec4 higher = vec4(1.055)*pow(linear, vec4(1.0/2.4)) - vec4(0.055);
    vec4 lower = linear * vec4(12.92);

    return vec4(mix(higher, lower, cutoff).rgb, linear.a);
}
vec4 to_linear(vec4 srgb)
{
    bvec4 cutoff = lessThan(srgb, vec4(0.04045));
    vec4 higher = pow((srgb + vec4(0.055))/vec4(1.055), vec4(2.4));
    vec4 lower = srgb/vec4(12.92);

    return vec4(mix(higher, lower, cutoff).rgb, srgb.a);
}

vec4 rgb_to_hsv(vec4 rgba)
{
    float v = max(max(rgba.r, rgba.g), rgba.b);
    float c = v - min(min(rgba.r, rgba.g), rgba.b);
    float s = v > 0.0 ? c / v : 0.0 ;
    float h = 0.0;
    if (c == 0.0)
    {
        h = 0.0;
    }
    else if (v == rgba.r)
    {
        h = 60.0 * (rgba.g - rgba.b)/c;
    }
    else if (v == rgba.g)
    {
        h = 60.0 * (rgba.b - rgba.r)/c + 120.0;
    }
    else
    {
        h = 60.0 * (rgba.r - rgba.g)/c + 240.0;
    }
    return vec4(h, s, v, rgba.a);
}
vec4 hsv_to_rgb(vec4 hsva)
{
    float c = hsva.z * hsva.y;
    float h2 = hsva.x / 60.0;
    float x = c * (1.0 - abs(mod(h2, 2.0) - 1.0));
    
    vec3 table[6] = vec3[6] (
        vec3(c, x, 0.0),
        vec3(x, c, 0.0),
        vec3(0.0, c, x),
        vec3(0.0, x, c),
        vec3(x, 0.0, c),
        vec3(c, 0.0, x)
    );
    vec3 triad = table[int(mod(h2, 6.0))];
    
    float m = hsva.z - c;
    return vec4(triad + vec3(m), hsva.a);
}
void main()
{
    float least_f = min(width, height);
    float box_margin = (least_f - box_size)/2.0;
    float x = vertex.x * width;
    float y = vertex.y * height;
    
    vec2 mid_diff = vertex*2.0 - vec2(1.0);
    float mid_dist = length(mid_diff);
    
    if (mid_dist + ring_size > 1.0 && mid_dist < 1.0)
    {
        float h = mod(degrees(atan(mid_diff.y, mid_diff.x)) + 360.0 + 150.0, 360.0);
        
        //distance, outline-rendering stuff
        float p = min(abs(1.0 - mid_dist), abs(1.0 - ring_size - mid_dist))*2.0;
        float a = clamp(p*least_f*ring_size/1.2, 0.0, 1.0);
        float b = clamp(p*least_f*ring_size/1.2 - 0.5, 0.0, 1.0);
        
        //out_color = to_linear(hsv_to_rgb(vec4(h, 0.9, b, a)));
        out_color = hsv_to_rgb(vec4(h, 0.9, b, a));
    }
    else if (x > box_margin && x < box_margin+box_size
          && y > box_margin && y < box_margin+box_size)
    {
        float s = (x-box_margin) / box_size;
        float v = 1.0 - (y-box_margin) / box_size;
        //out_color = to_linear(hsv_to_rgb(vec4(hue, s, v, 1.0)));
        out_color = hsv_to_rgb(vec4(hue, s, v, 1.0));
    }
    else if (x > box_margin-1.0 && x < box_margin+box_size+1.0
          && y > box_margin-1.0 && y < box_margin+box_size+1.0)
    {
        //out_color = to_linear(hsv_to_rgb(vec4(0.0, 0.0, 0.0, 0.75)));
        out_color = hsv_to_rgb(vec4(0.0, 0.0, 0.0, 0.75));
    }
    else if (x > box_margin-2.0 && x < box_margin+box_size+2.0
          && y > box_margin-2.0 && y < box_margin+box_size+2.0)
    {
        //out_color = to_linear(hsv_to_rgb(vec4(0.0, 0.0, 0.0, 0.25)));
        out_color = hsv_to_rgb(vec4(0.0, 0.0, 0.0, 0.25));
    }
}