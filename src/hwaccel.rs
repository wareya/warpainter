use glow::*;
use crate::warimage::*;

fn create_render_target(gl : &Context, width : i32, height : i32) -> (Framebuffer, Texture)
{
    unsafe
    {
        let fbo = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(FRAMEBUFFER, Some(fbo));

        let tex = gl.create_texture().unwrap();
        gl.bind_texture(TEXTURE_2D, Some(tex));
        gl.tex_image_2d(
            TEXTURE_2D,
            0,
            RGBA as i32,
            width,
            height,
            0,
            RGBA,
            UNSIGNED_BYTE,
            PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
        gl.framebuffer_texture_2d(FRAMEBUFFER, COLOR_ATTACHMENT0, TEXTURE_2D, Some(tex), 0);

        assert_ne!(gl.check_framebuffer_status(FRAMEBUFFER), FRAMEBUFFER_INCOMPLETE_ATTACHMENT);
        assert_ne!(gl.check_framebuffer_status(FRAMEBUFFER), FRAMEBUFFER_INCOMPLETE_DIMENSIONS);
        assert_ne!(gl.check_framebuffer_status(FRAMEBUFFER), FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT);
        assert_ne!(gl.check_framebuffer_status(FRAMEBUFFER), FRAMEBUFFER_UNSUPPORTED);
        assert_eq!(gl.check_framebuffer_status(FRAMEBUFFER), FRAMEBUFFER_COMPLETE);
        gl.bind_framebuffer(FRAMEBUFFER, None);

        (fbo, tex)
    }
}
fn upload_texture_rgba8(gl : &Context, rect : [[u32; 2]; 2], width : i32, height : i32, pixels : &[u8]) -> Texture
{
    unsafe
    {
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(TEXTURE_2D, Some(tex));
        
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
        // not supported on glow's web backend
        //gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_BORDER as i32);
        //gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_BORDER as i32);
        
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, width as i32);

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            (rect[1][0] - rect[0][0]) as i32,
            (rect[1][1] - rect[0][1]) as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(&pixels[(rect[0][1] * width as u32 + rect[0][0]) as usize * 4..])),
        );

        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);

        gl.tex_image_2d(
            TEXTURE_2D,
            0,
            RGBA as i32,
            width,
            height,
            0,
            RGBA,
            UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(pixels)),
        );
        
        gl.bind_buffer(PIXEL_UNPACK_BUFFER, None);
        
        tex
    }
}
fn upload_texture_r8(gl : &Context, width : i32, height : i32, pixels : &[u8]) -> Texture
{
    unsafe
    {
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(TEXTURE_2D, Some(tex));
        gl.tex_image_2d(
            TEXTURE_2D,
            0,
            RED as i32,
            width,
            height,
            0,
            RED,
            UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(pixels))
        );
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
        
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_BORDER as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_BORDER as i32);
        
        tex
    }
}

struct OpenGLContextState
{
    //program: Option<u32>,
    fbo: Option<Framebuffer>,
    //buffers: HashMap<u32, u32>,
}

impl OpenGLContextState
{
    fn new() -> Self
    {
        OpenGLContextState
        {
            //program: None,
            fbo: None,
            //buffers: HashMap::new(),
        }
    }

    fn save_state(&mut self, gl: &Context)
    {
        //self.program = Some(unsafe { gl.get_parameter_i32(PROGRAM) as u32 });
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.fbo = unsafe { gl.get_parameter_framebuffer(FRAMEBUFFER_BINDING) };
        }
        
        //let buffer_targets = vec![
        //    ARRAY_BUFFER,
        //    ELEMENT_ARRAY_BUFFER,
        //    COPY_READ_BUFFER,
        //    COPY_WRITE_BUFFER,
        //];
        //
        //for &target in &buffer_targets {
        //    let buffer_id = unsafe { gl.get_parameter_i32(target) };
        //    self.buffers.insert(target, buffer_id as u32);
        //}
    }
    #[cfg(target_arch = "wasm32")]
    fn load_state(&self, gl: &Context)
    {
        //if let Some(program_id) = self.program
        //{
        //    unsafe { gl.use_program(Some(WebProgramKey::from(KeyData.program_id))) };
        //}
        
        unsafe { gl.bind_framebuffer(FRAMEBUFFER, None) };
        
        //for (&target, &id) in &self.buffers
        //{
        //    unsafe { gl.bind_buffer(target, Some(WebBufferKey::from(KeyData.id))) };
        //}
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn load_state(&self, gl: &Context)
    {
        //if let Some(program_id) = self.program
        //{
        //    unsafe { gl.use_program(NonZeroU32::new(program_id).map(|x| NativeProgram(x))) };
        //}
        
        unsafe { gl.bind_framebuffer(FRAMEBUFFER, None) };
        
        //for (&target, &id) in &self.buffers
        //{
        //    unsafe { gl.bind_buffer(target, NonZeroU32::new(id).map(|x| NativeBuffer(x))) };
        //}
    }
}

