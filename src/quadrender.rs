use eframe::egui_glow::glow;
use glow::{HasContext, PixelUnpackData, RGBA8, UNSIGNED_BYTE};
use crate::hwaccel;
use crate::{px_lerp_biased_float, px_lerp_float, px_to_int, warimage::*};

pub (crate) struct ShaderQuad
{
    program : glow::Program,
    vertex_array : glow::VertexArray,
    vertex_buffer : glow::Buffer,
    vertices : Vec<f32>,
    uvs : Vec<f32>,
    need_to_delete : bool,
    texture_handle : [Option<glow::Texture>; 8],
    texture_sizes : [[i32; 2]; 8],
}

const VERT_SHADER : &str = "
    layout(location = 0) in vec2 in_vertex;
    layout(location = 1) in vec2 in_uv;
    
    out vec2 vertex;
    out vec2 uv;
    
    void main()
    {
        gl_Position = vec4(in_vertex * vec2(1.0, -1.0), 0.0, 1.0);
        vertex = in_vertex * vec2(0.5) + vec2(0.5);
        uv = in_uv;
    }
";
const FRAG_SHADER : &str = "
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


const MIP_FRAG_SHADER : &str = "
    in vec2 vertex;
    in vec2 uv;
    out vec4 out_color;
    
    uniform sampler2D user_texture_0;
    
    uniform float miplevel;
    
    uniform float prev_w;
    uniform float prev_h;
    uniform float w;
    uniform float h;
    
    void main()
    {
        vec2 uv2 = vec2(uv.x, 1.0 - uv.y);
        vec2 pxs = 0.5 / vec2(prev_w, prev_h);
        vec4 a = texture(user_texture_0, uv2 + vec2(+pxs.x, +pxs.y), miplevel - 1.0);
        vec4 b = texture(user_texture_0, uv2 + vec2(+pxs.x, -pxs.y), miplevel - 1.0);
        vec4 c = texture(user_texture_0, uv2 + vec2(-pxs.x, +pxs.y), miplevel - 1.0);
        vec4 d = texture(user_texture_0, uv2 + vec2(-pxs.x, -pxs.y), miplevel - 1.0);
        a.rgb *= a.a;
        b.rgb *= b.a;
        c.rgb *= c.a;
        d.rgb *= d.a;
        vec4 r = (a+b+c+d) * 0.25;
        r.rgb /= (r.a + 0.00001);
        out_color = r;
    }
";

use std::sync::Mutex;
use std::sync::OnceLock;
static MIPSHADER : OnceLock<Mutex<ShaderQuad>> = OnceLock::new();
fn get_mipshader(gl : &glow::Context) -> &'static Mutex<ShaderQuad>
{
    MIPSHADER.get_or_init(|| Mutex::new(ShaderQuad::new(gl, Some(MIP_FRAG_SHADER)).unwrap()))
    //MIPSHADER.get_or_init(|| Mutex::new(ShaderQuad::new(gl, None::<&str>).unwrap()))
}

pub (crate) fn fix_mipmaps(gl : &glow::Context, handle : glow::Texture, width : usize, height : usize)
{
    unsafe
    {
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        
        let mut state = hwaccel::OpenGLContextState::new();
        state.save_state(gl);
        // rerender mipmaps
        
        let (mut w, mut h) = (width, height);
        let mut i = 1;
        let mut prev_w = w;
        let mut prev_h = h;
        w = (w/2).max(1);
        h = (h/2).max(1);
        
        let mut _shader = get_mipshader(gl).lock().unwrap();
        let shader = &mut *_shader;
        
        loop
        {
            gl.use_program(Some(shader.program));
            shader.texture_handle[0] = Some(handle);
            gl.viewport(0, 0, w as i32, h as i32);
            
            gl.bind_texture(glow::TEXTURE_2D, Some(handle));
            gl.tex_image_2d(glow::TEXTURE_2D, i as i32, glow::RGBA8 as i32, w as i32, h as i32, 0, glow::RGBA, UNSIGNED_BYTE, PixelUnpackData::Slice(None));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_BASE_LEVEL, i - 1);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, i);
            
            let framebuffer = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            
            // Attach texture at the desired mip level
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(handle),
                i,
            );
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE
            {
                panic!("Framebuffer is not complete! {:?}", gl.check_framebuffer_status(glow::FRAMEBUFFER));
            }
            
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            shader.render(gl, &[("miplevel", i as f32), ("prev_w", prev_w as f32), ("prev_h", prev_h as f32), ("w", w as f32), ("h", h as f32)]);
            
            i += 1;
        
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(framebuffer);
            
            if w == 1 && h == 1
            {
                break;
            }
            
            prev_w = w;
            prev_h = h;
            
            h = (h/2).max(1);
            w = (w/2).max(1);
        }
        
        let maxlv = (width.max(height) as f32).log2().floor() as i32;
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_BASE_LEVEL, 0);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAX_LEVEL, maxlv);
        
        state.load_state(gl);
        
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    }
}

