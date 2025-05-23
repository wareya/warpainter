
use alloc::sync::Arc;

use eframe::egui;
use eframe::egui_glow;
use crate::quadrender;
use crate::transform::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub (crate) struct CanvasInputState
{
    pub (crate) time : f32,
    pub (crate) delta : f32,
    pub (crate) held : [bool; 5],
    pub (crate) canvas_mouse_coord : [f32; 2],
    pub (crate) window_mouse_coord : [f32; 2],
    pub (crate) view_mouse_coord : [f32; 2],
    pub (crate) window_mouse_motion : [f32; 2],
    pub (crate) mouse_scroll : f32,
    pub (crate) zoom : f32,
    pub (crate) touch_rotation : f32,
    pub (crate) touch_scroll : [f32; 2],
    pub (crate) touch_center : [f32; 2],
    pub (crate) pressure : f32,
    pub (crate) mouse_in_canvas : bool,
    pub (crate) mouse_in_canvas_area : bool,
    pub (crate) cancel : bool,
    pub (crate) pos_is_real : bool,
}

pub (crate) fn to_array(v : egui::Pos2) -> [f32; 2]
{
    [v.x, v.y]
}

impl CanvasInputState
{
    fn update(&mut self, app : &crate::Warpainter, input : &egui::InputState, response : &egui::Response, pxscale : f32)
    {
        self.time = input.time as f32;
        self.delta = input.unstable_dt;
        self.window_mouse_motion = to_array(input.pointer.delta().to_pos2());
        self.window_mouse_coord = to_array(input.pointer.interact_pos().unwrap_or_default());
        self.pos_is_real = input.pointer.interact_pos().is_some();
        self.view_mouse_coord = vec_sub(&self.window_mouse_coord, &to_array(response.rect.min));
        self.mouse_scroll = input.raw_scroll_delta.y;
        
        self.held = [
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_down(egui::PointerButton::Secondary),
            input.pointer.button_down(egui::PointerButton::Middle),
            input.pointer.button_down(egui::PointerButton::Extra1),
            input.pointer.button_down(egui::PointerButton::Extra2),
        ];
        
        self.touch_scroll = [0.0, 0.0];
        self.touch_center = self.window_mouse_coord;
        self.touch_rotation = 0.0;
        self.zoom = 1.0;
        self.pressure = 1.0;
        
        if let Some(mt) = input.multi_touch()
        {
            self.touch_scroll[0] = mt.translation_delta.x;
            self.touch_scroll[1] = mt.translation_delta.y;
            
            self.touch_center[0] = mt.center_pos.x;
            self.touch_center[1] = mt.center_pos.y;
            
            self.touch_rotation = mt.rotation_delta * 180.0 / 3.1415926535;
            self.zoom = mt.zoom_delta;
            
            println!("mt force: {}", mt.force);
            self.pressure = mt.force;
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.pressure = *app.shared.lock();
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.pressure = unsafe { crate::WEB_PRESSURE };
        }
        
        for x in &input.raw.events
        {
            match x
            {
                #[allow(unused)]
                egui::Event::Touch{device_id, id, phase, pos, force} =>
                {
                    println!("{:?}", force);
                    if let Some(p) = force
                    {
                        self.pressure = *p;
                    }
                }
                _ => {}
            }
        }
        
        if !response.is_pointer_button_down_on() && !response.drag_stopped()
        {
            for e in self.held.iter_mut()
            {
                *e = false;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if !response.hovered()
            {
                self.mouse_scroll = 0.0;
            }
        }
        
        self.canvas_mouse_coord = {
            let mut coord = self.window_mouse_coord;
            let mut xform = app.xform.clone();
            let center = response.rect.center();
            xform.scale(pxscale);
            xform.translate([center.x, center.y]);
            xform = xform.inverse();
            
            coord = &xform * &coord;
            
            coord[0] += app.canvas_width as f32 / 2.0;
            coord[1] += app.canvas_height as f32 / 2.0;
            
            coord
        };
        
        let in_x = self.canvas_mouse_coord[0] >= 0.0 && self.canvas_mouse_coord[0] < app.canvas_width as f32;
        let in_y = self.canvas_mouse_coord[1] >= 0.0 && self.canvas_mouse_coord[1] < app.canvas_height as f32;
        
        self.mouse_in_canvas = response.hovered() && in_x && in_y;
        self.mouse_in_canvas_area = response.hovered();
    }
}

pub (crate) fn canvas(ui : &mut egui::Ui, app : &mut crate::Warpainter, focus_is_global : bool, multitouch : bool) -> (egui::Response, CanvasInputState)
{
    let realstart = web_time::Instant::now();
    
    let input = ui.input(|input| input.clone());
    let mut response = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());
    let painter = ui.painter_at(response.rect);
    