pub (crate) fn hw_blend(gl : &Context, rect : [[i32; 2]; 2], f : Option<String>, img1 : Option<&Image<4>>, mut img1_pos : [f32; 2], img2 : Option<&Image<4>>, mut img2_pos : [f32; 2], opacity : f32, modifier : f32, funny_flag : bool) -> Result<Vec<u8>, String>
{
    let mut state = OpenGLContextState::new();
    state.save_state(gl);
    
    fn vec_array_to_bytes<const N : usize>(vec: &Vec<[u8; N]>) -> Vec<u8>
    {
        vec.iter().flat_map(|arr| arr.iter().copied()).flat_map(|x| x.to_ne_bytes()).collect()
    }
    let w = (rect[1][0] - rect[0][0]) as i32;
    let h = (rect[1][1] - rect[0][1]) as i32;
    if w <= 0 || h <= 0
    {
        return Ok(vec!());
    }
    println!("{:?}", rect);
    
    unsafe
    {
        let prefix = {
        #[cfg(not(target_arch = "wasm32"))]
        {
            "#version 330\n".to_string()
        }
        #[cfg(target_arch = "wasm32")]
        {
            "#version 300 es\nprecision highp float;".to_string()
        }};
        
        let shvert = gl.create_shader(VERTEX_SHADER);
        if let Err(err) = shvert
        {
            return Err(err);
        }
        let shvert = shvert.unwrap();
        gl.shader_source(shvert, &(prefix.clone() + "
        in vec3 vertPos;
        out vec2 uv;
        void main()
        {
            gl_Position = vec4(vertPos, 1.0);
            uv = vertPos.xy * 0.5 + vec2(0.5);
        }
        "));
        gl.compile_shader(shvert);
        let shader_log = gl.get_shader_info_log(shvert);
        if !shader_log.is_empty()
        {
            gl.delete_shader(shvert);
            return Err(format!("Vertex Shader Compile Error: {}", shader_log));
        }
        
        let shfrag = gl.create_shader(FRAGMENT_SHADER);
        if let Err(err) = shfrag
        {
            gl.delete_shader(shvert);
            return Err(err);
        }
        let shfrag = shfrag.unwrap();
        let shfrag_src = prefix + &"
        in vec2 uv;
        out vec4 out_color;
        
        uniform sampler2D tex1;
        uniform sampler2D tex2;
        uniform vec2 out_size;
        uniform vec2 tex1_pos;
        uniform vec2 tex2_pos;
        
        uniform float opacity;
        uniform float _fill_opacity;
        uniform float funny_flag;
        
        //JIT_CODE_INSERTION_POINT
        
        void main()
        {
            vec2 uv1 = uv - tex1_pos / out_size;
            uv1 /= vec2(textureSize(tex1, 0)) / out_size;
            vec2 uv2 = uv - tex2_pos / out_size;
            uv2 /= vec2(textureSize(tex2, 0)) / out_size;
            out_color = f(uv1, uv2);
        }".replace("//JIT_CODE_INSERTION_POINT", f.unwrap_or("
        vec4 f(vec2 uv1, vec2 uv2)
        {
            vec4 a = texture(tex1, uv1);
            vec4 b = texture(tex2, uv2);
            if (uv1.x < 0.0 || uv1.x > 1.0 || uv1.y < 0.0 || uv1.y > 1.0)
                a *= 0.0;
            if (uv2.x < 0.0 || uv2.x > 1.0 || uv2.y < 0.0 || uv2.y > 1.0)
                b *= 0.0;
            return (a+b)*0.5;
        }
        ".to_string()).as_str());
        gl.shader_source(shfrag, &shfrag_src);
        
        //println!("{}", shfrag_src);
        
        gl.compile_shader(shfrag);
        let shader_log = gl.get_shader_info_log(shfrag);
        if !shader_log.is_empty()
        {
            gl.delete_shader(shvert);
            gl.delete_shader(shfrag);
            return Err(format!("Vertex Shader Compile Error: {}", shader_log));
        }
        
        let prog = gl.create_program();
        if let Err(err) = prog
        {
            gl.delete_shader(shvert);
            gl.delete_shader(shfrag);
            return Err(err);
        }
        let prog = prog.unwrap();
        gl.attach_shader(prog, shvert);
        gl.attach_shader(prog, shfrag);
        gl.link_program(prog);
        let linked = gl.get_program_info_log(prog);
        if !linked.is_empty()
        {
            gl.delete_shader(shvert);
            gl.delete_shader(shfrag);
            gl.delete_program(prog);
            return Err(format!("Program link error: {}", linked));
        }
        
        let vao = gl.create_vertex_array();
        if let Err(err) = vao
        {
            gl.delete_shader(shvert);
            gl.delete_shader(shfrag);
            gl.delete_program(prog);
            return Err(err);
        }
        let vao = vao.unwrap();
        
        let vbo = gl.create_buffer();
        if let Err(err) = vbo
        {
            gl.delete_vertex_array(vao);
            gl.delete_shader(shvert);
            gl.delete_shader(shfrag);
            gl.delete_program(prog);
            return Err(err);
        }
        let vbo = vbo.unwrap();
        
        let start = web_time::Instant::now();
        
        let rect_offset = |mut rect : [[i32; 2]; 2], pos : &mut [f32; 2], w : i32, h : i32|
        {
            rect[0][0] -= pos[0] as i32;
            rect[0][1] -= pos[1] as i32;
            rect[1][0] -= pos[0] as i32;
            rect[1][1] -= pos[1] as i32;
            
            if rect[0][0] < 0
            {
                pos[0] -= rect[0][0] as f32;
                rect[0][0] = 0;
            }
            if rect[0][1] < 0
            {
                pos[1] -= rect[0][1] as f32;
                rect[0][1] = 0;
            }
            
            if rect[1][0] > w
            {
                rect[1][0] = w;
            }
            if rect[1][1] > h
            {
                rect[1][1] = h;
            }
            
            let rect = [[rect[0][0] as u32, rect[0][1] as u32], [rect[1][0] as u32, rect[1][1] as u32]];
            rect
        };

        let tex1 = if let Some(x) = img1
        {
            let rect = rect_offset(rect, &mut img1_pos, x.width as i32, x.height as i32);
            if rect[0][0] >= rect[1][0] || rect[0][1] >= rect[1][1]
            {
                None
            }
            else
            {
                println!("rect1: {:?}", rect);
                if let ImageData::Int(ref data) = x.data
                {
                    Some(upload_texture_rgba8(gl, rect, x.width as i32, x.height as i32, std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)))
                }
                else { panic!(); }
            }
        } else { None };
        
        let elapsed = start.elapsed().as_secs_f32();
        println!("Upload tex1 took {:.6} seconds", elapsed);
        
        let start = web_time::Instant::now();
        
        let tex2 = if let Some(x) = img2
        {
            let rect = rect_offset(rect, &mut img2_pos, x.width as i32, x.height as i32);
            if rect[0][0] >= rect[1][0] || rect[0][1] >= rect[1][1]
            {
                None
            }
            else
            {
                println!("rect2: {:?}", rect);
                if let ImageData::Int(ref data) = x.data
                {
                    Some(upload_texture_rgba8(gl, rect, x.width as i32, x.height as i32, std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)))
                }
                else { panic!(); }
            }
        } else { None };
        
        img1_pos[0] -= rect[0][0] as f32;
        img1_pos[1] -= rect[0][1] as f32;
        img2_pos[0] -= rect[0][0] as f32;
        img2_pos[1] -= rect[0][1] as f32;
        
        let elapsed = start.elapsed().as_secs_f32();
        println!("Upload tex2 took {:.6} seconds", elapsed);
        
        let (target, tex) = create_render_target(gl, w, h);
        
        gl.bind_framebuffer(FRAMEBUFFER, Some(target));
        gl.viewport(0, 0, w, h);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(COLOR_BUFFER_BIT);
        
        gl.use_program(Some(prog));
        gl.active_texture(TEXTURE0);
        gl.bind_texture(TEXTURE_2D, tex1);
        gl.uniform_1_i32(gl.get_uniform_location(prog, "tex1").as_ref(), 0);
        
        gl.active_texture(TEXTURE1);
        gl.bind_texture(TEXTURE_2D, tex2);
        gl.uniform_1_i32(gl.get_uniform_location(prog, "tex2").as_ref(), 1);
        
        gl.uniform_2_f32(gl.get_uniform_location(prog, "out_size").as_ref(), w as f32, h as f32);
        
        gl.uniform_2_f32(gl.get_uniform_location(prog, "tex1_pos").as_ref(), img1_pos[0] as f32, img1_pos[1] as f32);
        gl.uniform_2_f32(gl.get_uniform_location(prog, "tex2_pos").as_ref(), img2_pos[0] as f32, img2_pos[1] as f32);
        
        gl.uniform_1_f32(gl.get_uniform_location(prog, "opacity").as_ref(), opacity);
        gl.uniform_1_f32(gl.get_uniform_location(prog, "_fill_opacity").as_ref(), modifier);
        gl.uniform_1_f32(gl.get_uniform_location(prog, "funny_flag").as_ref(), if funny_flag { 1.0 } else { 0.0 });
        
        let vertices : [f32; 12] = [
            -1.0,  1.0, 0.0,
             1.0,  1.0, 0.0,
            -1.0, -1.0, 0.0,
             1.0, -1.0, 0.0,
        ];
        
        let mut vert_bytes = Vec::with_capacity(vertices.len() * std::mem::size_of::<f32>());
        for &v in &vertices
        {
            vert_bytes.extend_from_slice(&v.to_ne_bytes());
        }
        
        gl.bind_vertex_array(Some(vao));
        
        gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(ARRAY_BUFFER, &vert_bytes, STATIC_DRAW);
        
        gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, 3 * std::mem::size_of::<f32>() as i32, 0);
        gl.enable_vertex_attrib_array(0);
        
        gl.bind_vertex_array(None);
        gl.bind_buffer(ARRAY_BUFFER, None);
        
        gl.bind_vertex_array(Some(vao));
        gl.draw_arrays(TRIANGLE_STRIP, 0, 4);
        
        gl.finish();
        
        let start = web_time::Instant::now();

        let mut pixels = vec![0u8; (w * h * 4) as usize];
        gl.bind_texture(TEXTURE_2D, Some(tex));
        gl.read_pixels(0, 0, w, h, RGBA, UNSIGNED_BYTE, PixelPackData::Slice(Some(&mut pixels)));
        
        let elapsed = start.elapsed().as_secs_f32();
        println!("Readback took {:.6} seconds", elapsed);

        state.load_state(gl);
        
        if let Some(x) = tex1 { gl.delete_texture(x); }
        if let Some(x) = tex2 { gl.delete_texture(x); }
        gl.delete_texture(tex);
        gl.delete_framebuffer(target);
        gl.delete_vertex_array(vao);
        gl.delete_buffer(vbo);
        gl.delete_shader(shvert);
        gl.delete_shader(shfrag);
        gl.delete_program(prog);
        
        return Ok(pixels);
    }
}

