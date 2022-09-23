#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main()
{
    let mut options = eframe::NativeOptions::default();
    options.follow_system_theme = false;
    options.default_theme = eframe::Theme::Light;
    options.initial_window_size = Some([1280.0, 720.0].into());
    eframe::run_native (
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

mod warimage;
mod transform;

use warimage::*;
use transform::*;


struct MyApp
{
    layers : Vec<String>,
    image : Image,
    image_preview : Option<egui::TextureHandle>,
    xform : Transform,
    debug_text : Vec<String>,
}

impl Default for MyApp
{
    fn default() -> Self
    {
        let img = image::io::Reader::open("grass4x4plus.png").unwrap().decode().unwrap().to_rgba8();
        let img = Image::from_rgbaimage(&img);
        
        Self {
            layers: vec!(
                "New Layer 3".to_string(),
                "New Layer 2".to_string(),
                "New Layer 1".to_string()
            ),
            image : img,
            image_preview : None,
            xform : Transform::ident(),
            debug_text : vec!(),
        }
    }
}
impl MyApp
{
    fn zoom(&mut self, amount : f32)
    {
        let mut log_zoom = self.xform.get_scale().max(0.01).log(2.0);
        let mut old_zoom = (log_zoom*2.0).round()/2.0;
        
        log_zoom += amount;
        
        let mut new_zoom = (log_zoom*2.0).round()/2.0;
        if new_zoom == old_zoom
        {
            new_zoom = log_zoom;
        }
        log_zoom = log_zoom.clamp(-16.0, 16.0);
        self.xform.set_scale(2.0_f32.powf(log_zoom));
    }
    fn debug(&mut self, text : String)
    {
        self.debug_text.push(text);
    }
}

impl eframe::App for MyApp
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame)
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        match &mut self.image_preview
        {
            Some(texhandle) =>
            {
                let img2 = self.image.to_egui();
                let img2 = egui::ImageData::Color(img2);
                let filter = if self.xform.get_scale() >= 1.0
                {
                    egui::TextureFilter::Nearest
                }
                else
                {
                    egui::TextureFilter::Linear
                };
                texhandle.set(img2, filter);
            }
            _ => {}
        }
        if self.image_preview.is_none()
        {
            let img2 = self.image.to_egui();
            let img2 = ctx.load_texture(
                "my-image",
                img2,
                egui::TextureFilter::Nearest
            );
            
            self.image_preview = Some(img2);
        }
        
        egui::Window::new("Debug Text").show(ctx, |ui|
        {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui|
            {
                if self.debug_text.len() > 500
                {
                    self.debug_text.drain(0..self.debug_text.len()-500);
                }
                ui.label(&self.debug_text.join("\n"));
            });
        });
        
        egui::SidePanel::right("RightPanel").show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                for layer in &self.layers
                {
                    ui.label(layer);
                }
            });
        });
        egui::SidePanel::left("ToolPanel").min_width(64.0).default_width(64.0).show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                if ui.button("Pencil").clicked()
                {
                    self.debug(format!("pressed pencil"));
                }
                if ui.button("Fill").clicked()
                {
                    self.debug(format!("pressed fill"));
                }
            });
        });
        egui::SidePanel::left("ToolSettings").show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                let input = ui.input().clone();
                let time = input.time as f32;
                
                ui.add(|ui: &mut egui::Ui| -> egui::Response
                {
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
                    let tex = ctx.load_texture(
                        "colorpalette",
                        img.to_egui(),
                        egui::TextureFilter::Nearest
                    );
                    
                    let least_vec2 = [least as f32, least as f32].into();
                    let mut response = ui.allocate_response(least_vec2, egui::Sense::click_and_drag());
                    
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
                });
            });
        });
        egui::CentralPanel::default().show(ctx, |ui|
        {
            ui.heading("My egui Application");
            ui.label("todo");
            ui.add(|ui: &mut egui::Ui| -> egui::Response
            {
                let input = ui.input().clone();
                
                let time = input.time;
                let delta = input.unstable_dt;
                let mouse_motion = input.pointer.delta();
                let mouse_position = input.pointer.interact_pos().unwrap_or_default();
                let mouse_scroll = input.scroll_delta.y;
                
                let pressed = (
                    input.pointer.button_clicked(egui::PointerButton::Primary),
                    input.pointer.button_clicked(egui::PointerButton::Secondary),
                    input.pointer.button_clicked(egui::PointerButton::Middle),
                    input.pointer.button_clicked(egui::PointerButton::Extra1),
                    input.pointer.button_clicked(egui::PointerButton::Extra2),
                );
                let held = (
                    input.pointer.button_down(egui::PointerButton::Primary),
                    input.pointer.button_down(egui::PointerButton::Secondary),
                    input.pointer.button_down(egui::PointerButton::Middle),
                    input.pointer.button_down(egui::PointerButton::Extra1),
                    input.pointer.button_down(egui::PointerButton::Extra2),
                );
                let released = (
                    input.pointer.button_released(egui::PointerButton::Primary),
                    input.pointer.button_released(egui::PointerButton::Secondary),
                    input.pointer.button_released(egui::PointerButton::Middle),
                    input.pointer.button_released(egui::PointerButton::Extra1),
                    input.pointer.button_released(egui::PointerButton::Extra2),
                );
                
                if input.key_pressed(egui::Key::Num0)
                {
                    self.xform = Transform::ident();
                }
                for _ in 0..input.num_presses(egui::Key::Num2)
                {
                    self.xform.rotate(15.0);
                    self.debug(format!("{}", self.xform.get_scale()));
                    self.debug(format!("{}", self.xform.get_rotation()));
                    self.debug(format!("{:?}", self.xform.get_translation()));
                }
                for _ in 0..input.num_presses(egui::Key::Num1)
                {
                    self.xform.rotate(-15.0);
                    self.debug(format!("{}", self.xform.get_scale()));
                    self.debug(format!("{}", self.xform.get_rotation()));
                    self.debug(format!("{:?}", self.xform.get_translation()));
                }
                if held.2
                {
                    self.xform.translate([mouse_motion.x, mouse_motion.y]);
                }
                // FIXME: enforce that the canvas does not go offscreen
                // (idea: if center of screen is not inside of canvas, prevent center of canvas from going past edges)
                
                
                // todo
                let mut response = ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());
                
                if mouse_scroll != 0.0
                {
                    let offset = mouse_position - response.rect.center();
                    self.xform.translate([-offset.x, -offset.y]);
                    self.zoom(mouse_scroll/128.0);
                    self.xform.translate([ offset.x,  offset.y]);
                }
                
                if held.0
                {
                    let offset = mouse_position;
                    let mut coord = [offset.x, offset.y];
                    
                    let mut xform = self.xform.clone();
                    let center = response.rect.center();
                    xform.translate([center.x, center.y]);
                    xform = xform.inverse();
                    
                    coord = &xform * &coord;
                    coord[0] += self.image.width as f32 / 2.0;
                    coord[1] += self.image.height as f32 / 2.0;
                    
                    self.debug(format!("{:?}", coord));
                    self.image.set_pixel(coord[0] as isize, coord[1] as isize, [0,0,0,255]);
                }
                
                let tex = self.image_preview.as_ref().unwrap();
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
                for vert in mesh.vertices.iter_mut()
                {
                    let mut xform = self.xform.clone();
                    let center = response.rect.center();
                    xform.translate([center.x, center.y]);
                    let new = &xform * &[vert.pos[0], vert.pos[1]];
                    vert.pos.x = new[0];
                    vert.pos.y = new[1];
                }
                
                let painter = ui.painter_at(response.rect);
                painter.add(egui::Shape::mesh(mesh));
                
                
                response
            });
        });
    }
}