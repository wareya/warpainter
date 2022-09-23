
use eframe::egui;
use crate::warimage::*;
use crate::transform::*;
use crate::gizmos::draw_doubled;

/*
pub (crate) fn alpha_picker(ui: &mut egui::Ui, app : &mut crate::Warpaint) -> egui::Response
{
    let input = ui.input().clone();
    
    let avail = ui.available_size();
    let w = avail.x;
    let h = 16.0;
    let least_f = least as f32;
*/
    
pub (crate) fn color_picker(ui: &mut egui::Ui, app : &mut crate::Warpaint) -> egui::Response
{
    let input = ui.input().clone();
    let time = input.time as f32;
    
    let avail = ui.available_size();
    let least = avail.x.min(avail.y) as usize;
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
    let mut response = ui.allocate_response(least_vec2, egui::Sense::click_and_drag());
    
    let box_start = response.rect.min + [box_margin, box_margin].into();
    let box_end   = box_start + [box_size, box_size].into();
    let sv_box : egui::Rect = [box_start, box_end].into();
    
    // do input
    
    if response.dragged()
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
                h = (pos[1].atan2(pos[0]) / std::f32::consts::PI * 180.0 + 360.0 + 150.0)%360.0;
                app.set_main_color_hsv([h, s, v, a]);
            }
        }
    }
    
    // do rendering
    
    let mut img = Image::blank(least, least);
    for y in 0..least
    {
        let y = y as f32;
        let y_mid = y/least_f*2.0 - 1.0;
        for x in 0..least
        {
            let x = x as f32;
            let x_mid = x/least_f*2.0 - 1.0;
            let mid_dist = length(&[y_mid, x_mid]);
            
            if mid_dist + ring_size > 1.0 && mid_dist < 1.0
            {
                let y_mid = y_mid / mid_dist;
                let x_mid = x_mid / mid_dist;
                
                // angle
                let h = (y_mid.atan2(x_mid) / std::f32::consts::PI * 180.0 + 360.0 + 150.0)%360.0;
                
                //distance, outline-rendering stuff
                let p = (1.0 - mid_dist).abs().min((1.0 - ring_size - mid_dist).abs())*2.0;
                let a = (p*least_f*ring_size/1.2).clamp(0.0, 1.0);
                let b = (p*least_f*ring_size/1.2 - 0.5).clamp(0.0, 1.0);
                
                img.set_pixel(x as isize, y as isize, px_to_int(hsv_to_rgb([h, 0.9, b, a])));
            }
            else if x > box_margin && x < box_margin+box_size
                 && y > box_margin && y < box_margin+box_size
            {
                let s = (x-box_margin) / box_size;
                let v = 1.0 - (y-box_margin) / box_size;
                img.set_pixel(x as isize, y as isize, px_to_int(hsv_to_rgb([h, s, v, 1.0])));
            }
            else if x > box_margin-1.0 && x < box_margin+box_size+1.0
                 && y > box_margin-1.0 && y < box_margin+box_size+1.0
            {
                img.set_pixel(x as isize, y as isize, [0, 0, 0, 192]);
            }
            else if x > box_margin-2.0 && x < box_margin+box_size+2.0
                 && y > box_margin-2.0 && y < box_margin+box_size+2.0
            {
                img.set_pixel(x as isize, y as isize, [0, 0, 0, 92]);
            }
        }
    }
    
    let tex = response.ctx.load_texture(
        "colorpalette",
        img.to_egui(),
        egui::TextureFilter::Nearest
    );
    
    let mut mesh = egui::Mesh::with_texture(tex.id());
    let mut rect : egui::Rect = [[0.0, 0.0].into(), least_vec2.to_pos2()].into();
    let uv = [[0.0, 0.0].into(), [1.0, 1.0].into()].into();
    mesh.add_rect_with_uv (
        rect.translate(response.rect.min.to_vec2()),
        uv,
        egui::Color32::WHITE
    );
    
    let painter = ui.painter_at(response.rect);
    painter.add(egui::Shape::mesh(mesh));
    
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
    
    draw_doubled(&painter, &[
        [h_points[0], h_points[1]],
        [h_points[0], h_points[2]],
        [h_points[1], h_points[3]],
        [h_points[2], h_points[3]],
    ]);
    
    response
}