    let scale = 1.0/ui.pixels_per_point();
    
    response.rect.min.x = (response.rect.min.x/scale).round()*scale;
    response.rect.min.y = (response.rect.min.y/scale).round()*scale;
    response.rect.max.x = (response.rect.max.x/scale).round()*scale;
    response.rect.max.y = (response.rect.max.y/scale).round()*scale;
    
    // collect input
    
    let mut inputstate = CanvasInputState::default();
    inputstate.update(app, &input, &response, scale);
    
    if !app.loaded_shaders
    {
        return (response, inputstate);
    }
    
    // handle input
    
    if focus_is_global
    {
        if input.key_pressed(egui::Key::Num0)
        {
            app.xform = Transform::ident();
        }
        for _ in 0..input.num_presses(egui::Key::Num2)
        {
            app.xform.rotate(5.0);
            app.debug(format!("{}", app.xform.get_scale()));
            app.debug(format!("{}", app.xform.get_rotation()));
            app.debug(format!("{:?}", app.xform.get_translation()));
        }
        for _ in 0..input.num_presses(egui::Key::Num1)
        {
            app.xform.rotate(-5.0);
            app.debug(format!("{}", app.xform.get_scale()));
            app.debug(format!("{}", app.xform.get_rotation()));
            app.debug(format!("{:?}", app.xform.get_translation()));
        }
    }
    
    let mut view_moved = false;
    
    // FIXME: enforce that the canvas does not go offscreen
    // (idea: if center of screen is not inside of canvas, prevent center of canvas from going past edges)
    // (idea 2: prevent right extrema from going more than 25% leftwards from center, and so on for all extrema)
    if inputstate.held[2] && !matches!(inputstate.window_mouse_motion, [0.0, 0.0])
    {
        app.xform.translate(inputstate.window_mouse_motion);
        view_moved = true;
    }
    if !matches!(inputstate.touch_scroll, [0.0, 0.0])
    {
        app.xform.translate(vec_mul_scalar(&inputstate.touch_scroll, 1.0/scale));
        view_moved = true;
    }
    if inputstate.touch_rotation != 0.0
    {
        let offset = vec_mul_scalar(&vec_sub(&inputstate.touch_center, &to_array(response.rect.center())), 1.0/scale);
        app.xform.translate(vec_sub(&[0.0, 0.0], &offset));
        app.xform.rotate(inputstate.touch_rotation);
        app.xform.translate(offset);
        view_moved = true;
    }
    
    if inputstate.zoom != 1.0
    {
        let offset = vec_mul_scalar(&vec_sub(&inputstate.touch_center, &to_array(response.rect.center())), 1.0/scale);
        app.xform.translate(vec_sub(&[0.0, 0.0], &offset));
        app.zoom_unrounded((inputstate.zoom).log2());
        app.xform.translate(offset);
        view_moved = true;
    }
    else if inputstate.mouse_scroll != 0.0
    {
        if inputstate.window_mouse_coord[0] >= response.rect.min.x && // fix phantom zooming on desktop web
            inputstate.window_mouse_coord[1] >= response.rect.min.y &&
            inputstate.window_mouse_coord[0] <= response.rect.max.x &&
            inputstate.window_mouse_coord[1] <= response.rect.max.y
        {
            let offset = vec_sub(&inputstate.window_mouse_coord, &to_array(response.rect.center()));
            app.xform.translate(vec_sub(&[0.0, 0.0], &offset));
            app.zoom(inputstate.mouse_scroll/128.0/2.0);
            app.xform.translate(offset);
            view_moved = true;
        }
    }
    
