#version 140
in vec2 position;
out vec4 out_color;

uniform sampler2D user_texture;

uniform float width;
uniform float height;

uniform float canvas_width;
uniform float canvas_height;

uniform float mat_0_0;
uniform float mat_0_1;
uniform float mat_1_0;
uniform float mat_1_1;
uniform float mat_2_0;
uniform float mat_2_1;


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
float det_of_line(vec2 a, vec2 b, vec2 point)
{
    return (point.x-a.x)*(b.y-a.y) - (point.y-a.y)*(b.x-a.x);
}
void main()
{
    mat3 xform = transpose(mat3 (
        vec3(mat_0_0, mat_1_0, mat_2_0),
        vec3(mat_0_1, mat_1_1, mat_2_1),
        vec3(0.0, 0.0, 1.0)
    ));
    mat3 xform_inv = inverse(xform);
    
    float x1 = -canvas_width /2.0;
    float y1 = -canvas_height/2.0;
    float x2 =  canvas_width /2.0;
    float y2 =  canvas_height/2.0;
    
    vec2 point_0 = (xform * vec3(x1, y1, 1.0)).xy;
    vec2 point_1 = (xform * vec3(x2, y1, 1.0)).xy;
    vec2 point_2 = (xform * vec3(x1, y2, 1.0)).xy;
    vec2 point_3 = (xform * vec3(x2, y2, 1.0)).xy;
    
    float minima_x = min(min(point_0.x, point_1.x), min(point_2.x, point_3.x));
    float minima_y = min(min(point_0.y, point_1.y), min(point_2.y, point_3.y));
    
    float x = position.x * width;
    float y = position.y * height;
    
    float x_checker = floor((x - minima_x) / 8.0);
    float y_checker = floor((y - minima_y) / 8.0);
    float checker = mod(x_checker + y_checker, 2.0);
    
    vec3 color = mix(vec3(0.8), vec3(1.0), checker);
    
    vec2 point = vec2(x, y);
    
    float det = max (
        max(det_of_line(point_0, point_1, point), det_of_line(point_1, point_3, point)),
        max(det_of_line(point_3, point_2, point), det_of_line(point_2, point_0, point))
    );
    
    vec2 texcoord = vec2(x, y);
    texcoord = (xform_inv * vec3(texcoord, 1.0)).xy;
    texcoord /= vec2(canvas_width, canvas_height);
    texcoord += vec2(0.5);
    vec4 c = texture2D(user_texture, texcoord);
    
    if (det <= 0.0)
    {
        out_color = vec4(color, 1.0);
        out_color = mix_normal(c, out_color);
        out_color = to_linear(out_color);
    }
    else
        out_color = vec4(0.0);
    
}