#[cfg(test)]
#[allow(deprecated)]
#[cfg(target_os = "windows")]
mod tests
{
    use super::*;
    use winit::{
        event_loop::EventLoop,
        platform::windows::EventLoopBuilderExtWindows as _,
        raw_window_handle::{HasDisplayHandle, HasRawWindowHandle},
        window::WindowAttributes,
        dpi::*
    };
    use glutin::{
        config::{Api, ConfigTemplateBuilder},
        context::ContextAttributesBuilder,
        display::{Display, DisplayApiPreference},
        prelude::{GlDisplay as _, NotCurrentGlContext as _},
        surface::{PbufferSurface, SurfaceAttributesBuilder}
    };

    #[test]
    pub fn test()
    {
        let mut img1 = Image::<4>::blank(4, 1);
        img1.set_pixel(0, 0, [255, 255, 0, 255]);
        img1.set_pixel(1, 0, [255, 0, 0, 255]);
        img1.set_pixel(2, 0, [0, 0, 0, 255]);
        img1.set_pixel(3, 0, [255, 0, 128, 255]);
        
        let mut img2 = Image::<4>::blank(1, 4);
        img2.set_pixel(0, 0, [0,   0, 255, 255]);
        img2.set_pixel(0, 1, [0,   128, 0, 255]);
        img2.set_pixel(0, 2, [128,   128, 0, 255]);
        img2.set_pixel(0, 3, [255, 128,   0, 255]);
        
        let el = EventLoop::builder().with_any_thread(true).build().unwrap();

        let mut attrs = WindowAttributes::default();
        attrs.visible = false;
        attrs.inner_size = Some(Size::Physical(PhysicalSize { width: 1, height: 1 }));
        
        let window = el.create_window(attrs).unwrap();
        let raw_handle = window.raw_window_handle().unwrap();
        
        unsafe
        {
            let raw_disp_handle = el.display_handle().unwrap().as_raw();
            let display = Display::new(raw_disp_handle, DisplayApiPreference::EglThenWgl(Some(raw_handle))).unwrap();
            
            let template = ConfigTemplateBuilder::new().with_api(Api::OPENGL).build();  
            let config = display.find_configs(template).unwrap().next().expect("no matching config");
            
            let surface_attrs = SurfaceAttributesBuilder::<PbufferSurface>::new().build(128u32.try_into().unwrap(), 128u32.try_into().unwrap());
            let pbuffer = display.create_pbuffer_surface(&config, &surface_attrs).unwrap();
            
            let context_attributes = ContextAttributesBuilder::new();
            let ctx = display.create_context(&config, &context_attributes.build(Some(raw_handle))).unwrap();
            let _gl_ctx = ctx.make_current(&pbuffer).unwrap();
            
            let gl = Context::from_loader_function_cstr(|s| display.get_proc_address(s) as *const _);
            
            hw_blend(&gl, None, Some(&img1), [2.0, 0.0], Some(&img2), [0.0, 0.0], [20, 20], 1.0, 1.0, false).unwrap();
        }
    }
}