    if view_moved
    {
        inputstate.cancel = true;
        inputstate.held[1] = true;
    }
    
    if !multitouch
    {
        app.tool_think(&inputstate);
    }
    
    // render canvas
    
    let _start = web_time::Instant::now();
    
    use std::sync::Mutex;
    use std::sync::LazyLock;
    static LAST_PROGRESS : LazyLock<Arc<Mutex<u128>>> = std::sync::LazyLock::new(|| Arc::new(Mutex::new(!0u128)));
    
    let start = web_time::Instant::now();
    
    use std::ops::DerefMut;
    let mut tex_new = false;
    let newprog = app.edit_progress;
    #[allow(irrefutable_let_patterns)] // bugged warning. the let binding holds the lock guard
    if let x = LAST_PROGRESS.lock().unwrap().deref_mut()
    {
        if *x != newprog
        {
            //println!("reflattening (edit progress)");
            app.flatten();
            tex_new = true;
        }
    }
    let mut texture = app.flatten_use();
    if texture.is_none()
    {
        //println!("reflattening");
        app.flatten();
        texture = app.flatten_use();
        assert!(texture.is_some());
        tex_new = true;
    }
    
    if start.elapsed().as_secs_f64() > 0.001
    {
        //println!("flatten time: {:.3}ms", start.elapsed().as_secs_f64() * 1000.0);
    }
    
    let mut update_rect = None;
    if tex_new
    {
        let r = app.cache_rect;
        update_rect = Some(r);
    }
    
    let texture = texture.unwrap();
    
    let elapsed = start.elapsed().as_secs_f64();
    if elapsed > 0.01
    {
        //println!("time to flatten: {}ms", elapsed * 1000.0);
    }
    
    let (w, h) = (texture.width as f32, texture.height as f32);
    
    let mut xform = app.xform.clone();
    let logical_scale = xform.get_scale();
    xform.scale(scale);
    
