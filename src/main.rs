//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#![windows_subsystem = "console"]

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
    fn edits_inplace(&self) -> bool; // whether the layer gets a full layer copy or a blank layer that gets composited on top
    fn is_brushlike(&self) -> bool; // ctrl is color picker, otherwise tool-contolled
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

fn draw_line_no_start_float(image : &mut Image, mut from : [f32; 2], mut to : [f32; 2], color : [f32; 4])
{
    from[0] = from[0].floor();
    from[1] = from[1].floor();
    to[0] = to[0].floor();
    to[1] = to[1].floor();
    let diff = vec_sub(&from, &to);
    let max = diff[0].abs().max(diff[1].abs());
    for i in 1..=max as usize
    {
        let amount = i as f32 / max;
        let coord = vec_lerp(&from, &to, amount);
        image.set_pixel_float(coord[0].round() as isize, coord[1].round() as isize, color);
    }
}
fn draw_line_no_start(image : &mut Image, from : [f32; 2], to : [f32; 2], color : [u8; 4])
{
    draw_line_no_start_float(image, from, to, px_to_float(color))
}

impl Tool for Pencil
{
    fn think(&mut self, app : &mut crate::Warpaint, new_input : &CanvasInputState)
    {
        if new_input.held[0] && !self.prev_input.held[0]
        {
            app.begin_edit(self.edits_inplace());
        }
        if new_input.held[0]
        {
            let prev_coord = self.prev_input.canvas_mouse_coord;
            let coord = new_input.canvas_mouse_coord;
            
            app.debug(format!("{:?}", coord));
            let color = app.main_color_rgb;
            if let Some(mut image) = app.get_editing_layer()
            {
                if !self.prev_input.held[0]
                {
                    image.set_pixel_float(coord[0] as isize, coord[1] as isize, color);
                }
                else if prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                {
                    draw_line_no_start_float(image, prev_coord, coord, color);
                }
            }
        }
        else if self.prev_input.held[0]
        {
            app.commit_edit();
        }
        if new_input.held[1] && !self.prev_input.held[1]
        {
            app.cancel_edit();
        }
        
        self.prev_input = new_input.clone();
    }
    fn edits_inplace(&self) -> bool
    {
        true
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, app : &crate::Warpaint, focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = self.prev_input.canvas_mouse_coord;
        pos[0] -= app.image.width as f32 / 2.0;
        pos[1] -= app.image.height as f32 / 2.0;
        let mut gizmo = BrushGizmo { x : pos[0].floor() + 0.5, y : pos[1].floor() + 0.5, r : 0.5 };
        Some(Box::new(gizmo))
    }
}

struct Warpaint
{
    layers : Vec<String>,
    image : Image,
    
    edit_is_direct : bool,
    editing_image : Option<Image>,
    
    image_preview : Option<egui::TextureHandle>,
    xform : Transform,
    debug_text : Vec<String>,
    
    main_color_rgb : [f32; 4],
    main_color_hsv : [f32; 4],
    sub_color_rgb : [f32; 4],
    sub_color_hsv : [f32; 4],
    
    tools : Vec<Box<dyn Tool>>,
    curr_tool : usize,
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
            
            edit_is_direct : false,
            editing_image : None,
            
            image_preview : None,
            xform : Transform::ident(),
            debug_text : vec!(),
            
            main_color_rgb : [0.0, 0.0, 0.0, 1.0],
            main_color_hsv : [0.0, 0.0, 0.0, 1.0],
            sub_color_rgb : [1.0, 1.0, 1.0, 1.0],
            sub_color_hsv : [1.0, 1.0, 1.0, 1.0],
            
            tools : vec!(Box::new(crate::Pencil::new())),
            curr_tool : 0,
        }
    }
}

impl Warpaint
{
    fn tool_think(&mut self, inputstate : &CanvasInputState)
    {
        if self.curr_tool < self.tools.len()
        {
            let mut tool = self.tools.remove(self.curr_tool);
            tool.think(self, inputstate);
            self.tools.insert(self.curr_tool, tool);
        }
    }
    fn get_tool(&self) -> Option<&Box<dyn Tool>>
    {
        self.tools.get(self.curr_tool)
    }
}

impl Warpaint
{
    fn begin_edit(&mut self, inplace : bool)
    {
        self.edit_is_direct = inplace;
        if inplace
        {
            self.editing_image = Some(self.image.clone());
        }
        else
        {
            self.editing_image = Some(self.image.blank_with_same_size());
        }
    }
    
    fn get_editing_layer<'a>(&'a mut self) -> Option<&'a mut Image>
    {
        (&mut self.editing_image).as_mut()
    }
    fn get_temp_edit_image(&mut self) -> Image
    {
        if self.edit_is_direct
        {
            if let Some(image) = &self.editing_image
            {
                image.clone()
            }
            else
            {
                self.image.clone()
            }
        }
        else
        {
            if let Some(image) = &self.editing_image
            {
                let mut r = self.image.clone();
                r.blend_from(image);
                r
            }
            else
            {
                self.image.clone()
            }
        }
    }
    fn commit_edit(&mut self)
    {
        self.debug("Committing edit");
        self.image = self.get_temp_edit_image();
        self.editing_image = None;
        self.edit_is_direct = false;
    }
    fn cancel_edit(&mut self)
    {
        self.editing_image = None;
        self.edit_is_direct = false;
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
        let img = self.get_temp_edit_image().to_egui();
        match &mut self.image_preview
        {
            Some(texhandle) =>
            {
                let img = egui::ImageData::Color(img);
                let filter = if self.xform.get_scale() >= 0.97
                {
                    egui::TextureFilter::Nearest
                }
                else
                {
                    egui::TextureFilter::Linear
                };
                texhandle.set(img, filter);
            }
            None =>
            {
                let img = ctx.load_texture(
                    "my-image",
                    img,
                    egui::TextureFilter::Nearest
                );
                
                self.image_preview = Some(img);
            }
        }
    }
    
    fn debug<T : ToString>(&mut self, text : T)
    {
        self.debug_text.push(text.to_string());
    }
}
impl Warpaint
{
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
