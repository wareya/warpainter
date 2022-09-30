#version 140
in vec2 vertex;
in vec2 uv;
out vec4 out_color;

uniform sampler2D user_texture;

uniform float width;
uniform float height;

uniform float canvas_width;
uniform float canvas_height;

uniform float minima_x;
uniform float minima_y;

vec4 mix_normal(vec4 a, vec4 b)
{
    vec4 ret = vec4(0.0);
    ret.rgb = a.rgb * a.a + b.rgb * b.a * (1.0 - a.a);
    ret.a = a.a + b.a*(1.0 - a.a);
    if (ret.a > 0.0)
    {
        ret.rgb /= ret.a;
    }
    return ret;
}

vec4 to_linear(vec4 srgb)
{
    bvec4 cutoff = lessThan(srgb, vec4(0.04045));
    vec4 higher = pow((srgb + vec4(0.055))/vec4(1.055), vec4(2.4));
    vec4 lower = srgb/vec4(12.92);

    return vec4(mix(higher, lower, cutoff).rgb, srgb.a);
}

void main()
{
    float x = (vertex.x-0.5) * width;
    float y = (vertex.y-0.5) * height;
    
    float x_checker = floor((x - minima_x) / 8.0);
    float y_checker = floor((y - minima_y) / 8.0);
    float checker = mod(x_checker + y_checker, 2.0);
    
    vec3 color = mix(vec3(0.8), vec3(1.0), checker);
    
    vec4 c = texture2D(user_texture, uv);
    
    out_color = vec4(color, 1.0);
    out_color = mix_normal(c, out_color);
    out_color = to_linear(out_color);
}