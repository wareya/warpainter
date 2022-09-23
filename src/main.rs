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

enum ImageData
{
    Float(Vec<f32>),
    Int(Vec<u8>),
}

fn to_float(x : u8) -> f32
{
    (x as f32)*255.0
}
fn to_int(x : f32) -> u8
{
    (x*255.0).round().clamp(0.0, 255.0) as u8
}

impl ImageData
{
    fn new_int(w : usize, h : usize) -> Self
    {
        Self::Int(vec!(0; w*h*4))
    }
    fn to_int(&self) -> Vec<u8>
    {
        match self
        {
            Self::Int(data) => data.clone(),
            Self::Float(data) =>
            {
                let mut out = vec!(0; data.len());
                for i in 0..data.len()
                {
                    out[i] = to_int(data[i]);
                }
                out
            }
        }
    }
}

// always RGBA
struct Image
{
    width : usize,
    height : usize,
    data : ImageData,
}

impl Image
{
    fn from_rgbaimage(input : &image::RgbaImage) -> Self
    {
        let (w, h) = input.dimensions();
        let data = ImageData::new_int(w as usize, h as usize);
        let mut ret = Self { width : w as usize, height : h as usize, data };
        for y in 0..ret.height
        {
            for x in 0..ret.width
            {
                use image::Pixel;
                let px = input.get_pixel(x as u32, y as u32).0;
                ret.set_pixel(x, y, px);
            }
        }
        ret
    }
    fn to_egui(&self) -> egui::ColorImage
    {
        match &self.data
        {
            ImageData::Int(data) =>
                egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &data),
            _ =>
                egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &self.data.to_int()),
        }
    }
    fn set_pixel(&mut self, x : usize, y : usize, px : [u8; 4])
    {
        if x >= self.width || y >= self.height
        {
            return;
        }
        let index = y*self.width*4 + x*4;
        match &mut self.data
        {
            ImageData::Int(data) =>
            {
                for i in 0..4
                {
                    data[index + i] = px[i];
                }
            }
            ImageData::Float(data) =>
            {
                for i in 0..4
                {
                    data[index + i] = to_float(px[i]);
                }
            }
        }
    }
    fn set_pixel_float(&mut self, x : usize, y : usize, px : [f32; 4])
    {
        if x >= self.width || y >= self.height
        {
            return;
        }
        let index = y*self.width*4 + x*4;
        match &mut self.data
        {
            ImageData::Int(data) =>
            {
                for i in 0..4
                {
                    data[index + i] = to_int(px[i]);
                }
            }
            ImageData::Float(data) =>
            {
                for i in 0..4
                {
                    data[index + i] = px[i];
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Transform
{
    rows : [[f32; 3]; 3],
}
impl Default for Transform
{
    fn default() -> Self
    {
        Self::ident()
    }
}
impl<'a, 'b> std::ops::Mul<&'b Transform> for &'a Transform
{
    type Output = Transform;
    fn mul(self, other : &'b Transform) -> Transform
    {
        let mut out = Transform::zero();
        for row in 0..3
        {
            for col in 0..3
            {
                out.rows[row][col] = 0.0;
                for i in 0..3
                {
                    out.rows[row][col] += self.rows[row][i] * other.rows[i][col];
                }
            }
        }
        out
    }
}
impl<'a, 'b> std::ops::Mul<&'b [f32; 2]> for &'a Transform
{
    type Output = [f32; 2];
    fn mul(self, other : &'b [f32; 2]) -> [f32; 2]
    {
        let other = [other[0], other[1], 1.0];
        let mut out = [0.0, 0.0, 0.0];
        for row in 0..3
        {
            for col in 0..3
            {
                out[row] += self.rows[row][col] * other[col];
            }
        }
        [out[0], out[1]]
    }
}

fn length(vec : &[f32]) -> f32
{
    let mut r = 0.0;
    for x in vec.iter()
    {
        r += x*x;
    }
    r.sqrt()
}

impl Transform {
    fn zero() -> Self
    {
        Self { rows : [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]] }
    }
    fn ident() -> Self
    {
        Self { rows : [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] }
    }
    fn get_translation(&self) -> [f32; 2]
    {
        [self.rows[0][2], self.rows[1][2]]
    }
    // FIXME make a vector
    fn get_scale(&self) -> f32
    {
        let a = self.rows[0][0];
        let b = self.rows[0][1];
        let c = self.rows[1][0];
        let d = self.rows[1][1];
        
        let x = length(&[a, c]);
        let y = length(&[b, d]);
        x/2.0 + y/2.0
    }
    fn get_rotation(&self) -> f32
    {
        let mut d = self.clone();
        d.rows[0][2] = 0.0;
        d.rows[1][2] = 0.0;
        d.set_scale(1.0);
        
        let r = &d * &[1.0, 0.0];
        
        let psi = (r[1]).atan2(r[0]);
        
        psi / std::f32::consts::PI * 180.0
    }
    fn translate(&mut self, translation : [f32; 2])
    {
        let mut other = Self::ident();
        other.rows[0][2] = translation[0];
        other.rows[1][2] = translation[1];
        
        let new = &other * &*self;
        self.rows = new.rows;
    }
    // FIXME make a vector
    fn scale(&mut self, scale : f32)
    {
        let mut other = Self::ident();
        other.rows[0][0] = scale;
        other.rows[1][1] = scale;
        
        let new = &other * &*self;
        self.rows = new.rows;
    }
    fn set_scale(&mut self, scale : f32)
    {
        let old_scale = self.get_scale();
        if old_scale > 0.0
        {
            self.scale(1.0 / old_scale);
        }
        self.scale(scale);
    }
    fn rotate(&mut self, angle : f32)
    {
        let mut other = Self::ident();
        let _cos = (angle * std::f32::consts::PI / 180.0).cos();
        let _sin = (angle * std::f32::consts::PI / 180.0).sin();
        other.rows[0][0] =  _cos;
        other.rows[0][1] = -_sin;
        other.rows[1][0] =  _sin;
        other.rows[1][1] =  _cos;
        
        let new = &other * &*self;
        self.rows = new.rows;
    }
    fn make_uniform(&mut self)
    {
        let mut other = Self::ident();
        // FIXME / TODO
    }
    fn inverse(&self) -> Self
    {
        let mut other = Self::ident();
        
        let trans = self.get_translation();
        
        other.translate([-trans[0], -trans[1]]);
        other.scale(1.0/self.get_scale());
        other.rotate(-self.get_rotation());
        
        other
    }
}

struct MyApp
{
    layers : Vec<String>,
    image : Image,
    image_preview : Option<egui::TextureHandle>,
    xform : Transform,
    debug_text : String,
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
            debug_text : "".to_string(),
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
        self.debug_text.push_str(&text);
        self.debug_text.push_str("\n");
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
                ui.label(&self.debug_text);
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
        egui::SidePanel::left("ToolSettings").min_width(64.0).default_width(64.0).show(ctx, |ui|
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
        egui::SidePanel::left("ToolPanel").show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                for layer in &self.layers
                {
                    ui.label(layer);
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui|
        {
            ui.heading("My egui Application");
            ui.label("todo");
            ui.add(|ui: &mut egui::Ui| -> egui::Response
            {
                let input = ui.input();
                
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
                
                
                
                drop(input);
                
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
                    self.image.set_pixel(coord[0] as usize, coord[1] as usize, [0,0,0,255]);
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