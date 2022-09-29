//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// always show console, because still in early development
#![windows_subsystem = "console"]
// not useful while prototyping
#![allow(dead_code)]

extern crate alloc;

use eframe::egui;
use alloc::sync::Arc;
use egui::mutex::Mutex;
use eframe::egui_glow::glow;

mod warimage;
mod transform;
mod widgets;
mod canvas;
mod gizmos;
mod tools;
mod layers;
mod quadrender;
mod vecmap;

use warimage::*;
use transform::*;
use widgets::*;
use canvas::*;
use tools::*;
use layers::*;
use quadrender::*;
use vecmap::*;

struct Warpainter
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
    current_tool : usize,
    
    loaded_shaders : bool,
    shaders : VecMap<&'static str, Arc<Mutex<ShaderQuad>>>,
    
    loaded_icons : bool,
    icons : VecMap<&'static str, egui::TextureHandle>,
}

impl Default for Warpainter
{
    fn default() -> Self
    {
        let img = image::io::Reader::open("grass4x4plus.png").unwrap().decode().unwrap().to_rgba8();
        let img = Image::from_rgbaimage(&img);
        let canvas_width = img.width;
        let canvas_height = img.height;
        
        let mut root_layer = Layer::new_group("___root___");
        root_layer.uuid = 0;
        let image_layer = Layer::new_layer_from_image("New Layer", img);
        let image_layer_uuid = image_layer.uuid;
        root_layer.children = vec!(image_layer);
        
        let ret = Self {
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
            
            tools : vec!(
                Box::new(Pencil::new()),
                Box::new(Fill::new()),
            ),
            current_tool : 0,
            
            loaded_shaders : false,
            shaders : VecMap::new(),
            
            loaded_icons : false,
            icons : VecMap::new(),
        };
        
        ret
    }
}

impl Warpainter
{
    fn load_shaders(&mut self, frame : &mut eframe::Frame)
    {
        if self.loaded_shaders
        {
            return;
        }
        self.loaded_shaders = true;
        
        let colorpicker_shader = ShaderQuad::new(frame.gl().unwrap(), Some(include_str!("color_picker.glsl"))).unwrap();
        self.shaders.insert("colorpicker", Arc::new(Mutex::new(colorpicker_shader)));
        
        let colorpicker_shader = ShaderQuad::new(frame.gl().unwrap(), Some(include_str!("canvas_background.glsl"))).unwrap();
        self.shaders.insert("canvasbackground", Arc::new(Mutex::new(colorpicker_shader)));
    }
    fn load_icons(&mut self, ctx : &egui::Context)
    {
        if self.loaded_icons
        {
            return;
        }
        self.loaded_icons = true;
        
        let stuff = [
            ("new layer",           include_bytes!("icons/new layer.png")           .to_vec()),
            ("delete layer",        include_bytes!("icons/delete layer.png")        .to_vec()),
            ("duplicate layer",     include_bytes!("icons/duplicate layer.png")     .to_vec()),
            ("new group",           include_bytes!("icons/new group.png")           .to_vec()),
            ("into group",          include_bytes!("icons/into group.png")          .to_vec()),
            ("transfer down",       include_bytes!("icons/transfer down.png")       .to_vec()),
            ("merge down",          include_bytes!("icons/merge down.png")          .to_vec()),
            ("lock",                include_bytes!("icons/lock.png")                .to_vec()),
            ("lock alpha",          include_bytes!("icons/lock alpha.png")          .to_vec()),
            ("clipping mask",       include_bytes!("icons/clipping mask.png")       .to_vec()),
            ("move layer up",       include_bytes!("icons/move layer up.png")       .to_vec()),
            ("move layer down",     include_bytes!("icons/move layer down.png")     .to_vec()),
            
            ("tool pencil",         include_bytes!("icons/tool pencil.png")         .to_vec()),
            ("tool fill",           include_bytes!("icons/tool fill.png")           .to_vec()),
        ];
        for thing in stuff
        {
            let img = image::io::Reader::new(std::io::Cursor::new(&thing.1[..])).with_guessed_format().unwrap().decode().unwrap().to_rgba8();
            let img = Image::from_rgbaimage(&img).to_egui();
            let img = ctx.load_texture(
                "my-image",
                img,
                egui::TextureFilter::Nearest
            );
            self.icons.insert(thing.0, img);
        }
    }
}

