
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
    
    let mut xform = app.xform.clone();
    let center = response.rect.center();
    xform.translate([center.x, center.y]);
    xform.translate([-response.rect.min.x, -response.rect.min.y]);
    //// !!!! evil vile code
    let texture = app.flatten().clone(); // FIXME use an Arc somehow
    let uniforms = [
        ("width", response.rect.width()),
        ("height", response.rect.height()),
        ("canvas_width", texture.width as f32),
        ("canvas_height", texture.height as f32),
        ("mat_0_0", xform.rows[0][0]),
        ("mat_0_1", xform.rows[1][0]),
        ("mat_1_0", xform.rows[0][1]),
        ("mat_1_1", xform.rows[1][1]),
        ("mat_2_0", xform.rows[0][2]),
        ("mat_2_1", xform.rows[1][2]),
    ];
    let colorpicker_shader = Arc::clone(app.shaders.get("canvasbackground").unwrap());
    let cb = egui_glow::CallbackFn::new(move |_info, glow_painter|
    {
        let mut shader = colorpicker_shader.lock();
        shader.add_texture(glow_painter.gl(), &texture);
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