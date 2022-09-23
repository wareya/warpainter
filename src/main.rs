//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

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
use gizmos::*;

trait Tool
{
    fn think(&mut self, app : &mut crate::Warpaint, new_input : &CanvasInputState);
    fn is_brushlike(&self) -> bool;
    fn get_gizmo(&self, app : &crate::Warpaint, focused : bool) -> Option<Box<dyn Gizmo>>;
}

struct Pencil
{
    size : f32,
    prev_input : CanvasInputState,
}

impl Pencil
{
    fn new() -> Self
    {
        Pencil { size : 1.0, prev_input : CanvasInputState::default() }
    }
}

impl Tool for Pencil
{
    fn think(&mut self, app : &mut crate::Warpaint, new_input : &CanvasInputState)
    {
        self.prev_input = new_input.clone();
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, app : &crate::Warpaint, focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let pos = self.prev_input.canvas_mouse_coord;
        let mut gizmo = BrushGizmo { x : pos[0].floor() + 0.5, y : pos[1].floor() + 0.5, r : 0.5 };
        Some(Box::new(gizmo))
        //gizmo.draw(ui, app, &mut response, &painter);
    }
}

struct Warpaint
{
    layers : Vec<String>,
    image : Image,
    image_preview : Option<egui::TextureHandle>,
    xform : Transform,
    debug_text : Vec<String>,
    
    main_color_rgb : [f32; 4],
    main_color_hsv : [f32; 4],
    sub_color_rgb : [f32; 4],
    sub_color_hsv : [f32; 4],
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
            main_color_rgb : [0.0, 0.0, 0.0, 1.0],
            main_color_hsv : [0.0, 0.0, 0.0, 1.0],
            sub_color_rgb : [1.0, 1.0, 1.0, 1.0],
            sub_color_hsv : [1.0, 1.0, 1.0, 1.0],
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
    fn update_canvas_preview(&mut self, ctx : &egui::Context)
    {
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
    }
    
    fn debug(&mut self, text : String)
    {
        self.debug_text.push(text);
    }
    
    fn set_main_color_rgb8(&mut self, new : [u8; 4])
    {
        self.set_main_color_rgb(px_to_float(new));
    }
    fn set_main_color_rgb(&mut self, new : [f32; 4])
    {
        self.main_color_rgb = new.clone();
        self.main_color_hsv = rgb_to_hsv(new);
    }
    fn set_main_color_hsv8(&mut self, new : [u8; 4])
    {
        self.set_main_color_hsv(px_to_float(new));
    }
    fn set_main_color_hsv(&mut self, new : [f32; 4])
    {
        self.main_color_rgb = hsv_to_rgb(new);
        self.main_color_hsv = new;
    }
    
    fn set_sub_color_rgb8(&mut self, new : [u8; 4])
    {
        self.set_sub_color_rgb(px_to_float(new));
    }
    fn set_sub_color_rgb(&mut self, new : [f32; 4])
    {
        self.sub_color_rgb = new.clone();
        self.sub_color_hsv = rgb_to_hsv(new);
    }
    fn set_sub_color_hsv8(&mut self, new : [u8; 4])
    {
        self.set_sub_color_hsv(px_to_float(new));
    }
    fn set_sub_color_hsv(&mut self, new : [f32; 4])
    {
        self.sub_color_rgb = hsv_to_rgb(new);
        self.sub_color_hsv = new;
    }
}

impl eframe::App for Warpaint
{
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame)
    {
        egui::TopBottomPanel::top("Menu Bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui|
            {
                ui.menu_button("File", |ui|
                {
                    if ui.button("Open...").clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_file()
                        {
                            let img = image::io::Reader::open(path).unwrap().decode().unwrap().to_rgba8();
                            self.image = Image::from_rgbaimage(&img);
                            let img2 = self.image.to_egui();
                            let img2 = ctx.load_texture(
                                "my-image",
                                img2,
                                egui::TextureFilter::Nearest
                            );
                            
                            self.image_preview = Some(img2);
                        }
                    }
                });
                ui.menu_button("Edit", |ui|
                {
                    if ui.button("Undo").clicked()
                    {
                    }
                    if ui.button("Redo").clicked()
                    {
                    }
                });
                ui.menu_button("View", |ui|
                {
                    if ui.button("Zoom In").clicked()
                    {
                    }
                    if ui.button("Zoom Out").clicked()
                    {
                    }
                });
            });
        });
        
        self.update_canvas_preview(&ctx);
        
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
        
        // DON'T USE; BUGGY / REENTRANT / CAUSES CRASH (in egui 0.19.0 at least)
        //ctx.request_repaint_after(std::time::Duration::from_millis(200));
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
