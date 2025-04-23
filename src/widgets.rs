
use alloc::sync::Arc;

use eframe::egui;
use eframe::egui_glow;
use crate::gizmos::draw_doubled_smaller;
use crate::transform::*;
use crate::gizmos::draw_doubled;

/*
pub (crate) fn alpha_picker(ui: &mut egui::Ui, app : &mut crate::Warpainter) -> egui::Response
{
    let input = ui.input().clone();
    
    let avail = ui.available_size();
    let w = avail.x;
    let h = 16.0;
    let least_f = least as f32;
*/

/*
pub (crate) fn canvas_preview(ui: &mut egui::Ui, app : &mut crate::Warpainter, frame : &mut eframe)
{
    let mut response = ui.allocate_ui([0.0, 0.0].into(), egui::Sense::click_and_drag());
    
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
    let center = response.rect.center();
    for vert in mesh.vertices.iter_mut()
    {
        vert.pos += center.to_vec2();
    }
}
*/

/// test
/// ```
/// color_picker(): test
/// ```

pub (crate) fn color_picker(ui: &mut egui::Ui, app : &mut crate::Warpainter, small : bool) -> egui::Response
{
    let input = ui.input(|input| input.clone());
    let _time = input.time as f32;
    
    let avail = ui.available_size();
    let mut least = avail.x.min(avail.y) as usize;
    least = least.clamp(10, if small { 100 } else { 200 });
    let least_f = least as f32;
    
    let ring_size = 0.085*2.0;
    let box_diagonal = least_f * (1.0 - ring_size*1.5);
    let box_size = box_diagonal * 0.5f32.sqrt();
    let box_margin = (least_f - box_size)/2.0;
    
    let ring_inner_radius = least_f/2.0 - least_f/2.0*ring_size;
    let ring_outer_radius = least_f/2.0;
    
    let mut h = app.main_color_hsv[0];
    let mut s = app.main_color_hsv[1];
    let mut v = app.main_color_hsv[2];
    let a = app.main_color_hsv[3];
    
    let least_vec2 = [least as f32, least as f32].into();
    let response = ui.allocate_response(least_vec2, egui::Sense::click_and_drag());
    
    if !app.loaded_shaders
    {
        return response;
    }
    let box_start = response.rect.min + [box_margin, box_margin].into();
    let box_end   = box_start + [box_size, box_size].into();
    let sv_box : egui::Rect = [box_start, box_end].into();
    
    // do input
    
    if response.is_pointer_button_down_on()
    {
        if let Some(drag_origin) = input.pointer.press_origin()
        {
            let rel_origin = drag_origin - sv_box.center();
            let rel_dist = length(&[rel_origin[0], rel_origin[1]]);
            if sv_box.contains(drag_origin)
            {
                // FIXME use input.pointer.interact_pos instead
                let pos = response.interact_pointer_pos().unwrap() - box_start.to_vec2();
                s = pos.x / box_size;
                v = 1.0 - (pos.y / box_size);
                s = s.clamp(0.0, 1.0);
                v = v.clamp(0.0, 1.0);
                app.set_main_color_hsv([h, s, v, a]);
            }
            else if rel_dist > ring_inner_radius && rel_dist < ring_outer_radius
            {
                let pos = response.interact_pointer_pos().unwrap() - sv_box.center();
                h = (pos[1].atan2(pos[0]) / core::f32::consts::PI * 180.0 + 360.0 + 150.0)%360.0;
                app.set_main_color_hsv([h, s, v, a]);
            }
        }
    }
    
    // do rendering
    let painter = ui.painter_at(response.rect);
    
    let mut rect : egui::Rect = [[0.0, 0.0].into(), least_vec2.to_pos2()].into();
    rect = rect.translate(response.rect.min.to_vec2());
    
    //// !!!! evil vile code
    let uniforms = [
        ("width", rect.width()),
        ("height", rect.height()),
        ("hue", h),
        ("ring_size", ring_size),
        ("box_size", box_size),
    ];
    let colorpicker_shader = Arc::clone(app.shaders.get("colorpicker").unwrap());
    let cb = egui_glow::CallbackFn::new(move |_info, glow_painter|
    {
        colorpicker_shader.lock().render(glow_painter.gl(), &uniforms);
    });
    let callback = egui::PaintCallback { rect, callback : Arc::new(cb) };
    painter.add(callback);
    //// !!!! evil vile code (end)
    
    let x = lerp(box_start.x, box_end.x, s);
    let y = lerp(box_start.y, box_end.y, 1.0 - v);
    
    let white = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
    let black = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255));
    
    painter.circle_stroke([x, y].into(), 4.0, white);
    painter.circle_stroke([x, y].into(), 5.0, black);
    
    let mut h_points = [
        [ring_inner_radius, 3.0], [ring_outer_radius, 3.0],
        [ring_inner_radius, -3.0], [ring_outer_radius, -3.0],
    ];
    let mut xform = Transform::ident();
    xform.rotate(h + 120.0 + 90.0);
    xform.translate([sv_box.center().x, sv_box.center().y]);
    xform_points(&xform, &mut h_points);
    
    draw_doubled(&painter, &[&[h_points[0], h_points[1], h_points[3], h_points[2], h_points[0]]]);
    
    response
}