    let mut vertices = [
        [-w/2.0, -h/2.0],
        [ w/2.0, -h/2.0],
        [-w/2.0,  h/2.0],
        [ w/2.0,  h/2.0]
    ];
    let uvs = [
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0]
    ];
    
    let mut minima_x = 1000000.0f32;
    let mut minima_y = 1000000.0f32;
    
    let rot = (xform.get_rotation() + (360.0 + 45.0)) % 90.0;
    
    for vert in vertices.iter_mut()
    {
        *vert = &xform * &[vert[0], vert[1]];
        
        minima_x = minima_x.min(vert[0]);
        minima_y = minima_y.min(vert[1]);
        
        //println!("{}", logical_scale);
        if (logical_scale - 1.0).abs() < 0.001 && (rot - 45.0).abs() < 0.001
        {
            vert[0] = (vert[0]/scale).round()*scale;
            vert[1] = (vert[1]/scale).round()*scale;
            if ((response.rect.width()/scale).round() as isize & 1) == 1
            {
                vert[0] += 0.5*scale;
            }
            if ((response.rect.height()/scale).round() as isize & 1) == 1
            {
                vert[1] += 0.5*scale;
            }
        }
        
        vert[0] *= 2.0/response.rect.width();
        vert[1] *= 2.0/response.rect.height();
    }
    
    //let mut r = [[response.rect.min.x, response.rect.min.y], [response.rect.max.x, response.rect.max.y]];
    //r[0][0] = r[0][0]/scale;
    //r[0][1] = r[0][1]/scale;
    //r[1][0] = r[1][0]/scale;
    //r[1][1] = r[1][1]/scale;
    //println!("{:?} {:?}", response.rect, r);
    
    let uniforms = [
        ("width", response.rect.width()),
        ("height", response.rect.height()),
        ("canvas_width", w),
        ("canvas_height", h),
        ("minima_x", minima_x),
        ("minima_y", minima_y),
        ("zoom_level", logical_scale),
    ];
    
    let loops = app.get_selection_loop_data();
    
    let canvas_shader = Arc::clone(app.shaders.get("canvasbackground").unwrap());
    
    let mut shader = canvas_shader.lock();
    
    let gl = unsafe { &* &raw const crate::GL }.as_ref().unwrap();
    
    let gl = &*gl;
    
    if tex_new
    {
        let _start = web_time::Instant::now();
        #[allow(irrefutable_let_patterns)] // bugged. the binding holds a lock guard
        if let x = LAST_PROGRESS.lock().unwrap().deref_mut()
        {
            if *x != newprog
            {
                *x = newprog;
            }
        }
        let t = &texture;
        if let (Some(handle), Some(mut rect)) = (shader.use_texture(gl, 0), update_rect)
        {
            rect[0][0] = rect[0][0].max(0.0);
            rect[0][1] = rect[0][1].max(0.0);
            rect[1][0] = rect[1][0].min(t.width as f32);
            rect[1][1] = rect[1][1].min(t.height as f32);
            
            let s = shader.get_texture_size(0);
            
            if (rect[1][0] - rect[0][0] != t.width as f32 || rect[1][1] - rect[0][1] != t.height as f32)
                && s == [t.width as i32, t.height as i32]
            {
                //println!("partial upload {:?}", rect);
                quadrender::update_texture(gl, handle, t, rect);
            }
            else
            {
                println!("full upload A...");
                shader.add_texture(gl, t, 0);
            }
        }
        else
        {
            println!("full upload B...");
            shader.add_texture(gl, t, 0);
        }
        
        app.cache_rect_reset();
        //println!("canvas upload time: {:.3}ms", start.elapsed().as_secs_f64() * 1000.0);
    }
    drop(shader);
    
    let cb = egui_glow::CallbackFn::new(move |_info, glow_painter|
    {
        let mut shader = canvas_shader.lock();
        
        shader.add_data(glow_painter.gl(), &loops, 1);
        
        shader.add_vertices(&vertices, &uvs);
        shader.render(glow_painter.gl(), &uniforms);
    });
    let callback = egui::PaintCallback { rect : response.rect, callback : Arc::new(cb) };
    painter.add(callback);
    
    if inputstate.mouse_in_canvas_area
    {
        if let Some(tool) = app.get_tool()
        {
            if let Some(mut gizmo) = tool.get_gizmo(app, true)
            {
                gizmo.draw(ui, app, &mut response, &painter);
            }
        }
    }
    
    let realtime = realstart.elapsed().as_secs_f32();
    if realtime > 0.001
    {
        //println!("canvas update time: {:.3}ms", realtime * 1000.0);
    }
    /*
    let grid_size = 16.0;
    
    if grid_size * app.get_zoom() > 8.49
    {
        let grid_x_last = (w/grid_size).floor()*grid_size;
        let grid_y_last = (h/grid_size).floor()*grid_size;
        let x_line_count = (grid_x_last/grid_size) as usize;
        let y_line_count = (grid_y_last/grid_size) as usize;
        
        use crate::gizmos::draw_dotted;
        for (line_count, a, b, c, d) in [
            (x_line_count, vertices[0], vertices[1], vertices[2], vertices[3]),
            (y_line_count, vertices[0], vertices[2], vertices[1], vertices[3])
        ]
        {
            for i in 1..line_count
            {
                let t = i as f32 / line_count as f32;
                
                let fx = response.rect.width()/2.0;
                let fy = response.rect.height()/2.0;
                
                let mut a = vec_lerp(&a, &b, t);
                a[0] = (a[0] + 1.0) * fx + response.rect.min.x;
                a[1] = (a[1] + 1.0) * fy + response.rect.min.y;
                
                let mut b = vec_lerp(&c, &d, t);
                b[0] = (b[0] + 1.0) * fx + response.rect.min.x;
                b[1] = (b[1] + 1.0) * fy + response.rect.min.y;
                
                draw_dotted(&painter, a, b, 2.0);
            }
        }
    }
    */
    
    (response, inputstate)
}