pub (crate) fn upload_texture(gl : &glow::Context, handle : glow::Texture, texture : &Image<4>)
{
    unsafe
    {
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        
        let bytes = texture.bytes();
        
        let internal_type = if texture.is_float() { glow::RGBA16F } else { glow::RGBA8 } as i32;
        let input_type = if texture.is_float() { glow::FLOAT } else { glow::UNSIGNED_BYTE };
        
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0, // target
            internal_type,
            texture.width as i32,
            texture.height as i32,
            0, // border
            glow::RGBA,
            input_type,
            glow::PixelUnpackData::Slice(Some(bytes))
        );
        
        let start = web_time::Instant::now();
        
        fix_mipmaps(gl, handle, texture.width, texture.height);
        println!("--ASDFASDFASDF {:.6}ms", start.elapsed().as_secs_f64() * 1000.0);
    }
}

pub (crate) fn update_texture(gl : &glow::Context, handle : glow::Texture, texture : &Image<4>, rect : [[f32; 2]; 2])
{
    unsafe
    {
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        
        let bytes = texture.bytes();
        
        let internal_type = if texture.is_float() { glow::RGBA16F } else { glow::RGBA8 } as i32;
        let input_type = if texture.is_float() { glow::FLOAT } else { glow::UNSIGNED_BYTE };
        
        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, texture.width as i32);

        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            rect[0][0] as i32,
            rect[0][1] as i32,
            (rect[1][0] - rect[0][0]) as i32,
            (rect[1][1] - rect[0][1]) as i32,
            glow::RGBA,
            input_type,
            PixelUnpackData::Slice(Some(&bytes[(rect[0][1] as u32 * texture.width as u32 + rect[0][0] as u32) as usize * 4..])),
        );

        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
        
        let start = web_time::Instant::now();
        
        fix_mipmaps(gl, handle, texture.width, texture.height);
        
        println!("--ASDFASDFASDF (rebuild) {:.6}ms", start.elapsed().as_secs_f64() * 1000.0);
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
        
        use byte_slice_cast::*;
        let bytes = data.as_byte_slice();
        
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
            glow::PixelUnpackData::Slice(Some(bytes))
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
                vertex_shader   = "#version 330".to_string() + &vertex_shader;
                fragment_shader = "#version 330".to_string() + &fragment_shader;
            }
            #[cfg(target_arch = "wasm32")]
            {
                let prefix = "#version 300 es\nprecision highp float;".to_string();
                vertex_shader   = prefix.clone() + &vertex_shader;
                fragment_shader = prefix.clone() + &fragment_shader;
                
                //log(&vertex_shader);
                //log(&fragment_shader);
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
                        eprintln!("{}", source);
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        log(&err);
                        log(&source);
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

            Some(ShaderQuad { program, vertex_array, vertex_buffer, vertices, uvs, need_to_delete : true, texture_handle : [None; 8], texture_sizes : [[0, 0]; 8] } )
        }
    }
    pub (crate) fn get_texture_size(&mut self, which : usize) -> [i32; 2]
    {
        let which = which.clamp(0, 7);
        self.texture_sizes[which]
    }
    pub (crate) fn use_texture(&mut self, gl : &glow::Context, which : usize) -> Option<glow::Texture>
    {
        unsafe
        {
            let which = which.clamp(0, 7);
            gl.active_texture(glow::TEXTURE0 + which as u32);
            self.texture_handle[which]
        }
    }
    pub (crate) fn add_texture(&mut self, gl : &glow::Context, texture : &Image<4>, which : usize)
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
            upload_texture(gl, handle, texture);
            self.texture_sizes[which] = [texture.width as i32, texture.height as i32];
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
            upload_data(gl, handle, data);
            eframe::egui_glow::check_for_gl_error!(gl, "after texture upload");
        }
    }
    pub (crate) fn add_vertices(&mut self, verts : &[[f32; 2]], uvs : &[[f32; 2]])
    {
        assert!(verts.len() == uvs.len());
        self.vertices = vec!(0.0; verts.len()*2);
        for (i, vert) in verts.iter().enumerate()
        {
            self.vertices[i*2    ] = vert[0];
            self.vertices[i*2 + 1] = vert[1];
        }
        
        self.uvs = vec!(0.0; uvs.len()*2);
        for (i, uv) in uvs.iter().enumerate()
        {
            self.uvs[i*2    ] = uv[0];
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
            
            eframe::egui_glow::check_for_gl_error!(gl, "mid quad render A");
            
            let attrib_location = 0;//gl.get_attrib_location(self.program, "in_vertex").unwrap();
            gl.vertex_attrib_pointer_f32(attrib_location, 2, glow::FLOAT, false, 2 * std::mem::size_of::<f32>() as i32, 0);
            gl.enable_vertex_attrib_array(attrib_location);
            
            eframe::egui_glow::check_for_gl_error!(gl, "mid quad render B");
            
            let attrib_location = 1;//gl.get_attrib_location(self.program, "in_uv").unwrap();
            gl.vertex_attrib_pointer_f32(attrib_location, 2, glow::FLOAT, false, 2 * std::mem::size_of::<f32>() as i32, verts.len() as i32);
            gl.enable_vertex_attrib_array(attrib_location);
            
            eframe::egui_glow::check_for_gl_error!(gl, "mid quad render C");
            
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