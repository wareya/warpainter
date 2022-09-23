
use eframe::egui;
use crate::warimage::*;
use crate::transform::*;

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
    
    let mut img = Image::blank(least, least);
    let h = (time*128.0) % 360.0;
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
                let r = (y_mid.atan2(x_mid) / std::f32::consts::PI * 180.0 + 360.0 + 150.0)%360.0;
                let p = (1.0 - mid_dist).abs().min((1.0 - ring_size - mid_dist).abs())*2.0;
                let a = (p*least_f*ring_size/1.2).clamp(0.0, 1.0);
                let b = (p*least_f*ring_size/1.2 - 0.5).clamp(0.0, 1.0);
                img.set_pixel(x as isize, y as isize, px_to_int(hsv_to_rgb([r, 0.9, b, a])));
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
    let least_vec2 = [least as f32, least as f32].into();
    let mut response = ui.allocate_response(least_vec2, egui::Sense::click_and_drag());
    
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
    
    response
}