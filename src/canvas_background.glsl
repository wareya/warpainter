

in vec2 vertex;
in vec2 uv;
out vec4 out_color;

uniform sampler2D user_texture_0;
uniform sampler2D user_texture_1;

uniform float width;
uniform float height;

uniform float canvas_width;
uniform float canvas_height;

uniform float minima_x;
uniform float minima_y;

uniform float zoom_level;

float dist_sq(vec2 a, vec2 b)
{
    vec2 d = a - b;
    return dot(d, d);
}

float coord_to_grand_sdf(vec2 c, float width)
{
    float len = textureSize(user_texture_1, 0).x;
    float closest = 10000000.0 / zoom_level;
    vec2 a = texture(user_texture_1, vec2(0.0, 0.0)).xy;
    for(int i = 0; i < len; i += 1)
    {
        float x = (float((i+1) % int(len)) + 0.5) / float(len);
        vec2 b = texture(user_texture_1, vec2(x, 0.0)).xy;
        vec2 u = b - a;
        vec2 v = a - c;
        float t = -(dot(v, u)/dot(u, u));
        if (t > 0.0 && t < 1.0)
            closest = min(closest, dist_sq(mix(a, b, t), c));
        closest = min(closest, dist_sq(a, c));
        closest = min(closest, dist_sq(b, c));
        
        a = b;
    }
    return (sqrt(closest)) * zoom_level - width;
}

vec2 coord_to_sdf(vec2 c, float scale, float width)
{
    scale /= 2.0;
    float x = (1.0-abs(mod(c.x/scale, 2.0)-1.0)) * scale * zoom_level - width;
    float y = (1.0-abs(mod(c.y/scale, 2.0)-1.0)) * scale * zoom_level - width;
    return vec2(x, y);
}

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
    
    // render checkerboard background for canvas
    
    float x_checker = floor((x - minima_x) / 8.0);
    float y_checker = floor((y - minima_y) / 8.0);
    float checker = mod(x_checker + y_checker, 2.0);
    
    vec3 color = mix(vec3(0.8), vec3(1.0), checker);
    
    // render canvas image
    
    vec4 tex_color = texture(user_texture_0, uv);
    
    out_color = vec4(color, 1.0);
    out_color = mix_normal(tex_color, out_color);
    
    // render grid
    
    float grid_size = 16.0;
    if (grid_size*zoom_level > 11.5)
    {
        vec2 raw_texcoord = uv * vec2(canvas_width, canvas_height);
        vec2 texcoord = raw_texcoord;
        if (abs(zoom_level-1.0) < 0.01)
            texcoord = floor(texcoord);
        
        vec2 sv = coord_to_sdf(texcoord, grid_size, 1.0);
        float s = min(sv.x, sv.y);
        float n = coord_to_grand_sdf(texcoord, 1.0); // FIXME unfinished / not used properly
        s = min(s, n);
        
        float canvas_x_checker = floor(texcoord.x * zoom_level / 3.0 + 0.5);
        float canvas_y_checker = floor(texcoord.y * zoom_level / 3.0 + 0.5);
        float canvas_checker = mod((sv.x > sv.y) ? canvas_x_checker : canvas_y_checker, 2.0);
        
        // don't draw grid on edge of image
        s *= clamp((              raw_texcoord.x) * zoom_level - 0.5, 0.0, 1.0);
        s *= clamp((canvas_width -raw_texcoord.x) * zoom_level - 0.5, 0.0, 1.0);
        s *= clamp((              raw_texcoord.y) * zoom_level - 0.5, 0.0, 1.0);
        s *= clamp((canvas_height-raw_texcoord.y) * zoom_level - 0.5, 0.0, 1.0);
        
        float grid_strength = clamp(-s, 0.0, 1.0);
        
        if (abs(zoom_level-1.0) < 0.01)
            grid_strength = round(grid_strength);
        
        vec3 grid_color = mix(vec3(0.0), vec3(1.0), canvas_checker);
        vec4 grid = vec4(grid_color, grid_strength*0.5);
        
        out_color = mix_normal(grid, out_color);
        
        //out_color.rgb = vec3(-s/grid_size*0.5 + 0.5);
        //out_color.rgb = vec3(-n/grid_size*0.5 + 0.5);
    }
    
    //out_color = to_linear(out_color);
}