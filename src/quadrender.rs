use alloc::sync::Arc;
use egui::mutex::Mutex;

use eframe::egui;
use eframe::egui_glow::glow;
use glow::HasContext;

pub (crate) struct ShaderQuad
{
    program : glow::Program,
    vertex_array : glow::VertexArray,
    need_to_delete : bool,
}

const VERT_SHADER : &'static str = "
    #version 140
    const vec2 verts[4] = vec2[4] (
        vec2(-1.0, -1.0),
        vec2(-1.0,  1.0),
        vec2( 1.0, -1.0),
        vec2( 1.0,  1.0)
    );
    out vec2 position;
    void main()
    {
        gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
        position = verts[gl_VertexID] * vec2(0.5, -0.5) + vec2(0.5, 0.5);
    }
";
const FRAG_SHADER : &'static str = "
    #version 140
    in vec2 position;
    out vec4 out_color;
    void main()
    {
        float r = 1.0-position.x;
        float g = 1.0-position.y;
        float b = min(position.y, position.x);
        out_color = vec4(r, g, b, 1.0);
    }
";

#[allow(unsafe_code)]
impl ShaderQuad
{
    pub (crate) fn new(gl : &glow::Context, shader : Option<impl ToString>) -> Option<ShaderQuad>
    {
        unsafe
        {
            // FIXME is this safe? not adding any data?
            let vertex_array = gl.create_vertex_array().ok()?;
            
            let program = gl.create_program().ok()?;

            let vertex_shader = VERT_SHADER;
            let fragment_shader = shader.map(|x| x.to_string()).unwrap_or_else(|| FRAG_SHADER.to_string());

            let mut shaders = vec!();
            let mut build = |shader_type, source|
            {
                let shader = gl.create_shader(shader_type).ok()?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader)
                {
                    eprintln!("shader compilation failed:\n{}", gl.get_shader_info_log(shader));
                    return None;
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
                Some(())
            };
            build(glow::VERTEX_SHADER, vertex_shader)?;
            build(glow::FRAGMENT_SHADER, &fragment_shader)?;

            gl.link_program(program);
            if !gl.get_program_link_status(program)
            {
                eprintln!("program linking failed:\n{}", gl.get_program_info_log(program));
                return None;
            }

            for shader in shaders
            {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            Some(ShaderQuad { program, vertex_array, need_to_delete : true } )
        }
    }
    pub (crate) fn render(&self, gl : &glow::Context, uniforms : &[(impl ToString, f32)])
    {
        unsafe
        {
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vertex_array));
            for uniform in uniforms
            {
                let location = gl.get_uniform_location(self.program, uniform.0.to_string().as_str());
                gl.uniform_1_f32(location.as_ref(), uniform.1);
            }
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
    }
    
    pub (crate) fn delete_data(&mut self, gl : &glow::Context)
    {
        if self.need_to_delete
        {
            self.need_to_delete = false;
            unsafe
            {
                gl.delete_program(self.program);
                gl.delete_vertex_array(self.vertex_array);
            }
        }
    }
}