pub (crate) fn bar_picker(ui: &mut egui::Ui, app : &mut crate::Warpainter, small : bool, glsl_mode : f32, glsl_dat : [f32; 4], float : &mut f32) -> egui::Response
{
    let height = 19.0;
    
    let input = ui.input(|input| input.clone());
    let _time = input.time as f32;
    
    let avail = ui.available_size();
    let mut least = avail.x.min(ui.available_size_before_wrap().x) as usize;
    if small
    {
        least = least.clamp(0, 100);
    }
    let least_f = least as f32;
    
    let least_vec2 = [least as f32, height].into();
    let response = ui.allocate_response(least_vec2, egui::Sense::click_and_drag());
    
    if !app.loaded_shaders
    {
        return response;
    }
    
    let box_start = response.rect.min;
    let box_end = response.rect.max;
    let sv_box : egui::Rect = [box_start, box_end].into();
    
    // deliberately render old version of value so app level clamping applies
    let oldfloat = *float;
    
    if response.is_pointer_button_down_on()
    {
        if let Some(drag_origin) = input.pointer.press_origin()
        {
            if sv_box.contains(drag_origin)
            {
                // FIXME use input.pointer.interact_pos instead
                let pos = response.interact_pointer_pos().unwrap() - box_start.to_vec2();
                *float = pos.x / least as f32;
            }
        }
    }
    
    // do rendering
    let painter = ui.painter_at(response.rect);
    
    let asdf : egui::Vec2 = [2.0, 2.0].into();
    let mut rect : egui::Rect = [[2.0, 2.0].into(), least_vec2.to_pos2() - asdf].into();
    rect = rect.translate(response.rect.min.to_vec2());
    
    //// !!!! evil vile code
    let uniforms = [
        ("width", rect.width()),
        ("height", rect.height()),
        ("funvalue", oldfloat),
        ("glsl_mode", glsl_mode),
        ("dat_0", glsl_dat[0]),
        ("dat_1", glsl_dat[1]),
        ("dat_2", glsl_dat[2]),
        ("dat_3", glsl_dat[3]),
    ];
    let colorpicker_shader = Arc::clone(app.shaders.get("funbar").unwrap());
    let cb = egui_glow::CallbackFn::new(move |_info, glow_painter|
    {
        colorpicker_shader.lock().render(glow_painter.gl(), &uniforms);
    });
    let callback = egui::PaintCallback { rect, callback : Arc::new(cb) };
    painter.add(callback);
    //// !!!! evil vile code (end)
    
    let mut h_points = [
        [-2.0,  height-2.0], [1.0,  height-2.0],
        [-2.0,         1.0], [1.0,         1.0],
    ];
    let mut xform = Transform::ident();
    xform.translate([sv_box.min.x, sv_box.min.y]);
    xform.translate([(oldfloat * (least_f - 6.0)).round() + 3.5, 0.5]);
    xform_points(&xform, &mut h_points);
    
    draw_doubled_smaller(&painter, &[&[h_points[0], h_points[1], h_points[3], h_points[2], h_points[0]]]);
    
    response
}

/*
// TODO: layer list widget
pub (crate) fn layer(ui: &mut egui::Ui, layer : &Layer)
{
    let input = ui.input().clone();
}
*/