impl Warpainter
{
    fn load_from_img(&mut self, img : Image)
    {
        self.layers = Layer::new_group("___root___");
        
        self.canvas_width = img.width;
        self.canvas_height = img.height;
        
        let image_layer = Layer::new_layer_from_image("New Layer", img);
        let image_layer_uuid = image_layer.uuid;
        
        self.layers.children = vec!(image_layer);
        self.current_layer = image_layer_uuid;
    }
}

impl Warpainter
{
    fn tool_think(&mut self, inputstate : &CanvasInputState)
    {
        if self.current_tool < self.tools.len()
        {
            let mut tool = self.tools.remove(self.current_tool);
            tool.think(self, inputstate);
            self.tools.insert(self.current_tool, tool);
        }
    }
    fn get_tool(&self) -> Option<&Box<dyn Tool>>
    {
        self.tools.get(self.current_tool)
    }
}

impl Warpainter
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
    fn flatten<'a>(&'a mut self) -> &'a Image
    {
        if let Some(override_image) = self.get_temp_edit_image()
        {
            self.layers.flatten_as_root(self.canvas_width, self.canvas_height, Some(self.current_layer), Some(&override_image))
        }
        else
        {
            self.layers.flatten_as_root(self.canvas_width, self.canvas_height, None, None)
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
                            r.blend_from(edit_image, 1.0); // FIXME use drawing opacity / brush alpha
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
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.dirtify_all();
        }
        
        self.editing_image = None;
        self.edit_is_direct = false;
    }
    fn cancel_edit(&mut self)
    {
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.dirtify_all();
        }
        self.editing_image = None;
        self.edit_is_direct = false;
    }
    fn mark_current_layer_dirty(&mut self, rect : [[f32; 2]; 2])
    {
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.dirtify_rect(rect);
        }
    }
}


