#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

mod warimage;
mod transform;
mod widgets;
mod canvas;
mod gizmos;

use warimage::*;
use transform::*;
use widgets::*;
use canvas::*;

struct Warpaint
{
    layers : Vec<String>,
    image : Image,
    image_preview : Option<egui::TextureHandle>,
    xform : Transform,
    debug_text : Vec<String>,
}

impl Default for Warpaint
{
    fn default() -> Self
    {
        let img = image::io::Reader::open("grass4x4plus.png").unwrap().decode().unwrap().to_rgba8();
        let img = Image::from_rgbaimage(&img);
        
        Self {
            layers: vec! (
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
impl Warpaint
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

impl eframe::App for Warpaint
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame)
    {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
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
                
                ui.add(|ui : &mut egui::Ui| color_picker(ui, self));
            });
        });
        egui::CentralPanel::default().show(ctx, |ui|
        {
            ui.add(|ui : &mut egui::Ui| canvas(ui, self));
        });
    }
}

fn main()
{
    let mut options = eframe::NativeOptions::default();
    options.follow_system_theme = false;
    options.default_theme = eframe::Theme::Light;
    options.initial_window_size = Some([1280.0, 720.0].into());
    eframe::run_native
    (
        "My egui App",
        options,
        Box::new(|_| Box::new(Warpaint::default())),
    );
}
