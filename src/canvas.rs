
use eframe::egui;
use crate::warimage::*;
use crate::transform::*;
use crate::gizmos::*;

pub (crate) fn canvas(ui : &mut egui::Ui, app : &mut crate::Warpaint) -> egui::Response
{
    let input = ui.input().clone();
    let mut response = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());
    
    // collect input
    
    let time = input.time;
    let delta = input.unstable_dt;
    let mouse_motion = input.pointer.delta();
    let mouse_position = input.pointer.interact_pos().unwrap_or_default();
    let mut mouse_scroll = input.scroll_delta.y;
    
    let mut pressed = [
        input.pointer.button_clicked(egui::PointerButton::Primary),
        input.pointer.button_clicked(egui::PointerButton::Secondary),
        input.pointer.button_clicked(egui::PointerButton::Middle),
        input.pointer.button_clicked(egui::PointerButton::Extra1),
        input.pointer.button_clicked(egui::PointerButton::Extra2),
    ];
    let mut held = [
        input.pointer.button_down(egui::PointerButton::Primary),
        input.pointer.button_down(egui::PointerButton::Secondary),
        input.pointer.button_down(egui::PointerButton::Middle),
        input.pointer.button_down(egui::PointerButton::Extra1),
        input.pointer.button_down(egui::PointerButton::Extra2),
    ];
    let mut released = [
        input.pointer.button_released(egui::PointerButton::Primary),
        input.pointer.button_released(egui::PointerButton::Secondary),
        input.pointer.button_released(egui::PointerButton::Middle),
        input.pointer.button_released(egui::PointerButton::Extra1),
        input.pointer.button_released(egui::PointerButton::Extra2),
    ];
    if !response.dragged()
    {
        for e in pressed.iter_mut()
        {
            *e = false;
        }
        for e in held.iter_mut()
        {
            *e = false;
        }
        for e in released.iter_mut()
        {
            *e = false;
        }
    }
    if !response.hovered()
    {
        mouse_scroll = 0.0;
    }
    
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
    if held[2]
    {
        app.xform.translate([mouse_motion.x, mouse_motion.y]);
    }
    
    if mouse_scroll != 0.0
    {
        let offset = mouse_position - response.rect.center();
        app.xform.translate([-offset.x, -offset.y]);
        app.zoom(mouse_scroll/128.0);
        app.xform.translate([ offset.x,  offset.y]);
    }
    
    let canvas_mouse_coord = {
        let offset = mouse_position;
        let mut coord = [offset.x, offset.y];
        
        let mut xform = app.xform.clone();
        let center = response.rect.center();
        xform.translate([center.x, center.y]);
        xform = xform.inverse();
        
        coord = &xform * &coord;
        coord
    };
    
    if held[0]
    {
        let mut coord = canvas_mouse_coord;
        coord[0] += app.image.width as f32 / 2.0;
        coord[1] += app.image.height as f32 / 2.0;
        app.debug(format!("{:?}", coord));
        let color = px_to_int(app.main_color_rgb);
        app.debug(format!("{:?}", color));
        app.image.set_pixel(coord[0] as isize, coord[1] as isize, color);
    }
    
    // render canvas
    
    let tex = app.image_preview.as_ref().unwrap();
    let size = tex.size();
    let (mut w, mut h) = (size[0] as f32, size[1] as f32);
    
    let mut mesh = egui::Mesh::with_texture(tex.id());
    let mut rect : egui::Rect = [[-w/2.0, -h/2.0].into(), [w/2.0, h/2.0].into()].into();
    let uv = [[0.0, 0.0].into(), [1.0, 1.0].into()].into();
    mesh.add_rect_with_uv (
        rect,
        uv,
        egui::Color32::WHITE
    );
    let mut xform = app.xform.clone();
    let center = response.rect.center();
    xform.translate([center.x, center.y]);
    for vert in mesh.vertices.iter_mut()
    {
        let new = &xform * &[vert.pos[0], vert.pos[1]];
        vert.pos.x = new[0];
        vert.pos.y = new[1];
    }
    
    let painter = ui.painter_at(response.rect);
    painter.add(egui::Shape::mesh(mesh));
    
    
    let mut gizmo = BrushGizmo { x : canvas_mouse_coord[0].floor() + 0.5, y : canvas_mouse_coord[1].floor() + 0.5, r : 0.5 };
    gizmo.draw(ui, app, &mut response, &painter);
    
    
    response
}