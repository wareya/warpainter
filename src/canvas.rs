
use alloc::sync::Arc;

use eframe::egui;
use eframe::egui_glow;
use crate::transform::*;

#[derive(Clone, Debug, Default)]
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
}

fn to_array(v : egui::Pos2) -> [f32; 2]
{
    [v.x, v.y]
}

impl CanvasInputState
{
    fn update(&mut self, app : &crate::Warpainter, input : &egui::InputState, response : &egui::Response)
    {
        self.time = input.time as f32;
        self.delta = input.unstable_dt;
        self.window_mouse_motion = to_array(input.pointer.delta().to_pos2());
        self.window_mouse_coord = to_array(input.pointer.interact_pos().unwrap_or_default());
        self.view_mouse_coord = vec_sub(&self.window_mouse_coord, &to_array(response.rect.min));
        self.mouse_scroll = input.scroll_delta.y;
        
        self.held = [
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_down(egui::PointerButton::Secondary),
            input.pointer.button_down(egui::PointerButton::Middle),
            input.pointer.button_down(egui::PointerButton::Extra1),
            input.pointer.button_down(egui::PointerButton::Extra2),
        ];
        
        if !response.hovered() && !response.dragged() && !response.drag_released()
        {
            for e in self.held.iter_mut()
            {
                *e = false;
            }
        }
        if !response.hovered()
        {
            self.mouse_scroll = 0.0;
        }
        
        self.canvas_mouse_coord = {
            let mut coord = self.window_mouse_coord;
            let mut xform = app.xform.clone();
            let center = response.rect.center();
            xform.translate([center.x, center.y]);
            xform = xform.inverse();
            
            coord = &xform * &coord;
            
            coord[0] += app.canvas_width as f32 / 2.0;
            coord[1] += app.canvas_height as f32 / 2.0;
            
            coord
        };
    }
}

pub (crate) fn canvas(ui : &mut egui::Ui, app : &mut crate::Warpainter) -> egui::Response
{
    let input = ui.input().clone();
    let mut response = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());
    
    let painter = ui.painter_at(response.rect);
    
    // collect input
    
    let mut inputstate = CanvasInputState::default();
    inputstate.update(app, &input, &response);
    
    // handle input
    
    if input.key_pressed(egui::Key::Num0)
    {
        app.xform = Transform::ident();
    }
    for _ in 0..input.num_presses(egui::Key::Num2)
    {
        app.xform.rotate(15.0);
        app.debug(format!("{}", app.xform.get_scale()));
        app.debug(format!("{}", app.xform.get_rotation()));
        app.debug(format!("{:?}", app.xform.get_translation()));
    }
    for _ in 0..input.num_presses(egui::Key::Num1)
    {
        app.xform.rotate(-15.0);
        app.debug(format!("{}", app.xform.get_scale()));
        app.debug(format!("{}", app.xform.get_rotation()));
        app.debug(format!("{:?}", app.xform.get_translation()));
    }
    
    // FIXME: enforce that the canvas does not go offscreen
    // (idea: if center of screen is not inside of canvas, prevent center of canvas from going past edges)
    // (idea 2: prevent right extrema from going more than 25% leftwards from center, and so on for all extrema)
    if inputstate.held[2]
    {
        app.xform.translate(inputstate.window_mouse_motion);
    }
    
    if inputstate.mouse_scroll != 0.0
    {
        let offset = vec_sub(&inputstate.window_mouse_coord, &to_array(response.rect.center()));
        app.xform.translate(vec_sub(&[0.0, 0.0], &offset));
        app.zoom(inputstate.mouse_scroll/128.0);
        app.xform.translate(offset);
    }
    
    app.tool_think(&inputstate);
    
    // render canvas
    
    let start = std::time::SystemTime::now();
    let texture = app.flatten().clone(); // FIXME
    let elapsed = start.elapsed();
    let elapsed = match elapsed { Ok(x) => x.as_secs_f64(), Err(x) => x.duration().as_secs_f64() };
    if elapsed > 0.1
    {
        println!("time to flatten: {}", elapsed);
    }
    let (w, h) = (texture.width as f32, texture.height as f32);
    
    let xform = app.xform.clone();
    
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
    
    for vert in vertices.iter_mut()
    {
        *vert = &xform * &[vert[0], vert[1]];
        
        minima_x = minima_x.min(vert[0]);
        minima_y = minima_y.min(vert[1]);
        
        vert[0] /= response.rect.width()/2.0;
        vert[1] /= response.rect.height()/2.0;
    }
    
    //// !!!! evil vile code
    let uniforms = [
        ("width", response.rect.width()),
        ("height", response.rect.height()),
        ("canvas_width", w),
        ("canvas_height", h),
        ("minima_x", minima_x),
        ("minima_y", minima_y),
    ];
    let colorpicker_shader = Arc::clone(app.shaders.get("canvasbackground").unwrap());
    let cb = egui_glow::CallbackFn::new(move |_info, glow_painter|
    {
        let mut shader = colorpicker_shader.lock();
        shader.add_texture(glow_painter.gl(), &texture); // FIXME need to clone texture to move into here
        shader.add_vertices(&vertices, &uvs);
        shader.render(glow_painter.gl(), &uniforms);
    });
    let callback = egui::PaintCallback { rect : response.rect, callback : Arc::new(cb) };
    painter.add(callback);
    //// !!!! evil vile code (end)
    
    if let Some(tool) = app.get_tool()
    {
        if let Some(mut gizmo) = tool.get_gizmo(app, true)
        {
            gizmo.draw(ui, app, &mut response, &painter);
        }
    }
    
    response
}