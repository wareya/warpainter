use eframe::egui_glow::glow;
use glow::HasContext;
use crate::warimage::*;

pub (crate) struct ShaderQuad
{
    program : glow::Program,
    vertex_array : glow::VertexArray,
    vertex_buffer : glow::Buffer,
    vertices : Vec<f32>,
    uvs : Vec<f32>,
    need_to_delete : bool,
    texture_handle : [Option<glow::Texture>; 8],
}

const VERT_SHADER : &'static str = "
    in vec2 in_vertex;
    in vec2 in_uv;
    
    out vec2 vertex;
    out vec2 uv;
    
    void main()
    {
        gl_Position = vec4(in_vertex * vec2(1.0, -1.0), 0.0, 1.0);
        vertex = in_vertex * vec2(0.5) + vec2(0.5);
        uv = in_uv;
    }
";
const FRAG_SHADER : &'static str = "
    in vec2 vertex;
    in vec2 uv;
    
    out vec4 out_color;
    
    void main()
    {
        float r = 1.0-uv.x;
        float g = 1.0-uv.y;
        float b = min(uv.y, uv.x);
        out_color = vec4(r, g, b, 1.0);
    }
";

fn upload_texture(gl : &glow::Context, handle : glow::Texture, texture : &Image)
{
    unsafe
    {
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        
        let bytes = texture.bytes();
        
        let internal_type = if texture.is_float() { glow::RGBA16F } else { glow::RGBA8 } as i32;
        let input_type = if texture.is_float() { glow::FLOAT } else { glow::UNSIGNED_BYTE };
        
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d (
            glow::TEXTURE_2D,
            0, // target
            internal_type,
            texture.width as i32,
            texture.height as i32,
            0, // border
            glow::RGBA,
            input_type,
            Some(bytes)
        );
        gl.generate_mipmap(glow::TEXTURE_2D);
    }
}
fn upload_data(gl : &glow::Context, handle : glow::Texture, data : &[[f32; 4]])
{
    unsafe
    {
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        
        let bytes = data.iter().map(|x| x.iter().map(|x| x.to_le_bytes()).flatten()).flatten().collect::<Vec<_>>();
        
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_image_2d (
            glow::TEXTURE_2D,
            0, // target
            glow::RGBA16F as i32,
            data.len() as i32, // width
            1, // height
            0, // border
            glow::RGBA,
            glow::FLOAT,
            Some(&bytes)
        );
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[allow(unsafe_code)]
impl ShaderQuad
{
    pub (crate) fn new(gl : &glow::Context, shader : Option<impl ToString>) -> Option<ShaderQuad>
    {
        unsafe
        {
            let vertex_array = gl.create_vertex_array().ok()?;
            gl.bind_vertex_array(Some(vertex_array));
            
            let vertex_buffer = gl.create_buffer().ok()?;

            let program = gl.create_program().ok()?;

            let mut vertex_shader = VERT_SHADER.to_string();
            let mut fragment_shader = shader.map(|x| x.to_string()).unwrap_or_else(|| FRAG_SHADER.to_string());
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                vertex_shader   = ("#version 140".to_string() + &vertex_shader).to_string();
                fragment_shader = ("#version 140".to_string() + &fragment_shader).to_string();
            }
            #[cfg(target_arch = "wasm32")]
            {
                vertex_shader   = ("#version 300 es".to_string() + &vertex_shader).to_string();
                fragment_shader = ("#version 300 es".to_string() + &fragment_shader).to_string();
                
                vertex_shader = vertex_shader
                    .replace(" float ", " highp float ")
                    .replace(" vec2 " , " highp vec2 ")
                    .replace(" vec3 " , " highp vec3 ")
                    .replace(" vec4 " , " highp vec4 ")
                    .replace("(float ", "(highp float ")
                    .replace("(vec2 ",  "(highp vec2 ")
                    .replace("(vec3 ",  "(highp vec3 ")
                    .replace("(vec4 ",  "(highp vec4 ")
                    .replace("\nfloat ", "\nhighp float ")
                    .replace("\nvec2 " , "\nhighp vec2 ")
                    .replace("\nvec3 " , "\nhighp vec3 ")
                    .replace("\nvec4 " , "\nhighp vec4 ")
                    ;
                fragment_shader = fragment_shader
                    .replace(" float ", " highp float ")
                    .replace(" vec2 ",  " highp vec2 ")
                    .replace(" vec3 ",  " highp vec3 ")
                    .replace(" vec4 ",  " highp vec4 ")
                    .replace("(float ", "(highp float ")
                    .replace("(vec2 ",  "(highp vec2 ")
                    .replace("(vec3 ",  "(highp vec3 ")
                    .replace("(vec4 ",  "(highp vec4 ")
                    .replace("\nfloat ", "\nhighp float ")
                    .replace("\nvec2 " , "\nhighp vec2 ")
                    .replace("\nvec3 " , "\nhighp vec3 ")
                    .replace("\nvec4 " , "\nhighp vec4 ")
                    ;
                
                log(&vertex_shader);
                log(&fragment_shader);
            }

            let mut shaders = vec!();
            let mut build = |shader_type, source|
            {
                let shader = gl.create_shader(shader_type).ok()?;
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader)
                {
                    let err = format!("shader compilation failed:\n{}", gl.get_shader_info_log(shader));
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        eprintln!("{}", err);
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        log(&err);
                    }
                    return None;
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
                Some(())
            };
            build(glow::VERTEX_SHADER, &vertex_shader)?;
            build(glow::FRAGMENT_SHADER, &fragment_shader)?;

            gl.link_program(program);
            if !gl.get_program_link_status(program)
            {
                let err = format!("program linking failed:\n{}", gl.get_program_info_log(program));
                
                #[cfg(not(target_arch = "wasm32"))]
                {
                    eprintln!("{}", err);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    log(&err);
                }
                return None;
            }

            for shader in shaders
            {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }
            
            let vertices = vec!(
                -1.0, -1.0,
                 1.0, -1.0,
                -1.0,  1.0,
                 1.0,  1.0
            );
            let uvs = vec!(
                0.0, 0.0,
                1.0, 0.0,
                0.0, 1.0,
                1.0, 1.0
            );

            Some(ShaderQuad { program, vertex_array, vertex_buffer, vertices, uvs, need_to_delete : true, texture_handle : [None; 8] } )
        }
    }
    pub (crate) fn add_texture(&mut self, gl : &glow::Context, texture : &Image, which : usize)
    {
        unsafe
        {
            eframe::egui_glow::check_for_gl_error!(gl, "before texture upload");
            let which = which.clamp(0, 7);
            gl.active_texture(glow::TEXTURE0 + which as u32);
            if self.texture_handle[which].is_none()
            {
                self.texture_handle[which] = gl.create_texture().ok();
            }
            let handle = self.texture_handle[which].unwrap();
            upload_texture(gl, handle, &texture);
            eframe::egui_glow::check_for_gl_error!(gl, "after texture upload");
        }
    }
    pub (crate) fn add_data(&mut self, gl : &glow::Context, data : &[[f32; 4]], which : usize)
    {
        unsafe
        {
            eframe::egui_glow::check_for_gl_error!(gl, "before texture upload");
            let which = which.clamp(0, 7);
            gl.active_texture(glow::TEXTURE0 + which as u32);
            if self.texture_handle[which].is_none()
            {
                self.texture_handle[which] = gl.create_texture().ok();
            }
            let handle = self.texture_handle[which].unwrap();
            upload_data(gl, handle, &data);
            eframe::egui_glow::check_for_gl_error!(gl, "after texture upload");
        }
    }
    pub (crate) fn add_vertices(&mut self, verts : &[[f32; 2]], uvs : &[[f32; 2]])
    {
        assert!(verts.len() == uvs.len());
        self.vertices = vec!(0.0; verts.len()*2);
        for (i, vert) in verts.iter().enumerate()
        {
            self.vertices[i*2 + 0] = vert[0];
            self.vertices[i*2 + 1] = vert[1];
        }
        
        self.uvs = vec!(0.0; uvs.len()*2);
        for (i, uv) in uvs.iter().enumerate()
        {
            self.uvs[i*2 + 0] = uv[0];
            self.uvs[i*2 + 1] = uv[1];
        }
    }
    pub (crate) fn render(&self, gl : &glow::Context, uniforms : &[(impl ToString, f32)])
    {
        unsafe
        {
            eframe::egui_glow::check_for_gl_error!(gl, "before render");
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vertex_array));
            
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer));
            
            use byte_slice_cast::*;
            let verts = self.vertices.as_byte_slice();
            let uvs = self.uvs.as_byte_slice();
            let mut bytes = vec!();
            bytes.extend(verts);
            bytes.extend(uvs);
            
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::DYNAMIC_DRAW);
            
            let attrib_location = gl.get_attrib_location(self.program, "in_vertex").unwrap();
            gl.vertex_attrib_pointer_f32(attrib_location, 2, glow::FLOAT, false, 2 * std::mem::size_of::<f32>() as i32, 0);
            gl.enable_vertex_attrib_array(attrib_location);
            
            let attrib_location = gl.get_attrib_location(self.program, "in_uv").unwrap();
            gl.vertex_attrib_pointer_f32(attrib_location, 2, glow::FLOAT, false, 2 * std::mem::size_of::<f32>() as i32, verts.len() as i32);
            gl.enable_vertex_attrib_array(attrib_location);
            
            for uniform in uniforms
            {
                let location = gl.get_uniform_location(self.program, uniform.0.to_string().as_str());
                gl.uniform_1_f32(location.as_ref(), uniform.1);
            }
            
            for (i, handle) in self.texture_handle.iter().enumerate()
            {
                if let Some(handle) = handle
                {
                    gl.uniform_1_i32(gl.get_uniform_location(self.program, &format!("user_texture_{}", i)).as_ref(), i as i32);
                    gl.active_texture(glow::TEXTURE0 + i as u32);
                    gl.bind_texture(glow::TEXTURE_2D, Some(*handle));
                }
            }
            
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            eframe::egui_glow::check_for_gl_error!(gl, "after render");
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