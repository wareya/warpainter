

in vec2 vertex;
out vec4 out_color;

uniform float width;
uniform float height;
uniform float funvalue;
uniform float glsl_mode;

uniform float dat_0;
uniform float dat_1;
uniform float dat_2;
uniform float dat_3;

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
    
    vec3 table[6];
    table[0] = vec3(c, x, 0.0);
    table[1] = vec3(x, c, 0.0);
    table[2] = vec3(0.0, c, x);
    table[3] = vec3(0.0, x, c);
    table[4] = vec3(x, 0.0, c);
    table[5] = vec3(c, 0.0, x);
    
    vec3 triad = table[int(mod(h2, 6.0))];
    
    float m = hsva.z - c;
    return vec4(triad + vec3(m), hsva.a);
}
void main()
{
    float x = vertex.x * width;
    float y = vertex.y * height;
    
    
    //if (glsl_mode == 1.0)
    {
        float grid = mod(floor(x/5.0) + floor(y/5.0), 2.0);
        out_color = vec4(mix(vec3(grid * 0.25 + 0.75), vec3(dat_0, dat_1, dat_2), vertex.x), 1.0);
    }
}