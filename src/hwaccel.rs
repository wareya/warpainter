use glow::*;

fn create_render_target(gl: &Context, width: i32, height: i32) -> (Framebuffer, Texture)
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

        assert_eq!(gl.check_framebuffer_status(FRAMEBUFFER), FRAMEBUFFER_COMPLETE);
        gl.bind_framebuffer(FRAMEBUFFER, None);

        (fbo, tex)
    }
}

fn upload_texture(gl: &Context, width: i32, height: i32, pixels: &[u8]) -> Texture
{
    unsafe
    {
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
            PixelUnpackData::Slice(Some(pixels))
        );
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
        gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
        tex
    }
}

fn run(gl : &glow::Context)
{
    let px1 = [255, 128, 64, 255, 255, 255, 64, 128];
    let tex1 = upload_texture(gl, 2, 1, &px1);
    let px2 = [255, 128, 64, 255, 64, 0, 255, 255];
    let tex2 = upload_texture(gl, 1, 2, &px2);
    
    let w = 20;
    let h = 20;
    
    unsafe
    {
        let shvert = gl.create_shader(VERTEX_SHADER).unwrap();
        gl.shader_source(shvert, "
        #version 330 core
        in vec3 vertPos;
        out vec2 uv;
        void main()
        {
            gl_Position = vec4(vertPos, 1.0);
            uv = vertPos.xy * 0.5 + vec2(0.5);
        }
        ");
        gl.compile_shader(shvert);
        let shader_log = gl.get_shader_info_log(shvert);
        if !shader_log.is_empty()
        {
            panic!("Vertex Shader Compile Error: {}", shader_log);
        }
        
        let shfrag = gl.create_shader(FRAGMENT_SHADER).unwrap();
        gl.shader_source(shfrag, "
        #version 330
        in vec2 uv;
        out vec4 out_color;
        
        uniform sampler2D tex1;
        uniform sampler2D tex2;
        uniform vec2 out_size;
        
        //JIT_CODE_INSERTION_POINT
        
        void main()
        {
            vec4 a = texture(tex1, uv);
            vec4 b = texture(tex2, uv);
            out_color = mix(a, b, 0.5);
        }");
        gl.compile_shader(shfrag);
        let shader_log = gl.get_shader_info_log(shfrag);
        if !shader_log.is_empty()
        {
            panic!("Vertex Shader Compile Error: {}", shader_log);
        }
        
        let prog = gl.create_program().unwrap();
        gl.attach_shader(prog, shvert);
        gl.attach_shader(prog, shfrag);
        gl.link_program(prog);
        let linked = gl.get_program_info_log(prog);
        if !linked.is_empty()
        {
            panic!("Program link error: {}", linked);
        }
        
        let (target, tex) = create_render_target(gl, w, h);
        
        gl.bind_framebuffer(FRAMEBUFFER, Some(target));
        gl.viewport(0, 0, w, h);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(COLOR_BUFFER_BIT);
        
        gl.use_program(Some(prog));
        gl.active_texture(TEXTURE0);
        gl.bind_texture(TEXTURE_2D, Some(tex1));
        gl.uniform_1_i32(gl.get_uniform_location(prog, "tex1").as_ref(), 0);
        
        gl.active_texture(TEXTURE1);
        gl.bind_texture(TEXTURE_2D, Some(tex2));
        gl.uniform_1_i32(gl.get_uniform_location(prog, "tex2").as_ref(), 1);
        
        gl.uniform_2_f32(gl.get_uniform_location(prog, "out_size").as_ref(), w as f32, h as f32);
        
        // Quad vertices (only positions)
        let vertices : [f32; 12] = [
            -1.0,  1.0, 0.0,  // Top-left
             1.0,  1.0, 0.0,  // Top-right
            -1.0, -1.0, 0.0,  // Bottom-left
             1.0, -1.0, 0.0,  // Bottom-right
        ];
        
        let mut vert_bytes = Vec::with_capacity(vertices.len() * std::mem::size_of::<f32>());
        for &v in &vertices {
            vert_bytes.extend_from_slice(&v.to_ne_bytes());
        }
        
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        
        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &vert_bytes, STATIC_DRAW);
        
        gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, 3 * std::mem::size_of::<f32>() as i32, 0);
        gl.enable_vertex_attrib_array(0);
        
        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        
        gl.bind_vertex_array(Some(vao));
        gl.draw_arrays(TRIANGLE_STRIP, 0, 4);
        
        gl.finish();
        
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        gl.bind_texture(TEXTURE_2D, Some(tex));
        gl.read_pixels(0, 0, w, h, RGBA, UNSIGNED_BYTE, glow::PixelPackData::Slice(Some(&mut pixels)));
        println!("{:?}", pixels);
        
        gl.bind_framebuffer(FRAMEBUFFER, None);
    }
}

#[cfg(test)]
#[allow(deprecated)]
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
            
            let gl = glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s) as *const _);

            run(&gl);
        }
    }
}