impl Warpainter
{
    fn zoom(&mut self, amount : f32)
    {
        let mut log_zoom = self.xform.get_scale().max(0.01).log(2.0);
        let old_zoom = (log_zoom*2.0).round()/2.0;
        
        log_zoom += amount;
        
        let mut new_zoom = (log_zoom*2.0).round()/2.0;
        if new_zoom == old_zoom
        {
            new_zoom = log_zoom;
        }
        new_zoom = new_zoom.clamp(-8.0, 8.0);
        self.xform.set_scale(2.0_f32.powf(new_zoom));
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
impl Warpainter
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

impl Warpainter
{
    fn new_layer(&mut self)
    {
        let layer = Layer::new_layer("New Layer", self.canvas_width, self.canvas_height);
        // FIXME use visit_layer_parent_mut
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
            if parent.children[i].is_drawable()
            {
                parent.children.insert(i, layer);
            }
            else
            {
                parent.children[i].children.insert(0, layer);
            }
        }
        else
        {
            self.current_layer = layer.uuid;
            self.layers.children.push(layer);
        }
    }
    fn delete_current_layer(&mut self)
    {
        // FIXME use visit_layer_parent_mut
        let total_count = self.layers.count();
        if let Some(layer) = self.layers.find_layer(self.current_layer)
        {
            if layer.count()+1 >= total_count
            {
                return;
            }
        }
        else
        {
            return;
        }
        let mut new_uuid = self.layers.uuid_of_next(self.current_layer);
        self.debug(format!("{} then {:?}", self.current_layer, new_uuid));
        if new_uuid.is_none()
        {
            self.debug("fallback...");
            new_uuid = self.layers.uuid_of_prev(self.current_layer);
        }
        if let Some(new_uuid) = new_uuid
        {
            self.layers.delete_layer(self.current_layer);
            self.current_layer = new_uuid;
        }
    }
}

impl eframe::App for Warpainter
{
    fn update(&mut self, ctx : &egui::Context, frame : &mut eframe::Frame)
    {
        self.load_icons(ctx);
        self.load_shaders(frame);
        
        egui::TopBottomPanel::top("Menu Bar").show(ctx, |ui|
        {
            egui::menu::bar(ui, |ui|
            {
                ui.menu_button("File", |ui|
                {
                    if ui.button("Open...").clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Supported Image Formats",
                                &["png", "jpg", "jpeg", "gif", "bmp", "tga", "tiff", "webp", "ico", "pnm", "pbm", "ppm", "avif", "dds"])
                            //.add_filter("Warpainter Project",
                            //    &["wrp"])
                            .pick_file()
                        {
                            self.cancel_edit();
                            
                            // FIXME handle error
                            let img = image::io::Reader::open(path).unwrap().decode().unwrap().to_rgba8();
                            let img = Image::from_rgbaimage(&img);
                            self.load_from_img(img);
                            self.update_canvas_preview(ctx);
                            
                            ui.close_menu();
                        }
                    }
                    if ui.button("Save Copy...").clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PNG", &["png"])
                            //.add_filter("Warpainter Project",
                            //    &["wrp"])
                            .save_file()
                        {
                            self.cancel_edit();
                            
                            let img = self.flatten().to_imagebuffer();
                            // FIXME handle error
                            img.save(path).unwrap();
                            
                            ui.close_menu();
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
        
        egui::SidePanel::right("RightPanel").show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                let focused_outline = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 255, 255, 255));
                if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                {
                    egui::ComboBox::from_id_source("blend_mode_dropdown")
                        .selected_text(&layer.blend_mode)
                        .show_ui(ui, |ui|
                    {
                        ui.selectable_value(&mut layer.blend_mode, "Normal".to_string(), "Normal");
                        ui.selectable_value(&mut layer.blend_mode, "Multiply".to_string(), "Multiply");
                    });
                    
                    let mut opacity = layer.opacity * 100.0;
                    ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).clamp_to_range(true));
                    if layer.opacity * 100.0 != opacity
                    {
                        layer.dirtify_all();
                    }
                    layer.opacity = opacity/100.0;
                }
                else
                {
                    egui::ComboBox::from_id_source("blend_mode_dropdown").selected_text("").show_ui(ui, |_ui|{});
                    
                    let mut opacity = 0.0;
                    ui.add_enabled(false, egui::Slider::new(&mut opacity, 0.0..=100.0).clamp_to_range(true));
                    
                }
        
                macro_rules! add_button { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                        $ui.add(egui::widgets::ImageButton::new(self.icons.get($icon).unwrap().id(), [18.0, 18.0]).selected($selected))
                           .on_hover_text($tooltip)
                } }
                macro_rules! add_button_disabled { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                        $ui.add_enabled(false, egui::widgets::ImageButton::new(self.icons.get($icon).unwrap().id(), [18.0, 18.0]).selected($selected))
                           .on_hover_text($tooltip)
                } }
                
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true), |ui|
                {
                    ui.spacing_mut().item_spacing = [1.0, 0.0].into();
                    ui.spacing_mut().button_padding = [0.0, 0.0].into();
                    if add_button_disabled!(ui, "clipping mask", "Toggle Clipping Mask", false).clicked()
                    {
                        // FIXME/TODO
                    }
                    if add_button_disabled!(ui, "lock", "Toggle Layer Lock", false).clicked()
                    {
                        // FIXME/TODO
                    }
                    if add_button_disabled!(ui, "lock alpha", "Toggle Alpha Lock", false).clicked()
                    {
                        // FIXME/TODO
                    }
                });
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true), |ui|
                {
                    ui.spacing_mut().item_spacing = [1.0, 0.0].into();
                    ui.spacing_mut().button_padding = [0.0, 0.0].into();
                    if add_button!(ui, "new layer", "New Layer", false).clicked()
                    {
                        self.new_layer();
                    }
                    if add_button!(ui, "new group", "New Group", false).clicked()
                    {
                        self.layers.add_group(self.current_layer);
                    }
                    if add_button!(ui, "into group", "Into New Group", false).clicked()
                    {
                        self.layers.into_group(self.current_layer);
                    }
                    if add_button_disabled!(ui, "duplicate layer", "Duplicate Layer", false).clicked()
                    {
                        // FIXME/TODO
                    }
                    if add_button!(ui, "move layer up", "Move Layer Up", false).clicked()
                    {
                        self.layers.move_layer_up(self.current_layer);
                    }
                    if add_button!(ui, "move layer down", "Move Layer Down", false).clicked()
                    {
                        self.layers.move_layer_down(self.current_layer);
                    }
                    if add_button_disabled!(ui, "transfer down", "Transfer Down", false).clicked()
                    {
                        // FIXME/TODO
                    }
                    if add_button_disabled!(ui, "merge down", "Merge Down", false).clicked()
                    {
                        // FIXME/TODO
                    }
                    if add_button!(ui, "delete layer", "Delete Layer", false).clicked()
                    {
                        self.delete_current_layer();
                    }
                });
                
                ui.separator();
                
                let mut layer_info = vec!();
                for layer in self.layers.children.iter()
                {
                    layer.visit_layers(0, &mut |layer, depth|
                    {
                        layer_info.push((layer.name.clone(), layer.uuid, depth));
                        Some(())
                    });
                }
                for info in layer_info
                {
                    ui.horizontal(|ui|
                    {
                        ui.allocate_space([info.2 as f32 * 8.0, 0.0].into());
                        let mut button = egui::Button::new(info.0);
                        if info.1 == self.current_layer
                        {
                            button = button.stroke(focused_outline);
                        }
                        if ui.add(button).clicked()
                        {
                            self.current_layer = info.1;
                        }
                    });
                }
            });
        });
        egui::SidePanel::left("ToolPanel").min_width(22.0).default_width(22.0).show(ctx, |ui|
        {
            macro_rules! add_button { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                    $ui.add(egui::widgets::ImageButton::new(self.icons.get($icon).unwrap().id(), [22.0, 22.0]).selected($selected))
                       .on_hover_text($tooltip)
            } }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                ui.spacing_mut().button_padding = [0.0, 0.0].into();
                if add_button!(ui, "tool pencil", "Pencil Tool", self.current_tool == 0).clicked()
                {
                    self.current_tool = 0;
                }
                if add_button!(ui, "tool fill", "Fill Tool", self.current_tool == 1).clicked()
                {
                    self.current_tool = 1;
                }
            });
        });
        egui::SidePanel::left("ToolSettings").show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                ui.add(|ui : &mut egui::Ui| color_picker(ui, self));
            });
        });
        
        egui::TopBottomPanel::bottom("DebugText").resizable(true).max_height(150.0).show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui|
            {
                if self.debug_text.len() > 500
                {
                    self.debug_text.drain(0..self.debug_text.len()-500);
                }
                let mut text = self.debug_text.join("\n");
                ui.add_enabled(false, egui::TextEdit::multiline(&mut text).hint_text("debug output"));
            });
        });
        
        let frame = egui::Frame {
            inner_margin: egui::style::Margin::same(0.0),
            rounding: egui::Rounding::none(),
            fill: ctx.style().visuals.window_fill(),
            stroke: Default::default(),
            ..Default::default()
        };
        
        egui::CentralPanel::default().frame(frame).show(ctx, |ui|
        {
            ui.spacing_mut().window_margin = 0.0.into();
            ui.add(|ui : &mut egui::Ui| canvas(ui, self));
        });
        
        // DON'T USE; BUGGY / REENTRANT / CAUSES CRASH (in egui/eframe 0.19.0 at least)
        //ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
    fn on_exit(&mut self, gl : Option<&glow::Context>)
    {
        if let Some(gl) = gl
        {
            for shader in self.shaders.values()
            {
                shader.lock().delete_data(gl);
            }
        }
    }
}

fn main()
{
    let mut options = eframe::NativeOptions::default();
    
    // eframe 0.19.0 is borked on windows 10, the window flickers violently when you resize it, flashing white
    // this is a seizure hazard when using the dark theme, so force the light theme instead
    options.follow_system_theme = false;
    options.default_theme = eframe::Theme::Light;
    
    options.initial_window_size = Some([1280.0, 720.0].into());
    eframe::run_native (
        "Warpainter",
        options,
        Box::new(|_| Box::new(Warpainter::default())),
    );
}
