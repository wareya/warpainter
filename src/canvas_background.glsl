

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

vec3 coord_to_poly_sdf(vec2 c, float width)
{
    float len = textureSize(user_texture_1, 0).x;
    float closest = 10000000.0 / zoom_level;
    vec2 a = texture(user_texture_1, vec2(0.0, 0.0)).xy;
    
    bool inside = false;
    float total_length = 0.0;
    float progress = 0.0;
    
    for(int i = 0; i < len; i += 1)
    {
        float tex_x = (float((i+1) % int(len)) + 0.5) / float(len);
        vec2 b = texture(user_texture_1, vec2(tex_x, 0.0)).xy;
        vec2 u = b - a;
        vec2 v = a - c;
        float len = length(u);
        
        float t = -(dot(v, u)/dot(u, u));
        if (t > 0.0 && t < 1.0)
        {
            float new = dist_sq(mix(a, b, t), c);
            if (new < closest)
            {
                closest = new;
                progress = total_length + t*len;
            }
        }
        closest = min(closest, dist_sq(a, c));
        
        if ((a.y > c.y) != (b.y > c.y))
        {
            vec2 cb = c - b;
            vec2 ab = a - b;
            float s = cb.x * ab.y - cb.y * ab.x;
            inside = inside != ((s < 0.0) == (ab.y < 0.0));
        }
        
        total_length += len;
        
        a = b;
    }
    
    return vec3((sqrt(closest)) * zoom_level - width, float(inside), progress * zoom_level);
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

float soft_square(float x, float hardness)
{
    return clamp(((1.0 - abs(mod(x/2.0, 1.0) * 2.0 - 1.0)) - 0.5) * hardness * 2.0, -1.0, 1.0) * 0.5 + 0.5;
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
        
        // sdf-related stuff
        vec2 sv = coord_to_sdf(texcoord, grid_size, 1.0);
        float s = min(sv.x, sv.y);
        vec3 info = coord_to_poly_sdf(texcoord, 1.0);
        float n = info.x;
        s = min(s, n);
        
        // outline pattern for grid
        float a = soft_square(texcoord.x * zoom_level / 3.0, 3.0);
        float b = soft_square(texcoord.y * zoom_level / 3.0, 3.0);
        float outline_checker = (sv.x > sv.y) ? a : b;
        
        // switch with selection outline if needed
        if (s == n)
            outline_checker = soft_square(info.z/4.0, 4.0);
        
        // don't draw grid on edge of image
        s *= clamp((              raw_texcoord.x) * zoom_level - 0.5, 0.0, 1.0);
        s *= clamp((canvas_width -raw_texcoord.x) * zoom_level - 0.5, 0.0, 1.0);
        s *= clamp((              raw_texcoord.y) * zoom_level - 0.5, 0.0, 1.0);
        s *= clamp((canvas_height-raw_texcoord.y) * zoom_level - 0.5, 0.0, 1.0);
        
        float grid_strength = clamp(-s, 0.0, 1.0);
        
        if (n != s)
        {
            if (abs(zoom_level-1.0) < 0.01)
                grid_strength = round(grid_strength);
            grid_strength *= 0.5;
        }
        
        vec3 grid_color = mix(vec3(0.0), vec3(1.0), outline_checker);
        vec4 grid = vec4(grid_color, grid_strength);
        
        if(info.y > 0.5)
            out_color = mix_normal(vec4(0.0, 0.45, 0.85, 0.2), out_color);
        
        out_color = mix_normal(grid, out_color);
    }
    
    //out_color = to_linear(out_color);
}