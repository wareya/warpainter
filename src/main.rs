//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#![windows_subsystem = "console"]

use eframe::egui;

mod warimage;
mod transform;
mod widgets;
mod canvas;
mod gizmos;
mod tools;
mod layers;

use warimage::*;
use transform::*;
use widgets::*;
use canvas::*;
use gizmos::*;
use tools::*;
use layers::*;

struct Warpaint
{
    layers : Layer,
    current_layer : u128,
    
    canvas_width : usize,
    canvas_height : usize,
    
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
        let canvas_width = img.width;
        let canvas_height = img.height;
        
        let mut root_layer = Layer::new_group("___root___");
        let image_layer = Layer::new_layer_from_image("New Layer", img);
        let image_layer_uuid = image_layer.uuid;
        root_layer.children = vec!(image_layer);
        
        Self {
            layers : root_layer,
            current_layer : image_layer_uuid,
            
            canvas_width,
            canvas_height,
            
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
    fn load_from_img(&mut self, img : Image)
    {
        self.layers = Layer::new_group("___root___");
        
        let canvas_width = img.width;
        let canvas_height = img.height;
        
        let image_layer = Layer::new_layer_from_image("New Layer", img);
        let image_layer_uuid = image_layer.uuid;
        
        self.layers.children = vec!(image_layer);
        self.current_layer = image_layer_uuid;
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
        if let Some(layer) = self.layers.find_layer(self.current_layer)
        {
            if !layer.locked
            {
                if let Some(image) = &layer.data
                {
                    self.edit_is_direct = inplace;
                    if inplace
                    {
                        self.editing_image = Some(image.clone());
                    }
                    else
                    {
                        self.editing_image = Some(image.blank_with_same_size());
                    }
                }
            }
        }
    }
    
    fn get_editing_image<'a>(&'a mut self) -> Option<&'a mut Image>
    {
        (&mut self.editing_image).as_mut()
    }
    fn flatten(&self) -> Image
    {
        if let Some(override_image) = self.get_temp_edit_image()
        {
            self.layers.flatten(self.canvas_width, self.canvas_height, self.current_layer, Some(&override_image))
        }
        else
        {
            self.layers.flatten(self.canvas_width, self.canvas_height, 0, None)
        }
    }
    fn get_temp_edit_image(&self) -> Option<Image>
    {
        if let Some(edit_image) = &self.editing_image
        {
            if let Some(layer) = self.layers.find_layer(self.current_layer)
            {
                if !layer.locked
                {
                    if let Some(current_image) = &layer.data
                    {
                        if self.edit_is_direct
                        {
                            return Some(edit_image.clone());
                        }
                        else
                        {
                            let mut r = current_image.clone();
                            r.blend_from(edit_image);
                            return Some(r);
                        }
                    }
                }
            }
        }
        None
    }
    fn commit_edit(&mut self)
    {
        self.debug("Committing edit");
        if let Some(image) = self.get_temp_edit_image()
        {
            if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
            {
                if !layer.locked
                {
                    if let Some(current_image) = &mut layer.data
                    {
                        *current_image = image;
                    }
                }
            }
        }
        
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
        let img = self.flatten().to_egui();
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

impl Warpaint
{
    fn new_layer(&mut self)
    {
        let layer = Layer::new_layer("New Layer", self.canvas_width, self.canvas_height);
        if let Some(parent) = self.layers.find_layer_parent_mut(self.current_layer)
        {
            let mut i = 0;
            for (j, check_layer) in parent.children.iter().enumerate()
            {
                if check_layer.uuid == self.current_layer
                {
                    i = j;
                    break;
                }
            }
            self.current_layer = layer.uuid;
            parent.children.insert(i, layer);
        }
        else
        {
            self.current_layer = layer.uuid;
            self.layers.children.push(layer);
        }
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
                            let img = Image::from_rgbaimage(&img);
                            self.load_from_img(img);
                            self.update_canvas_preview(ctx);
                        }
                    }
                });
                ui.menu_button("Edit", |ui|
                {
                    if ui.button("Undo").clicked()
                    {
                        // TODO/FIXME
                    }
                    if ui.button("Redo").clicked()
                    {
                        // TODO/FIXME
                    }
                });
                ui.menu_button("View", |ui|
                {
                    if ui.button("Zoom In").clicked()
                    {
                        // TODO/FIXME (easy)
                    }
                    if ui.button("Zoom Out").clicked()
                    {
                        // TODO/FIXME (easy)
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
                ui.horizontal(|ui|
                {
                    if ui.button("+").clicked()
                    {
                        self.new_layer();
                    }
                    if ui.button("x").clicked()
                    {
                        
                    }
                    if ui.button("g").clicked()
                    {
                        
                    }
                    if ui.button("*").clicked()
                    {
                        
                    }
                    if ui.button("↑").clicked()
                    {
                        
                    }
                    if ui.button("↓").clicked()
                    {
                        
                    }
                    if ui.button("?").clicked()
                    {
                        
                    }
                });
                
                let mut layer_info = vec!();
                for layer in self.layers.children.iter()
                {
                    layer.visit_layers(0, &mut |layer : &Layer|
                    {
                        layer_info.push((layer.name.clone(), layer.uuid));
                    });
                }
                for info in layer_info
                {
                    ui.horizontal(|ui|
                    {
                        let mut button = egui::Button::new(info.0);
                        if info.1 == self.current_layer
                        {
                            button = button.stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 255, 255, 255)));
                        }
                        if ui.add(button).clicked()
                        {
                            self.current_layer = info.1;
                        }
                    });
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
