#![allow(dead_code)]

extern crate alloc;

use std::io::{Read as _, Write as _};

use eframe::egui;
use alloc::sync::Arc;
use egui::mutex::Mutex;
use egui::{Ui, SliderClamping};
use eframe::egui_glow::glow;

use serde::{Serialize, Deserialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

mod wpsd;
mod rle16;
mod wpsd_raw;
mod warimage;
mod transform;
mod widgets;
mod canvas;
mod gizmos;
mod tools;
mod layers;
mod layers_fxblend;
mod quadrender;
mod vecmap;
mod pixelmath;
mod spline;
mod wabl;
mod hwaccel;

use rle16::*;
use wpsd::*;
use warimage::*;
use transform::*;
use widgets::*;
use canvas::*;
use tools::*;
use layers::*;
use quadrender::*;
use vecmap::*;
use pixelmath::*;

use bincode::{Decode, Encode};
#[derive(Clone, Debug, Default, Decode, Encode, Serialize, Deserialize)]
struct LayerInfoChange
{
    // the bson crate doesn't support u128s.........
    uuid : u128,
    old : LayerInfo,
    new : LayerInfo,
}

#[derive(Clone, Debug, Default, Decode, Encode, Serialize, Deserialize)]
struct LayerMove
{
    uuid : Vec<u128>,
    old_parent : Vec<u128>,
    new_parent : Vec<u128>,
    old_position : Vec<usize>,
    new_position : Vec<usize>,
}
#[derive(Clone, Debug, Default, Decode, Encode, Serialize, Deserialize)]
struct LayerPaint
{
    uuid : u128,
    rect : [[usize; 2]; 2],
    old : Image<4>,
    new : Image<4>,
    mask : Image<1>,
}
#[derive(Clone, Debug, Default, Decode, Encode, Serialize, Deserialize)]
enum UndoEvent
{
    #[default]
    Null,
    LayerInfoChange(LayerInfoChange),
    LayerMove(LayerMove),
    LayerPaint(LayerPaint),
}

impl UndoEvent
{
    fn compress(&self) -> Vec<u8>
    {
        /*
        match self
        {
            //UndoEvent::Null => UndoEvent::Null,
            //UndoEvent::LayerInfoChange(x) => UndoEvent::LayerInfoChange(x),
            //UndoEvent::LayerMove(x) => UndoEvent::LayerMove(x),
            //UndoEvent::LayerPaintCompressed(x) => UndoEvent::LayerPaintCompressed(x),
            UndoEvent::LayerPaint(x) =>
            {
                if let ImageData::<4>::Int(x) = &x.old.data
                {
                    let mut compressed : Vec<u8> = Vec::new();
                    {
                        let mut bw = std::io::BufWriter::new(rle16::Compressor::new(&mut compressed));
                        let s = unsafe { std::slice::from_raw_parts(x.as_ptr() as *const u8, x.len() * 4) };
                        bw.write_all(&s).unwrap();
                    }
                    println!("{}", compressed.len());
                }
                if let ImageData::<4>::Int(x) = &x.new.data
                {
                    let mut compressed : Vec<u8> = Vec::new();
                    {
                        let mut bw = std::io::BufWriter::new(rle16::Compressor::new(&mut compressed));
                        let s = unsafe { std::slice::from_raw_parts(x.as_ptr() as *const u8, x.len() * 4) };
                        bw.write_all(&s).unwrap();
                    }
                    println!("{}", compressed.len());
                }
                if let ImageData::<1>::Int(x) = &x.mask.data
                {
                    let mut compressed : Vec<u8> = Vec::new();
                    {
                        let mut bw = std::io::BufWriter::new(rle16::Compressor::new(&mut compressed));
                        let s = unsafe { std::slice::from_raw_parts(x.as_ptr() as *const u8, x.len()) };
                        bw.write_all(&s).unwrap();
                    }
                    println!("{}", compressed.len());
                }
            }
            _ => {}
        }*/
        
        let mut compressed : Vec<u8> = Vec::new();
        {
            //struct Asdf<'a> { s : std::io::BufWriter<rle16::Compressor<&'a mut Vec<u8>>> }
            let mut asdf = std::io::BufWriter::new(rle16::Compressor::new(&mut compressed));
            //let mut asdf = Asdf { s : std::io::BufWriter::new(rle16::Compressor::new(&mut compressed)) };
            //struct Asdf<'a> { s : std::io::BufWriter<snap::write::FrameEncoder<&'a mut Vec<u8>>> }
            //let mut asdf = Asdf { s : std::io::BufWriter::new(snap::write::FrameEncoder::new(&mut compressed)) };
            //impl<'a> bincode::enc::write::Writer for Asdf<'a>
            //{
            //    fn write(&mut self, bytes : &[u8]) -> Result<(), bincode::error::EncodeError>
            //    {
            //        self.s.write(bytes).unwrap();
            //        Ok(())
            //    }
            //}
            //bincode::encode_into_writer(self, &mut asdf, bincode::config::standard()).unwrap();
            cbor4ii::serde::to_writer(&mut asdf, self).unwrap();
        }

        compressed
    }
    
    fn decompress(data : &[u8]) -> Self
    {
        //struct Asdf<'a> { s : std::io::BufReader<rle16::Decompressor<std::io::Cursor<&'a [u8]>>> }
        let asdf = std::io::BufReader::new(rle16::Decompressor::new(std::io::Cursor::new(data)));
        //let asdf = Asdf { s : std::io::BufReader::new(rle16::Decompressor::new(std::io::Cursor::new(data)) )};
        //struct Asdf<'a> { s : std::io::BufReader<snap::read::FrameDecoder<std::io::Cursor<&'a [u8]>>> }
        //let asdf = Asdf { s : std::io::BufReader::new(snap::read::FrameDecoder::new(std::io::Cursor::new(data)) )};
        //impl<'a> bincode::de::read::Reader for Asdf<'a>
        //{
        //    fn read(&mut self, bytes : &mut [u8]) -> Result<(), bincode::error::DecodeError>
        //    {
        //        self.s.read_exact(bytes).unwrap();
        //        Ok(())
        //    }
        //}
        cbor4ii::serde::from_reader(asdf).unwrap()
        //bincode::decode_from_reader(asdf, bincode::config::standard()).unwrap()
    }
}

#[allow(non_snake_case)]
#[allow(unused)]
#[derive(Serialize, Deserialize)]
struct Warpainter
{
    #[serde(default)]
    // for detection of warpainter project/document files as different from other CBOR files
    WarpainterDocumentCBOR : (),
    
    layers : Layer, // tree, layers contain other layers
    current_layer : u128, // uuid
    
    canvas_width : usize,
    canvas_height : usize,
    
    eraser_mode : bool, // color mode that forces non-eraser tools to act like erasers
    main_color_rgb : [f32; 4],
    main_color_hsv : [f32; 4],
    sub_color_rgb : [f32; 4],
    sub_color_hsv : [f32; 4],
    
    #[serde(default)]
    current_tool : usize, // FIXME change to &'static str
    
    xform : Transform, // view/camera. FIXME: support mirroring
    
    selection_mask : Option<Image<1>>,
    selection_poly : Vec<Vec<[f32; 2]>>,
    
    // unsaved
    #[serde(skip)]
    cache_rect : [[f32; 2]; 2],
    
    #[serde(skip)]
    redo_buffer : Vec<Vec<u8>>,
    #[serde(skip)]
    undo_buffer : Vec<Vec<u8>>,
    
    #[serde(skip)]
    debug_text : Vec<String>,
    
    #[serde(skip)]
    tools : Vec<Box<dyn Tool>>, // FIXME change to VecMap<&'static str, ....
    
    #[serde(skip)]
    edit_is_direct : bool,
    #[serde(skip)]
    edit_ignores_selection : bool,
    #[serde(skip)]
    editing_image : Option<Image<4>>,
    #[serde(skip)]
    editing_offset : [f32; 2],
    
    #[serde(skip)]
    loaded_shaders : bool,
    #[serde(skip)]
    shaders : VecMap<&'static str, Arc<Mutex<ShaderQuad>>>,
    
    #[serde(skip)]
    loaded_fonts : bool,
    #[serde(skip)]
    loaded_icons : bool,
    #[serde(skip)]
    icons : VecMap<&'static str, (egui::TextureHandle, Image<4>)>,
    
    #[serde(skip)]
    did_event_setup : bool,
    
    #[serde(skip)]
    open_dialog : String,
    
    #[serde(skip)]
    edit_progress : u128,
    
    #[serde(skip)]
    file_open_promise : Option<poll_promise::Promise<Option<(String, Vec<u8>)>>>,
}

fn default_tools() -> Vec<Box<dyn Tool>>
{
    vec!(
        Box::new(Pencil::new()),
        Box::new(Pencil::new().into_eraser()),
        Box::new(Line::new()),
        Box::new(Fill::new()),
        Box::new(Eyedropper::new()),
        Box::new(Selection::new()),
        Box::new(MoveTool::new()),
    )
}

impl Default for Warpainter
{
    fn default() -> Self
    {
        let img = image::io::Reader::new(std::io::Cursor::new(&include_bytes!("data/grass4x4plus.png"))).with_guessed_format().unwrap().decode().unwrap().to_rgba8();
        let img = Image::<4>::from_rgbaimage(&img);
        //let img = Image::blank(1024, 1024);
        let canvas_width = img.width;
        let canvas_height = img.height;
        
        let mut root_layer = Layer::new_group("___root___");
        root_layer.uuid = 0;
        let image_layer = Layer::new_layer_from_image("New Layer", img);
        let image_layer_uuid = image_layer.uuid;
        root_layer.children = vec!(image_layer);
        
        let xform = Transform::ident();
        //xform.scale(8.0);
        //xform.translate([500.0, 50.0]);
        
        use rand::Rng;
        Self {
            WarpainterDocumentCBOR : (),
            layers : root_layer,
            current_layer : image_layer_uuid,
            
            canvas_width,
            canvas_height,
            
            edit_is_direct : false,
            edit_ignores_selection : false,
            editing_image : None,
            editing_offset : [0.0, 0.0],
            
            //image_preview : None,
            xform,
            debug_text : Vec::new(),
            
            eraser_mode : false,
            main_color_rgb : [0.0, 0.0, 0.0, 1.0],
            main_color_hsv : [0.0, 0.0, 0.0, 1.0],
            sub_color_rgb : [1.0, 1.0, 1.0, 1.0],
            sub_color_hsv : [1.0, 1.0, 1.0, 1.0],
            
            redo_buffer : Vec::new(),
            undo_buffer : Vec::new(),
            
            tools : Vec::new(),
            current_tool : 0,
            
            loaded_shaders : false,
            shaders : VecMap::new(),
            
            loaded_fonts : false,
            
            loaded_icons : false,
            icons : VecMap::new(),
            
            selection_mask : None,
            selection_poly : Vec::new(),
            
            cache_rect : [[0.0, 0.0], [0.0, 0.0]],
            
            did_event_setup : false,
            
            open_dialog : "".to_string(),
            
            edit_progress : rand::thread_rng().gen(),
            
            file_open_promise : None,
        }
    }
}

impl Warpainter
{
    fn load_from(&mut self, other : Self)
    {
        self.layers = other.layers;
        self.current_layer = other.current_layer;
        
        self.canvas_width = other.canvas_width;
        self.canvas_height = other.canvas_height;
        
        self.eraser_mode = other.eraser_mode;
        self.main_color_rgb = other.main_color_rgb;
        self.main_color_hsv = other.main_color_hsv;
        self.sub_color_rgb = other.sub_color_rgb;
        self.sub_color_hsv = other.sub_color_hsv;
        
        self.current_tool = other.current_tool;
        
        self.xform = other.xform;
        
        self.selection_mask = other.selection_mask;
        self.selection_poly = other.selection_poly;
    }
    fn load_shaders(&mut self, frame : &mut eframe::Frame)
    {
        if self.loaded_shaders || frame.gl().is_none()
        {
            return;
        }
        self.loaded_shaders = true;
        
        if let Some(shader) = ShaderQuad::new(frame.gl().unwrap(), Some(include_str!("color_picker.glsl")))
        {
            self.shaders.insert("colorpicker", Arc::new(Mutex::new(shader)));
        }
        else
        {
            self.loaded_shaders = false;
        }
        
        if let Some(shader) = ShaderQuad::new(frame.gl().unwrap(), Some(include_str!("funbar.glsl")))
        {
            self.shaders.insert("funbar", Arc::new(Mutex::new(shader)));
        }
        else
        {
            self.loaded_shaders = false;
        }
        
        if let Some(shader) = ShaderQuad::new(frame.gl().unwrap(), Some(include_str!("canvas_background.glsl")))
        {
            self.shaders.insert("canvasbackground", Arc::new(Mutex::new(shader)));
        }
        else
        {
            self.loaded_shaders = false;
        }
    }
    fn load_font(&mut self, ctx : &egui::Context)
    {
        if self.loaded_fonts
        {
            return;
        }
        let mut theme = egui::Visuals::dark();
        theme.panel_fill = theme.panel_fill.gamma_multiply(1.5);
        theme.widgets.active.fg_stroke.color = theme.widgets.active.fg_stroke.color.gamma_multiply(1.25);
        theme.widgets.inactive.fg_stroke.color = theme.widgets.inactive.fg_stroke.color.gamma_multiply(1.25);
        theme.widgets.noninteractive.fg_stroke.color = theme.widgets.noninteractive.fg_stroke.color.gamma_multiply(1.35);
        ctx.set_visuals(theme);
        self.loaded_fonts = true;
        
        const GZ_BYTES: &[u8] = include_bytes!("data/IBMPlexSansJP-Regular.ttf.gz");
        fn decompress_gz(data: &[u8]) -> Vec<u8> {
            use std::io::Read;
            let mut decoder = libflate::gzip::Decoder::new(data).expect("Invalid Gzip header");
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).expect("Failed to decompress");
            decompressed
        }
        
        let dec = decompress_gz(GZ_BYTES);
        
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("IBM Plex JP".to_owned(), egui::FontData::from_owned(dec).into());
        fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "IBM Plex JP".to_owned());
        
        ctx.set_fonts(fonts);
        
        ctx.style_mut(|style|
        {
            for (_s, f) in &mut style.text_styles
            {
                f.size *= 0.95;
            }
        });
    }
    fn load_icons(&mut self, ctx : &egui::Context)
    {
        if self.loaded_icons
        {
            return;
        }
        self.loaded_icons = true;
        
        let stuff = [
            ("new layer",                  include_bytes!("icons/new layer.png")                 .to_vec()),
            ("delete layer",               include_bytes!("icons/delete layer.png")              .to_vec()),
            ("duplicate layer",            include_bytes!("icons/duplicate layer.png")           .to_vec()),
            ("new group",                  include_bytes!("icons/new group.png")                 .to_vec()),
            ("into group",                 include_bytes!("icons/into group.png")                .to_vec()),
            ("transfer down",              include_bytes!("icons/transfer down.png")             .to_vec()),
            ("merge down",                 include_bytes!("icons/merge down.png")                .to_vec()),
            ("lock",                       include_bytes!("icons/lock.png")                      .to_vec()),
            ("lock alpha",                 include_bytes!("icons/lock alpha.png")                .to_vec()),
            ("funny alpha",                include_bytes!("icons/funny alpha.png")               .to_vec()),
            ("visible",                    include_bytes!("icons/visible.png")                   .to_vec()),
            ("invisible",                  include_bytes!("icons/invisible.png")                 .to_vec()),
            ("visible_big",                include_bytes!("icons/visible_big.png")               .to_vec()),
            ("clipping mask",              include_bytes!("icons/clipping mask.png")             .to_vec()),
            ("move layer up",              include_bytes!("icons/move layer up.png")             .to_vec()),
            ("move layer down",            include_bytes!("icons/move layer down.png")           .to_vec()),
            
            ("tool pencil",                include_bytes!("icons/tool pencil.png")               .to_vec()),
            ("tool eraser",                include_bytes!("icons/tool eraser.png")               .to_vec()),
            ("tool line",                  include_bytes!("icons/tool line.png")                 .to_vec()),
            ("tool fill",                  include_bytes!("icons/tool fill.png")                 .to_vec()),
            ("tool eyedropper",            include_bytes!("icons/tool eyedropper.png")           .to_vec()),
            ("tool select",                include_bytes!("icons/tool select.png")               .to_vec()),
            ("tool select cursor",         include_bytes!("icons/tool select cursor.png")        .to_vec()),
            ("tool move",                  include_bytes!("icons/tool move.png")                 .to_vec()),
            ("tool move cursor",           include_bytes!("icons/tool move cursor.png")          .to_vec()),
            
            ("icon group",                 include_bytes!("icons/icon group.png")                .to_vec()),
            
            ("undo",                       include_bytes!("icons/undo.png")                      .to_vec()),
            ("redo",                       include_bytes!("icons/redo.png")                      .to_vec()),
            
            ("crosshair",                  include_bytes!("icons/crosshair.png")                 .to_vec()),
        ];
        for thing in stuff
        {
            // FIXME: https://github.com/rust-lang/rust/issues/48331
            let img = image::io::Reader::new(std::io::Cursor::new(&thing.1[..])).with_guessed_format().unwrap().decode().unwrap().to_rgba8();
            let img = Image::from_rgbaimage(&img);
            let tex = ctx.load_texture(
                "my-image",
                img.to_egui(),
                egui::TextureOptions::NEAREST
            );
            self.icons.insert(thing.0, (tex, img));
        }
    }
    fn setup_canvas(&mut self)
    {
        if self.did_event_setup
        {
            return;
        }
        
        self.tools = default_tools();
        
        self.did_event_setup = true;
        
        //self.zoom(2.0);
        
        #[cfg(target_arch = "wasm32")]
        {
            self.debug(format!("setting up event suppression"));
            
            use wasm_bindgen::JsCast;
            
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let root : web_sys::HtmlElement = document.get_element_by_id("the_canvas_id").unwrap().dyn_into().unwrap();
            
            let c = wasm_bindgen::closure::Closure::wrap(Box::new(|e : web_sys::Event|
            {
                if let Ok(event) = e.dyn_into::<web_sys::MouseEvent>()
                {
                    //web_sys::console::log_1(&format!("event received").into());
                    if event.which() == 2
                    {
                        event.prevent_default();
                        //web_sys::console::log_1(&format!("suppressing event").into());
                        //self.debug(format!("suppressing event...."));
                    }
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            
            root.add_event_listener_with_callback("mousedown", c.as_ref().unchecked_ref()).unwrap();
            
            c.forget();
        }
        self.debug(format!("BE AWARE: Warpainter is still Alpha Software, and project files will fail to open in later versions."));
    }
}

impl Warpainter
{
    fn load_from_img(&mut self, img : Image<4>)
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
            //fn fast_rng() -> u32
            //{
            //    static mut SEED: u32 = 0x12345678;
            //    unsafe {
            //        SEED ^= SEED << 13;
            //        SEED ^= SEED >> 17;
            //        SEED ^= SEED << 5;
            //        SEED
            //    }
            //}
            
            let mut tool = self.tools.remove(self.current_tool);
            tool.think(self, inputstate);
            
            // think twice! (debugging buggy tools)
            //if (fast_rng() & 1) == 0
            //{
            //    tool.think(self, inputstate);
            //}
            
            self.tools.insert(self.current_tool, tool);
        }
    }
    fn tool_notify_changed(&mut self, prev : usize)
    {
        if prev < self.tools.len()
        {
            let mut tool = self.tools.remove(prev);
            tool.notify_tool_changed(self);
            self.tools.insert(prev, tool);
        }
    }
    fn tool_panel(&mut self, ui : &mut Ui)
    {
        if self.current_tool < self.tools.len()
        {
            let mut tool = self.tools.remove(self.current_tool);
            tool.settings_panel(self, ui);
            self.tools.insert(self.current_tool, tool);
        }
    }
    #[allow(clippy::borrowed_box)]
    fn get_tool(&self) -> Option<&Box<dyn Tool>>
    {
        self.tools.get(self.current_tool)
    }
}

impl Warpainter
{
    fn sample_poly_sdf(mut c : [f32; 2], points : &[[f32; 2]]) -> f32
    {
        c[0] += 0.5;
        c[1] += 0.5;
        let mut closest = 10000000.0;
        let mut a = points[0];
        
        let mut inside = false;
        
        for b in points.iter()
        {
            let b = *b;
            let u = vec_sub(&b, &a);
            let v = vec_sub(&a, &c);
            
            let den = vec_dot(&u, &u);
            
            if den > 0.0
            {
                // check if this is the closest line segment to our coord
                let t = -(vec_dot(&v, &u)/den);
                if t > 0.0 && t < 1.0
                {
                    let new = length_sq(&vec_sub(&vec_lerp(&a, &b, t), &c));
                    if new < closest
                    {
                        closest = new;
                    }
                }
                closest = closest.min(length_sq(&v));
                
                // even-odd rule rasterization for the fill
                if (a[1] > c[1]) != (b[1] > c[1])
                {
                    let cb = vec_sub(&c, &b);
                    let ab = vec_sub(&[0.0, 0.0], &u);
                    let s = cb[0] * ab[1] - cb[1] * ab[0];
                    inside = inside != ((s < 0.0) == (ab[1] < 0.0));
                }
            }
            
            a = b;
        }
        
        closest.sqrt() * if inside { 1.0 } else { -1.0 }
    }
    fn clear_selection(&mut self)
    {
        self.selection_mask = None;
        self.selection_poly = Vec::new();
    }
    fn commit_selection(&mut self, loops : Vec<Vec<[f32; 2]>>)
    {
        self.selection_mask = None;
        let mut mask = Image::<1>::blank_float(self.canvas_width, self.canvas_height);
        for y in 0..self.canvas_height
        {
            for x in 0..self.canvas_width
            {
                let mut mid : f32 = 1000000.0;
                for points in loops.iter()
                {
                    let new = Self::sample_poly_sdf([x as f32, y as f32], points);
                    if new.abs() < mid.abs()
                    {
                        mid = new;
                    }
                }
                let c = (mid + 0.5).clamp(0.0, 1.0);
                mask.set_pixel_float_wrapped(x as isize, y as isize, [c]);
            }
        }
        self.selection_mask = Some(mask);
        self.selection_poly = loops;
    }
    fn get_selection_loop_data(&self) -> Vec<[f32; 4]>
    {
        let mut ret = Vec::new();
        for points in self.selection_poly.iter()
        {
            for coord in points.iter()
            {
                ret.push([coord[0], coord[1], 0.0, 0.0]);
            }
            ret.push([0.0, 0.0, 1.0, 0.0]);
        }
        ret.push([0.0, 0.0, 1.0, 0.0]);
        ret
    }
}

impl Warpainter
{
    fn begin_edit(&mut self, inplace : bool, ignore_selection : bool)
    {
        self.edit_progress += 1;
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            if !layer.locked
            {
                //println!("start edit dirty rect {:?}", layer.edited_dirty_rect);
                //println!("start edit flattening rect {:?}", layer.flattened_dirty_rect);
                if let Some(image) = &layer.data
                {
                    layer.edited_dirty_rect = None;
                    self.edit_is_direct = inplace;
                    self.edit_ignores_selection = ignore_selection;
                    if inplace
                    {
                        self.editing_image = Some(image.clone());
                    }
                    else
                    {
                        self.editing_image = Some(image.blank_with_same_size());
                    }
                    self.editing_offset = [layer.offset[0] as f32, layer.offset[1] as f32];
                }
            }
        }
    }
    
    fn get_editing_offset(&self) -> [f32; 2]
    {
        self.editing_offset
    }
    fn get_editing_image(&mut self) -> Option<&mut Image<4>>
    {
        self.editing_image.as_mut()
    }
    fn get_current_offset(&self) -> [f32; 2]
    {
        if let Some(layer) = self.layers.find_layer(self.current_layer)
        {
            layer.offset
        }
        else
        {
            [0.0, 0.0]
        }
    }
    fn get_current_layer_image(&mut self) -> Option<& Image<4>>
    {
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            Some(layer.flatten(self.canvas_width, self.canvas_height, None, None))
        }
        else
        {
            None
        }
    }
    fn get_current_layer_data(&mut self) -> Option<&mut Image<4>>
    {
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.data.as_mut()
        }
        else
        {
            None
        }
    }
    fn is_editing(&self) -> bool
    {
        self.editing_image.is_some()
    }
    fn flatten(&mut self) -> &Image<4>
    {
        if let Some(override_image) = self.get_temp_edit_image()
        {
            // FIXME convey whether the edit is a direct edit
            self.layers.flatten_as_root(self.canvas_width, self.canvas_height, Some(self.current_layer), Some(&override_image))
        }
        else
        {
            self.layers.flatten_as_root(self.canvas_width, self.canvas_height, None, None)
        }
    }
    fn flatten_use(&self) -> Option<&Image<4>>
    {
        self.layers.flatten_get_cached()
    }
    fn get_temp_edit_image(&self) -> Option<Image<4>> // only used in flattening
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
                            if let Some(selection_mask) = &self.selection_mask
                            {
                                if !self.edit_ignores_selection
                                {
                                    let mut under = current_image.clone();
                                    under.blend_from(edit_image, Some(selection_mask), None, 1.0, [0, 0], "Interpolate");
                                    return Some(under);
                                }
                            }
                            return Some(edit_image.clone());
                        }
                        else
                        {
                            let mut drawn = current_image.clone(); // FIXME performance drain, find a way to use a dirty rect here
                            drawn.blend_from(edit_image, None, None, 1.0, [0, 0], "Normal"); // FIXME use drawing opacity / brush alpha
                            
                            if let Some(selection_mask) = &self.selection_mask
                            {
                                if !self.edit_ignores_selection
                                {
                                    let mut under = current_image.clone();
                                    under.blend_from(&drawn, Some(selection_mask), None, 1.0, [0, 0], "Interpolate");
                                    return Some(under);
                                }
                            }
                            return Some(drawn);
                        }
                    }
                }
            }
        }
        None
    }
    
    #[inline(never)]
    fn commit_edit(&mut self)
    {
        self.cache_rect_full();
        
        self.edit_progress += 1;
        self.debug(format!("Committing edit {}", self.edit_progress));
        if let Some(mut image) = self.get_temp_edit_image()
        {
            if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
            {
                if !layer.locked
                {
                    if let Some(current_image) = &mut layer.data
                    {
                        std::mem::swap(&mut image, &mut *current_image);
                        
                        self.redo_buffer = Vec::new();
                        let mut rect : Option<[[f32; 2]; 2]> = layer.edited_dirty_rect;
                        //println!("A? {:?}", rect);
                        if let Some(rect) = &mut rect
                        {
                            rect[0] = vec_sub(&rect[0], &layer.offset);
                            rect[1] = vec_sub(&rect[1], &layer.offset);
                        }
                        //println!("B? {:?}", rect);
                        let start = web_time::Instant::now();
                        let event = Image::<4>::analyze_edit(&image, current_image, self.current_layer, rect);
                        println!("Edit analysis time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
                        //println!("{}", event.len());
                        let start = web_time::Instant::now();
                        self.undo_buffer.push(event.compress());
                        println!("Edit encoding time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
                    }
                }
            }
        }
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.dirtify_edited();
        }
        
        self.editing_image = None;
        self.edit_is_direct = false;
        self.edit_ignores_selection = false;
    }
    fn cancel_edit(&mut self)
    {
        if self.editing_image.is_some()
        {
            self.cache_rect_full();
            println!("Cancelling edit");
            self.edit_progress += 1;
            if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
            {
                layer.dirtify_edited();
            }
            self.editing_image = None;
            self.edit_is_direct = false;
            self.edit_ignores_selection = false;
        }
    }
    fn log_layer_info_change(&mut self)
    {
        self.cache_rect_full();
        println!("Layer info change");
        self.edit_progress += 1;
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            let old_info = layer.old_info_for_undo.clone();
            let new_info = layer.get_info();
            layer.commit_info();
            
            self.redo_buffer = Vec::new();
            let event = UndoEvent::LayerInfoChange(LayerInfoChange {
                uuid : self.current_layer,
                old : old_info,
                new : new_info,
            });
            self.undo_buffer.push(event.compress());
        }
    }
    fn cache_rect_reset(&mut self)
    {
        self.cache_rect = [[1.0, 1.0], [1.0, 1.0]];
    }
    fn cache_rect_full(&mut self)
    {
        self.cache_rect = [[0.0, 0.0], [self.canvas_width as f32, self.canvas_height as f32]];
    }
    fn cache_rect_merge(&mut self, r : [[f32; 2]; 2])
    {
        if self.cache_rect[0][0] != self.cache_rect[1][0] || self.cache_rect[0][1] != self.cache_rect[1][1]
        {
            self.cache_rect = rect_enclose_rect(self.cache_rect, r);
        }
        else
        {
            self.cache_rect = r;
        }
    }
    fn perform_undo(&mut self)
    {
        println!("Undo");
        self.edit_progress += 1;
        if let Some(event) = self.undo_buffer.pop()
        {
            let start = web_time::Instant::now();
            let event = UndoEvent::decompress(&event);
            println!("Edit decoding time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
            match event
            {
                UndoEvent::LayerPaint(ref event) =>
                {
                    if let Some(layer) = self.layers.find_layer_mut(event.uuid)
                    {
                        if let Some(ref mut data) = &mut layer.data
                        {
                            data.undo_edit(event);
                            println!("undo done");
                        }
                        let r = event.rect;
                        println!("{:?}", r);
                        let r = [[r[0][0] as f32, r[0][1] as f32], [r[1][0] as f32, r[1][1] as f32]];
                        layer.dirtify_rect(r);
                        self.cache_rect_merge(r);
                        //layer.dirtify_all();
                    }
                }
                UndoEvent::LayerInfoChange(ref event) =>
                {
                    if let Some(layer) = self.layers.find_layer_mut(event.uuid)
                    {
                        layer.set_info(&event.old);
                        layer.dirtify_all();
                        self.cache_rect_full();
                        println!("info undo done");
                    }
                }
                _ =>
                {
                    println!("not supported yet ({:?})", event);
                }
            }
            self.redo_buffer.push(event.compress());
        }
        else
        {
            println!("nothing to undo");
        }
    }
    fn perform_redo(&mut self)
    {
        println!("Redo");
        self.edit_progress += 1;
        if let Some(event) = self.redo_buffer.pop()
        {
            let start = web_time::Instant::now();
            let event = UndoEvent::decompress(&event);
            println!("Edit decoding time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
            match event
            {
                UndoEvent::LayerPaint(ref event) =>
                {
                    if let Some(layer) = self.layers.find_layer_mut(event.uuid)
                    {
                        if let Some(ref mut data) = &mut layer.data
                        {
                            data.redo_edit(event);
                            println!("redo done");
                        }
                        let r = event.rect;
                        println!("{:?}", r);
                        let r = [[r[0][0] as f32, r[0][1] as f32], [r[1][0] as f32, r[1][1] as f32]];
                        layer.dirtify_rect(r);
                        self.cache_rect_merge(r);
                        //layer.dirtify_all();
                    }
                }
                UndoEvent::LayerInfoChange(ref event) =>
                {
                    if let Some(layer) = self.layers.find_layer_mut(event.uuid)
                    {
                        layer.set_info(&event.new);
                        layer.dirtify_all();
                        self.cache_rect_full();
                        println!("info redo done");
                    }
                }
                _ =>
                {
                    println!("not supported yet");
                }
            }
            self.undo_buffer.push(event.compress());
        }
        else
        {
            println!("nothing to redo");
        }
    }
    
    fn full_rerender(&mut self)
    {
        self.cache_rect_full();
        println!("Full Rerender");
        self.edit_progress += 1;
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.dirtify_all();
        }
    }
    fn full_rerender_with(&mut self, id : u128)
    {
        self.cache_rect_full();
        println!("Full Rerender of Layer {}", id);
        self.edit_progress += 1;
        if let Some(layer) = self.layers.find_layer_mut(id)
        {
            layer.dirtify_all();
        }
    }
    fn mark_current_layer_dirty(&mut self, rect : [[f32; 2]; 2])
    {
        self.cache_rect_merge(rect);
        //println!("Dirtify Rect");
        self.edit_progress += 1;
        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
        {
            layer.dirtify_rect(rect);
        }
    }
    
    fn current_layer_is_alpha_locked(&self) -> bool
    {
        if let Some(layer) = self.layers.find_layer(self.current_layer)
        {
            return layer.alpha_locked;
        }
        false
    }
    fn find_layer_parent_and_index(&self, layer_uuid : u128) -> Option<(u128, usize)>
    {
        if let Some(layer) = self.layers.find_layer_parent(self.current_layer)
        {
            for (i, child) in layer.children.iter().enumerate()
            {
                if child.uuid == layer_uuid
                {
                    return Some((layer.uuid, i));
                }
            }
        }
        None
    }
}


impl Warpainter
{
    fn get_zoom(&self) -> f32
    {
        self.xform.get_scale()
    }
    fn zoom(&mut self, amount : f32)
    {
        let mut log_zoom = self.xform.get_scale().max(0.01).log(2.0);
        let f = 4.0;
        let old_zoom = (log_zoom*f).round()/f;
        
        log_zoom += amount;
        
        let mut new_zoom = (log_zoom*f).round()/f;
        if new_zoom == old_zoom
        {
            new_zoom = log_zoom;
        }
        new_zoom = new_zoom.clamp(-8.0, 8.0);
        self.xform.set_scale(2.0_f32.powf(new_zoom));
    }
    fn zoom_unrounded(&mut self, amount : f32)
    {
        let mut log_zoom = self.xform.get_scale().max(0.01).log(2.0);
        log_zoom += amount;
        log_zoom = log_zoom.clamp(-8.0, 8.0);
        self.xform.set_scale(2.0_f32.powf(log_zoom));
    }
    fn view_reset(&mut self)
    {
        self.xform = Transform::ident();
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
        self.main_color_rgb = new;
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
        self.sub_color_rgb = new;
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
    fn build_ora_data(&mut self) -> Vec<u8>
    {
        self.cancel_edit();
        
        use xot::Xot;
        let mut xot = Xot::new();
        
        let image_name = xot.add_name("image");
        let stack_name = xot.add_name("stack");
        let layer_name = xot.add_name("layer");
        
        let version_name = xot.add_name("version");
        let src_name = xot.add_name("src");
        let name_name = xot.add_name("name");
        let w_name = xot.add_name("w");
        let h_name = xot.add_name("h");
        let x_name = xot.add_name("x");
        let y_name = xot.add_name("y");
        let opacity_name = xot.add_name("opacity");
        let visibility_name = xot.add_name("visibility");
        let isolation_name = xot.add_name("isolation");
        let composite_op_name = xot.add_name("composite-op");
        
        // extensions
        let fill_opacity_name = xot.add_name("fill-opacity");
        let real_opacity_name = xot.add_name("real-opacity");
        let clipped_name = xot.add_name("clipped");
        let uuidhex_name = xot.add_name("uuidhex");
        let _masks_name = xot.add_name("masks"); // TODO
        let _mask_name = xot.add_name("mask"); // TODO
        
        let root = xot.new_element(image_name);
        //xot.attributes_mut(root).insert(version_name, "0.0.6-wp.1".to_string());
        xot.attributes_mut(root).insert(version_name, "0.0.6".to_string());
        xot.attributes_mut(root).insert(w_name, format!("{}", self.canvas_width));
        xot.attributes_mut(root).insert(h_name, format!("{}", self.canvas_height));
        
        let stack = xot.new_element(stack_name);
        xot.append(root, stack).unwrap();
        
        use zip::write::ZipWriter;
        type Zw<'a> = ZipWriter<std::io::Cursor<&'a mut Vec<u8>>>;
        let mut zipbuf = vec!();
        let mut zip = zip::write::ZipWriter::new(std::io::Cursor::new(&mut zipbuf));
        let zipref = &mut zip;
        let _zip_options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let zip_options = _zip_options.clone();
        
        zipref.start_file("mimetype", zip_options).unwrap();
        zipref.write(b"image/openraster").unwrap();
        
        let mut img = self.flatten().to_imagebuffer();
        let bytes = save_image_to_vec(&img);
        zipref.start_file("mergedimage.png", zip_options).unwrap();
        zipref.write(&bytes).unwrap();
        
        fn premultiply(img : &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>)
        {
            for p in img.pixels_mut()
            {
                let [r, g, b, a] = p.0;
                let af = a as f32 / 255.0;
                p.0 = [(r as f32 * af) as u8, (g as f32 * af) as u8, (b as f32 * af) as u8, a];
            }
        }
        
        fn unpremultiply(img : &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>)
        {
            for p in img.pixels_mut()
            {
                let [r, g, b, a] = p.0;
                if a != 0
                {
                    let af = 255.0 / a as f32;
                    p.0 = [
                        (r as f32 * af).clamp(0.0, 255.0) as u8,
                        (g as f32 * af).clamp(0.0, 255.0) as u8,
                        (b as f32 * af).clamp(0.0, 255.0) as u8,
                        a,
                    ];
                }
            }
        }
        
        if img.width() > 256 || img.height() > 256
        {
            let mut w = img.width() as f64;
            let mut h = img.height() as f64;
            let n = 256.0 / w.max(h);
            w = (w*n).floor();
            h = (h*n).floor();
            premultiply(&mut img);
            img = image::imageops::resize(&img, w as u32, h as u32, image::imageops::FilterType::Gaussian);
            unpremultiply(&mut img);
        }
        
        let bytes = save_image_to_vec(&img);
        zipref.start_file("Thumbnails/thumbnail.png", zip_options).unwrap();
        zipref.write(&bytes).unwrap();
        
        fn save_image_to_vec(buffer : &image::ImageBuffer<image::Rgba<u8>, Vec<u8>>) -> Vec<u8>
        {
            let mut ret = Vec::new();
            {
                let mut encoder = png::Encoder::new(&mut ret, buffer.width(), buffer.height());
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
                encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
                let mut writer = encoder.write_header().unwrap();
                writer.write_image_data(buffer.as_raw()).unwrap();
            }
            ret
        }
        
        fn get_svg_composite_op(s : &str) -> &'static str
        {
            match s
            {
                "Normal" => "svg:src-over",
                "Composite" => "svg:src-over",
                "Multiply" => "svg:multiply",
                "Divide" => "svg:difference",
                "Screen" => "svg:multiply",
                "Add" => "svg:plus",
                "Glow Add" => "svg:plus",
                "Subtract" => "svg:difference",
                "Difference" => "svg:difference",
                "Signed Diff" => "svg:difference",
                "Signed Add" => "svg:plus",
                "Negation" => "svg:difference",
                "Lighten" => "svg:lighten",
                "Darken" => "svg:darken",
                "Linear Burn" => "svg:multiply",
                "Color Burn" => "svg:color-burn",
                "Color Dodge" => "svg:color-dodge",
                "Glow Dodge" => "svg:color-dodge",
                "Glow" => "svg:color-dodge", // or hard light
                "Reflect" => "svg:hard-light",
                "Overlay" => "svg:overlay",
                
                "Soft Light" => "svg:soft-light",
                
                "Hard Light" => "svg:hard-light",
                "Vivid Light" => "svg:hard-light",
                "Linear Light" => "svg:hard-light",
                "Pin Light" => "svg:hard-light",
                "Hard Mix" => "svg:hard-light",
                
                "Exclusion" => "svg:difference",
                
                "Hue" => "svg:hue",
                "Saturation" => "svg:saturation",
                "Color" => "svg:color",
                "Luminosity" => "svg:luminosity",
                "Flat Hue" => "svg:hue",
                "Flat Sat" => "svg:saturation",
                "Flat Color" => "svg:luminosity",
                "Value" => "svg:luminosity",
                "Hard Sat" => "svg:saturation",
                "Hard Color" => "svg:color",
                "Lightness" => "svg:luminosity",
                
                "Erase" => "svg:dst-out",
                "Reveal" => "svg:dst-atop",
                "Alpha Mask" => "svg:dst-in",
                "Alpha Reject" => "svg:dst-out",
                "Under" => "svg:src-over", // svg:dst-over
                "Interpolate" => "svg:src-over", // svg:src-in
                
                "Dither" => "svg:src-over",
                
                // internal
                "Hard Interpolate" => "FIXME",
                "Clamp Erase" => "FIXME",
                "Merge Alpha" => "FIXME",
                "Clip Alpha" => "FIXME",
                "Max Alpha" => "FIXME",
                "Copy Alpha" => "FIXME",
                
                "Copy" => "FIXME",
                "Weld Under" => "FIXME",
                "Alpha Antiblend" => "FIXME",
                "Blend Weld" => "FIXME",
                "Sum Weld" => "FIXME",
                "Weld" => "FIXME",
                "Soft Weld" => "FIXME",
                "Hard Weld" => "FIXME",
                "Clip Weld" => "FIXME",
                
                // fallback
                _ => "svg:src-over",
            }
        }
        
        use std::rc::Rc;
        use std::any::Any;
        let visitor
            : Rc<dyn Fn(&mut Zw, Rc<dyn Any>, &mut Xot, xot::Node, &Layer) -> ()>
            = Rc::new(move |zipref : &mut Zw, selfie : Rc<dyn Any>, xot : &mut Xot, node : xot::Node, layer : &Layer| -> ()
        {
            let d = if layer.is_group()
            {
                let d = xot.new_element(stack_name);
                xot.append(node, d).unwrap();
                xot.attributes_mut(d).insert(name_name, layer.name.clone());
                xot.attributes_mut(d).insert(composite_op_name, get_svg_composite_op(&layer.blend_mode).to_string());
                xot.attributes_mut(d).insert(isolation_name, "isolate".to_string());
                
                let f = selfie.clone().downcast::<Rc<dyn Fn(&mut Zw, Rc<dyn Any>, &mut Xot, xot::Node, &Layer)>>().unwrap();
                for c in layer.children.iter()
                {
                    f(zipref, Rc::clone(&selfie), xot, d, c);
                }
                
                d
            }
            else if let Some(data) = &layer.data
            {
                let d = xot.new_element(layer_name);
                xot.append(node, d).unwrap();
                xot.attributes_mut(d).insert(name_name, layer.name.clone());
                xot.attributes_mut(d).insert(composite_op_name, get_svg_composite_op(&layer.blend_mode).to_string());
                
                let img = data.to_imagebuffer();
                let bytes = save_image_to_vec(&img);
                let fname = format!("data/{}.png", layer.uuid);
                zipref.start_file(&fname, zip_options).unwrap();
                zipref.write(&bytes).unwrap();
                
                xot.attributes_mut(d).insert(x_name, format!("{}", layer.offset[0] as i64));
                xot.attributes_mut(d).insert(y_name, format!("{}", layer.offset[1] as i64));
                xot.attributes_mut(d).insert(src_name, fname);
                
                d
            }
            else
            {
                let d = xot.new_element(stack_name);
                xot.append(node, d).unwrap();
                xot.attributes_mut(d).insert(name_name, layer.name.clone() + " (dummy/adjustment)");
                xot.attributes_mut(d).insert(composite_op_name, get_svg_composite_op(&layer.blend_mode).to_string());
                d
            };
            
            xot.attributes_mut(d).insert(uuidhex_name, format!("{}", layer.uuid));
            xot.attributes_mut(d).insert(opacity_name, format!("{}", layer.opacity * layer.fill_opacity));
            xot.attributes_mut(d).insert(visibility_name, if layer.visible { "visible" } else { "hidden" }.to_string() );
            
            xot.attributes_mut(d).insert(fill_opacity_name, format!("{}", layer.fill_opacity));
            xot.attributes_mut(d).insert(real_opacity_name, format!("{}", layer.fill_opacity));
            xot.attributes_mut(d).insert(clipped_name, if layer.clipped { "true" } else { "false" }.to_string() );
        });
        
        for c in self.layers.children.iter()
        {
            visitor.clone()(zipref, Rc::new(visitor.clone()), &mut xot, stack, c);
        }
        
        let doc = xot.new_document_with_element(root).unwrap();
        
        let zip_options = _zip_options.clone();
        zipref.start_file("stack.xml", zip_options).unwrap();
        let xml = xot.to_string(doc).unwrap();
        zipref.write(xml.as_bytes()).unwrap();
        
        drop(zip);
        
        zipbuf
    }
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
        //self.debug(format!("{} then {:?}", self.current_layer, new_uuid));
        if new_uuid.is_none()
        {
            //self.debug("fallback...");
            new_uuid = self.layers.uuid_of_prev(self.current_layer);
        }
        if let Some(new_uuid) = new_uuid
        {
            self.layers.delete_layer(self.current_layer);
            self.current_layer = new_uuid;
        }
    }
}

static mut GL : Option<Arc<glow::Context>> = None;

use egui::{Margin, Frame};

impl eframe::App for Warpainter
{
    fn update(&mut self, ctx : &egui::Context, frame : &mut eframe::Frame)
    {
        self.setup_canvas();
        self.load_icons(ctx);
        self.load_font(ctx);
        self.load_shaders(frame);
        
        unsafe
        {
            if (*&raw const GL).is_none()
            {
                use eframe::egui_glow;
                let cb = egui_glow::CallbackFn::new(move |_info, glow_painter|
                {
                    GL = Some(Arc::clone(glow_painter.gl()));
                });
                let callback = egui::PaintCallback { rect : [[0.0, 0.0].into(), [100.0, 100.0].into()].into(), callback : Arc::new(cb) };
                ctx.debug_painter().add(callback);
                return;
            }
        }
        
        let mut focus_is_global = true;
        let mut new_dialog_opened = &self.open_dialog == "New Window";
        if new_dialog_opened
        {
            focus_is_global = false;
            // need an "Area" to be able to capture inputs outside of the open window
            egui::Area::new("New File Dummy BG".into()).interactable(true).fixed_pos(egui::Pos2::ZERO).show(ctx, |ui|
            {
                let screen_rect = ui.ctx().input(|i| i.screen_rect);
                ui.painter().rect_filled(screen_rect, egui::CornerRadius::ZERO, egui::Rgba::from_rgba_unmultiplied(0.1, 0.1, 0.1, 0.5));
                
                let mut still_open = true;
                let window_response = egui::Window::new("New File")
                    .resizable(false)
                    .open(&mut still_open)
                    .pivot(egui::Align2::CENTER_CENTER).default_pos((screen_rect.size() / 2.0).to_pos2())
                    .show(ctx, |ui|
                {
                    let (mut width, mut height) = ui.data(|map|
                    {
                        map.get_temp(egui::Id::new("New File Height/Width")).unwrap_or((800, 600))
                    });
                    ui.horizontal(|ui|
                    {
                        ui.add_sized([50.0, 16.0], egui::DragValue::new(&mut width).range(1..=32000));
                        ui.label("Width");
                    });
                    ui.horizontal(|ui|
                    {
                        ui.add_sized([50.0, 16.0], egui::DragValue::new(&mut height).range(1..=32000));
                        ui.label("Height");
                    });
                    ui.label("Note: Any unsaved progress in your current file will be immediately lost if you make a new file.");
                    ui.data_mut(|map|
                    {
                        map.insert_temp(egui::Id::new("New File Height/Width"), (width, height))
                    });
                    ui.allocate_ui_with_layout([190.0, 0.0].into(), egui::Layout::right_to_left(egui::Align::BOTTOM), |ui|
                    {
                        if ui.button("Cancel").clicked()
                        {
                            new_dialog_opened = false;
                        }
                        if width as usize * height as usize > 140000000
                        {
                            ui.add_enabled(false, egui::Button::new("OK"));
                        }
                        else
                        {
                            if ui.button("OK").clicked()
                            {
                                // TODO: reset view transform and zoom out to display entire canvas
                                let img = Image::<4>::blank_white_transparent(width, height);
                                self.load_from_img(img);
                                new_dialog_opened = false;
                            }
                        }
                    });
                    if width as usize * height as usize > 140000000
                    {
                        ui.label("Maximum supported resolution is 140 megapixels.");
                        ui.label("Reduce your canvas size.");
                    }
                });
                if !still_open
                {
                    new_dialog_opened = false;
                }
                ui.allocate_response(screen_rect.size(), egui::Sense::click());
                ctx.move_to_top(window_response.unwrap().response.layer_id); // prevent window from being hidden by "Area"
            });
        }
        
        if &self.open_dialog == "New Window" && !new_dialog_opened
        {
            self.open_dialog = "".to_string();
        }
        
        egui::TopBottomPanel::top("Menu Bar").show(ctx, |ui|
        {
            egui::menu::bar(ui, |ui|
            {
                ui.menu_button("File", |ui|
                {
                    if ui.button("New...").clicked()
                    {
                        self.open_dialog = "New Window".to_string();
                        ui.close_menu();
                    }
                    
                    // FIXME: highly duplicated grabage. deduplicate!!!
                    
                    macro_rules! get_state { () => {
                    {
                        self.cancel_edit();
                        
                        
                        //for l in self.layers.children.iter_mut()
                        //if false
                        {
                            //l.visit_layers_mut(0, &mut |l, _depth|
                            self.layers.visit_layers_mut(0, &mut |l, _depth|
                            {
                                std::mem::swap(&mut l.flattened_data, &mut l._dummy_flattened_data);
                                std::mem::swap(&mut l.flattened_dirty_rect, &mut l._dummy_flattened_dirty_rect);
                                Some(())
                            });
                        }
                        
                        let mut data = Vec::new();
                        cbor4ii::serde::to_writer(&mut data, self).unwrap();
                        
                        //for l in self.layers.children.iter_mut()
                        //if false
                        {
                            //l.visit_layers_mut(0, &mut |l, _depth|
                            self.layers.visit_layers_mut(0, &mut |l, _depth|
                            {
                                std::mem::swap(&mut l.flattened_data, &mut l._dummy_flattened_data);
                                std::mem::swap(&mut l.flattened_dirty_rect, &mut l._dummy_flattened_dirty_rect);
                                Some(())
                            });
                        }
                        
                        data
                    } } }
                    
                    let _ = &ui; // suppress unused warning on wasm32
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if ui.button("Open...").clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Supported Formats",
                                    &["wpp", "png", "jpg", "jpeg", "gif", "bmp", "tga", "tiff", "webp", "ico", "pnm", "pbm", "ppm", "avif", "dds", "qoi", "psd"])
                                .add_filter("Warpainter Project", &["wpp"])
                                .add_filter("Other Projects", &["psd"])
                                //.add_filter("Other Projects", &["psd", "ora"])
                                .add_filter("Images",
                                    &["png", "jpg", "jpeg", "gif", "bmp", "tga", "tiff", "webp", "ico", "pnm", "pbm", "ppm", "avif", "dds", "qoi"])
                                //.add_filter("Warpainter Project",
                                //    &["wrp"])
                                .pick_file()
                            {
                                self.cancel_edit();
                                
                                println!("{}", path.extension().unwrap().to_string_lossy());
                                if path.extension().unwrap().to_string_lossy() == "psd"
                                {
                                    let start = web_time::Instant::now();
                                    let bytes = std::fs::read(path).unwrap();
                                    wpsd_open(self, &bytes);
                                    println!("PSD load time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
                                }
                                else if path.extension().unwrap().to_string_lossy() == "wpp"
                                {
                                    self.cancel_edit();
                                    
                                    let start = web_time::Instant::now();
                                    
                                    fn load(path : &std::path::Path) -> Result<Warpainter, String>
                                    {
                                        let file = std::fs::File::open(path).map_err(|x| x.to_string())?;
                                        
                                        let reader = std::io::BufReader::new(file);
                                        
                                        let data : Warpainter = cbor4ii::serde::from_reader(reader).map_err(|x| x.to_string())?;
                                        Ok(data)
                                    }
                                    
                                    let new = load(&path).unwrap();
                                    self.load_from(new);
                                    println!("WPP load time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
                                }
                                else
                                {
                                    // FIXME handle error
                                    let img = image::io::Reader::open(path).unwrap().decode().unwrap().to_rgba8();
                                    let img = Image::<4>::from_rgbaimage(&img);
                                    self.load_from_img(img);
                                }
                            }
                            ui.close_menu();
                        }
                        
                        fn save_vec_u8_atomic(path : &std::path::Path, data : &[u8]) -> Result<(), String>
                        {
                            use atomicwrites::{AtomicFile, AllowOverwrite};
                            let af = AtomicFile::new(path, AllowOverwrite);
                            af.write(|f| f.write_all(data)).map_err(|x| x.to_string())
                        }
                        if ui.button("Save As...").clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Warpainter Project", &["wpp"])
                                .save_file()
                            {
                                // FIXME handle error
                                save_vec_u8_atomic(&path, &get_state!()).unwrap();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Save PNG...").clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG", &["png"])
                                .save_file()
                            {
                                self.cancel_edit();
                                
                                let img = self.flatten().to_imagebuffer();
                                // FIXME handle error
                                img.save(path).unwrap();
                            }
                            ui.close_menu();
                        }
                        if ui.button("Save ORA...").clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Open Raster", &["ora"])
                                .save_file()
                            {
                                let zipbuf = self.build_ora_data();
                                save_vec_u8_atomic(&path, &zipbuf).unwrap();
                            }
                            ui.close_menu();
                        }
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        if ui.button("Open").clicked()
                        {
                            let future = async
                            {
                                let file = rfd::AsyncFileDialog::new()
                                    .add_filter("Supported Formats",
                                                &["wpp", "png", "jpg", "jpeg", "gif", "bmp", "tga", "tiff", "webp", "ico", "pnm", "pbm", "ppm", "avif", "dds", "psd"])
                                    .add_filter("Warpainter Project", &["wpp"])
                                    .add_filter("Other Projects", &["psd"])
                                    //.add_filter("Other Projects", &["psd", "ora"])
                                    .add_filter("Images",
                                        &["png", "jpg", "jpeg", "gif", "bmp", "tga", "tiff", "webp", "ico", "pnm", "pbm", "ppm", "avif", "dds", "qoi"])
                                    .pick_file().await;
                                
                                if let Some(file) = file
                                {
                                    let data = file.read().await;
                                    Some((file.file_name(), data))
                                }
                                else
                                {
                                    None
                                }
                            };
                            self.file_open_promise = Some(poll_promise::Promise::spawn_local(future));
                            ui.close_menu();
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                            
                            // GOTO: OPENFILEWEB
                        }
                        if ui.button("Save As...").clicked()
                        {
                            let data = get_state!();
                            
                            let future = async move
                            {
                                if let Some(file_handle) = rfd::AsyncFileDialog::new()
                                    .set_file_name("WpProject.wpp").save_file().await
                                {
                                    file_handle.write(&data).await.unwrap();
                                }
                            };
                            wasm_bindgen_futures::spawn_local(future);
                            ui.close_menu();
                        }
                        if ui.button("Save PNG...").clicked()
                        {
                            self.cancel_edit();
                            let img = self.flatten().to_imagebuffer();
                            
                            let future = async move
                            {
                                if let Some(file_handle) = rfd::AsyncFileDialog::new()
                                    .set_file_name("WpProject.png").save_file().await
                                {
                                    let mut bytes = Vec::new();
                                    img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageOutputFormat::Png).unwrap();

                                    file_handle.write(&bytes).await.unwrap();
                                }
                            };
                            wasm_bindgen_futures::spawn_local(future);
                            ui.close_menu();
                        }
                        if ui.button("Save ORA...").clicked()
                        {
                            self.cancel_edit();
                            let zipbuf = self.build_ora_data();
                            
                            let future = async move
                            {
                                if let Some(file_handle) = rfd::AsyncFileDialog::new()
                                    .set_file_name("WpProject.ora").save_file().await
                                {
                                    file_handle.write(&zipbuf).await.unwrap();
                                }
                            };
                            wasm_bindgen_futures::spawn_local(future);
                            ui.close_menu();
                        }
                    }
                });
                
                #[cfg(target_arch = "wasm32")]
                {
                    // COMEFROM: OPENFILEWEB
                    if self.file_open_promise.is_some()
                    {
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    if let Some(Some(Some((name, data)))) = self.file_open_promise.as_ref().map(|x| x.ready())
                    {
                        let name = name.clone();
                        let data = data.clone();
                        println!("{}", name);
                        if name.ends_with(".psd")
                        {
                            wpsd_open(self, &data);
                        }
                        else if name.ends_with(".wpp")
                        {
                            self.cancel_edit();
                            // FIXME handle error
                            
                            let reader = std::io::BufReader::new(std::io::Cursor::new(data));
                            
                            let new = cbor4ii::serde::from_reader(reader).map_err(|x| x.to_string()).unwrap();
                            self.load_from(new);
                        }
                        else
                        {
                            use std::io::Cursor;
                            use image::io::Reader as ImageReader;
                            // FIXME handle error
                            let img = ImageReader::new(Cursor::new(data)).with_guessed_format().unwrap().decode().unwrap().to_rgba8();
                            let img = Image::<4>::from_rgbaimage(&img);
                            self.load_from_img(img);
                        }
                        self.file_open_promise = None;
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    if let Some(Some(None)) = self.file_open_promise.as_ref().map(|x| x.ready())
                    {
                        self.file_open_promise = None;
                    }
                }
                ui.menu_button("Edit", |ui|
                {
                    if ui.button("Undo").clicked()
                    {
                        self.perform_undo();
                    }
                    if ui.button("Redo").clicked()
                    {
                        self.perform_redo();
                    }
                });
                ui.menu_button("View", |ui|
                {
                    if ui.button("Zoom In").clicked()
                    {
                        self.zoom(0.25);
                    }
                    if ui.button("Zoom Out").clicked()
                    {
                        self.zoom(-0.25);
                    }
                    if ui.button("Reset").clicked()
                    {
                        self.view_reset();
                    }
                });
            });
        });
        
        egui::TopBottomPanel::top("ToolBar").show(ctx, |ui|
        {
            egui::menu::bar(ui, |ui|
            {
                macro_rules! add_button { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                        $ui.add(egui::widgets::ImageButton::new(egui::load::SizedTexture::new(self.icons.get($icon).unwrap().0.id(), [18.0, 18.0])).selected($selected))
                           .on_hover_text($tooltip)
                } }
                
                if add_button!(ui, "undo", "UndoButton", false).clicked()
                {
                    self.perform_undo();
                }
                if add_button!(ui, "redo", "RedoButton", false).clicked()
                {
                    self.perform_redo();
                }
            });
        });
        
        let window_size = ctx.input(|i: &egui::InputState| i.screen_rect());
        
        macro_rules! layer_panel { ($in_bottom:expr) => { |ui : &mut Ui|
        {
                if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                {
                    let old_blend_mode = layer.blend_mode.clone();
                    egui::ComboBox::from_id_salt("blend_mode_dropdown")
                        .selected_text(&layer.blend_mode)
                        .width(150.0)
                        .show_ui(ui, |ui|
                    {
                        ui.selectable_value(&mut layer.blend_mode, "Normal".to_string(), "Normal");
                        ui.selectable_value(&mut layer.blend_mode, "Dither".to_string(), "Dither");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Darken".to_string(), "Darken");
                        ui.selectable_value(&mut layer.blend_mode, "Multiply".to_string(), "Multiply");
                        ui.selectable_value(&mut layer.blend_mode, "Color Burn".to_string(), "Color Burn");
                        ui.selectable_value(&mut layer.blend_mode, "Linear Burn".to_string(), "Linear Burn");
                        ui.selectable_value(&mut layer.blend_mode, "Subtract".to_string(), "Subtract");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Lighten".to_string(), "Lighten");
                        ui.selectable_value(&mut layer.blend_mode, "Screen".to_string(), "Screen");
                        ui.selectable_value(&mut layer.blend_mode, "Color Dodge".to_string(), "Color Dodge");
                        ui.selectable_value(&mut layer.blend_mode, "Glow Dodge".to_string(), "Glow Dodge");
                        ui.selectable_value(&mut layer.blend_mode, "Add".to_string(), "Add"); // aka linear dodge
                        ui.selectable_value(&mut layer.blend_mode, "Glow Add".to_string(), "Glow Add");
                        ui.selectable_value(&mut layer.blend_mode, "Divide".to_string(), "Divide");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Glow".to_string(), "Glow");
                        ui.selectable_value(&mut layer.blend_mode, "Reflect".to_string(), "Reflect");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Overlay".to_string(), "Overlay");
                        ui.selectable_value(&mut layer.blend_mode, "Soft Light".to_string(), "Soft Light");
                        ui.selectable_value(&mut layer.blend_mode, "Hard Light".to_string(), "Hard Light");
                        ui.selectable_value(&mut layer.blend_mode, "Vivid Light".to_string(), "Vivid Light");
                        ui.selectable_value(&mut layer.blend_mode, "Linear Light".to_string(), "Linear Light");
                        ui.selectable_value(&mut layer.blend_mode, "Pin Light".to_string(), "Pin Light");
                        ui.selectable_value(&mut layer.blend_mode, "Hard Mix".to_string(), "Hard Mix");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Signed Add".to_string(), "Signed Add");
                        ui.selectable_value(&mut layer.blend_mode, "Signed Diff".to_string(), "Signed Diff");
                        ui.selectable_value(&mut layer.blend_mode, "Negation".to_string(), "Negation");
                        ui.selectable_value(&mut layer.blend_mode, "Difference".to_string(), "Difference");
                        ui.selectable_value(&mut layer.blend_mode, "Exclusion".to_string(), "Exclusion");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Hue".to_string(), "Hue");
                        ui.selectable_value(&mut layer.blend_mode, "Saturation".to_string(), "Saturation");
                        ui.selectable_value(&mut layer.blend_mode, "Color".to_string(), "Color");
                        ui.selectable_value(&mut layer.blend_mode, "Brightness".to_string(), "Brightness");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Flat Hue".to_string(), "Flat Hue");
                        ui.selectable_value(&mut layer.blend_mode, "Flat Sat".to_string(), "Flat Sat");
                        ui.selectable_value(&mut layer.blend_mode, "Flat Color".to_string(), "Flat Color");
                        ui.selectable_value(&mut layer.blend_mode, "Value".to_string(), "Value");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Hard Sat".to_string(), "Hard Sat");
                        ui.selectable_value(&mut layer.blend_mode, "Hard Color".to_string(), "Hard Color");
                        ui.selectable_value(&mut layer.blend_mode, "Lightness".to_string(), "Lightness");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Erase".to_string(), "Erase");
                        ui.selectable_value(&mut layer.blend_mode, "Reveal".to_string(), "Reveal");
                        ui.selectable_value(&mut layer.blend_mode, "Alpha Mask".to_string(), "Alpha Mask");
                        ui.selectable_value(&mut layer.blend_mode, "Alpha Reject".to_string(), "Alpha Reject");
                        
                        ui.selectable_value(&mut layer.blend_mode, "Interpolate".to_string(), "Interpolate");
                        
                        ui.separator();
                        
                        ui.selectable_value(&mut layer.blend_mode, "Custom".to_string(), "Custom");
                        ui.selectable_value(&mut layer.blend_mode, "Custom Tri".to_string(), "Custom Tri");
                        ui.selectable_value(&mut layer.blend_mode, "Custom Quad".to_string(), "Custom Quad");
                    });
                    
                    let mut rerender = false;
                    if layer.blend_mode == "Custom" || layer.blend_mode == "Custom Tri" || layer.blend_mode == "Custom Quad"
                    {
                        //if layer.custom_blend_mode == ""
                        //{
                        //    layer.custom_blend_mode = "".to_string();
                        //}
                        egui::Window::new("Custom Blend Mode Editor").vscroll(true).show(ctx, |ui|
                        {
                            let editor = egui::TextEdit::multiline(&mut layer.custom_blend_mode).code_editor();
                            let res = ui.add_sized(ui.available_size(), editor);
                            if res.changed()
                            {
                                rerender = true;
                            }
                            if res.has_focus()
                            {
                                focus_is_global = false;
                            }
                        });
                    }
                    
                    let old_opacity = layer.opacity * 100.0;
                    let old_fill_opacity = layer.fill_opacity * 100.0;
                    let mut opacity = old_opacity;
                    let mut fill_opacity = old_fill_opacity;
                    let slider_response = ui.add(egui::Slider::new(&mut opacity, 0.0..=100.0).clamping(SliderClamping::Always));
                    let slider_response2 = ui.add(egui::Slider::new(&mut fill_opacity, 0.0..=100.0).clamping(SliderClamping::Always));
                    layer.opacity = opacity/100.0;
                    layer.fill_opacity = fill_opacity/100.0;
                    let id = layer.uuid;
                    
                    #[allow(clippy::if_same_then_else)]
                    
                    if old_blend_mode != layer.blend_mode
                    {
                        self.log_layer_info_change();
                        rerender = true;
                    }
                    else if old_opacity != opacity && !slider_response.dragged()
                    {
                        self.log_layer_info_change();
                    }
                    else if old_fill_opacity != fill_opacity && !slider_response2.dragged()
                    {
                        self.log_layer_info_change();
                    }
                    else if slider_response.drag_stopped()
                    {
                        println!("making undo for opacity");
                        self.log_layer_info_change();
                    }
                    
                    if old_opacity != opacity || old_fill_opacity != fill_opacity || rerender
                    {
                        self.full_rerender_with(id);
                    }
                }
                else
                {
                    egui::ComboBox::from_id_salt("blend_mode_dropdown").selected_text("").show_ui(ui, |_ui|{});
                    
                    let mut opacity = 0.0;
                    ui.add_enabled(false, egui::Slider::new(&mut opacity, 0.0..=100.0).clamping(SliderClamping::Always));
                    
                }
        
                macro_rules! add_button { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                        $ui.add(egui::widgets::ImageButton::new(egui::load::SizedTexture::new(self.icons.get($icon).unwrap().0.id(), [18.0, 18.0])).selected($selected))
                           .on_hover_text($tooltip)
                } }
                macro_rules! add_button_disabled { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                        $ui.add_enabled(false, egui::widgets::ImageButton::new(egui::load::SizedTexture::new(self.icons.get($icon).unwrap().0.id(), [18.0, 18.0])).selected($selected))
                           .on_hover_text($tooltip)
                } }
                
                
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true), |ui|
                {
                    ui.spacing_mut().item_spacing = [1.0, 0.0].into();
                    ui.spacing_mut().button_padding = [0.0, 0.0].into();
                    
                    let layer = self.layers.find_layer_mut(self.current_layer);
                    let funny_flag   = layer.as_ref().map_or(false, |layer| layer.funny_flag  );
                    let visible      = layer.as_ref().map_or(false, |layer| layer.visible     );
                    let clipped      = layer.as_ref().map_or(false, |layer| layer.clipped     );
                    let locked       = layer.as_ref().map_or(false, |layer| layer.locked      );
                    let alpha_locked = layer.as_ref().map_or(false, |layer| layer.alpha_locked);
                    
                    if add_button!(ui, "visible_big", "Toggle Visibility", visible).clicked()
                    {
                        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                        {
                            layer.visible = !layer.visible;
                            let id = layer.uuid;
                            self.full_rerender_with(id);
                            self.log_layer_info_change();
                        }
                    }
                    if add_button!(ui, "clipping mask", "Toggle Clipping Mask", clipped).clicked()
                    {
                        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                        {
                            layer.clipped = !layer.clipped;
                            let id = layer.uuid;
                            self.full_rerender_with(id);
                            self.log_layer_info_change();
                        }
                    }
                    if add_button!(ui, "lock", "Toggle Layer Lock", locked).clicked()
                    {
                        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                        {
                            layer.locked = !layer.locked;
                            self.log_layer_info_change();
                        }
                    }
                    if add_button!(ui, "lock alpha", "Toggle Alpha Lock", alpha_locked).clicked()
                    {
                        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                        {
                            layer.alpha_locked = !layer.alpha_locked;
                            self.log_layer_info_change();
                        }
                    }
                    
                    if add_button!(ui, "funny alpha", "Funny Alpha Flag", funny_flag).clicked()
                    {
                        if let Some(layer) = self.layers.find_layer_mut(self.current_layer)
                        {
                            layer.funny_flag = !layer.funny_flag;
                            let id = layer.uuid;
                            self.full_rerender_with(id);
                            self.log_layer_info_change();
                        }
                    }
                });
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true), |ui|
                {
                    ui.spacing_mut().item_spacing = [1.0, 0.0].into();
                    ui.spacing_mut().button_padding = [0.0, 0.0].into();
                    if add_button!(ui, "new layer", "New Layer", false).clicked()
                    {
                        self.new_layer();
                        self.full_rerender();
                    }
                    if add_button!(ui, "new group", "New Group", false).clicked()
                    {
                        self.layers.add_group(self.current_layer);
                        self.full_rerender();
                    }
                    if add_button!(ui, "into group", "Into New Group", false).clicked()
                    {
                        self.layers.move_into_new_group(self.current_layer);
                        self.full_rerender();
                    }
                    if add_button_disabled!(ui, "duplicate layer", "Duplicate Layer", false).clicked()
                    {
                        // FIXME/TODO
                    }
                    if add_button!(ui, "move layer up", "Move Layer Up", false).clicked()
                    {
                        self.layers.move_layer_up(self.current_layer);
                        self.full_rerender();
                    }
                    if add_button!(ui, "move layer down", "Move Layer Down", false).clicked()
                    {
                        self.layers.move_layer_down(self.current_layer);
                        self.full_rerender();
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
                        self.full_rerender();
                    }
                });
                
                ui.separator();
                
                if $in_bottom { egui::ScrollArea::horizontal() } else { egui::ScrollArea::both() }.auto_shrink([false, false]).show(ui, |ui|
                {
                    let mut layer_info = vec!();
                    for c in self.layers.children.iter_mut()
                    {
                        c.visit_layers_mut(0, &mut |layer, depth|
                        {
                            let thumb_img = if layer.data.is_some() { &layer.data } else { &layer.flattened_data }.as_ref().map(|x| x.make_thumbnail());
                            let mut th_outer = None;
                            if let Some(thumb_img) = thumb_img
                            {
                                use uuid::Uuid;
                                if let Some(ref mut tn) = layer.thumbnail
                                {
                                    let tex = tn.to_mut::<egui::TextureHandle>().unwrap();
                                    th_outer = Some(tex.clone());
                                    tex.set(thumb_img.to_egui(), egui::TextureOptions::NEAREST);
                                }
                                else
                                {
                                    let ctx = ui.ctx();
                                    let nd = thumb_img.to_egui();
                                    let tex = ctx.load_texture(format!("{}", Uuid::new_v4().as_u128()), nd, egui::TextureOptions::NEAREST);
                                    th_outer = Some(tex.clone());
                                    layer.thumbnail = Some(Box::new(tex));
                                }
                            }
                            
                            layer_info.push((
                                layer.name.clone(),
                                layer.uuid,
                                depth,
                                layer.mask.is_some(),
                                layer.clipped,
                                layer.children.len(),
                                layer.visible,
                                th_outer,
                            ));
                            Some(())
                        });
                    }
                    
                    macro_rules! add_button { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                            $ui.add(egui::widgets::ImageButton::new(egui::load::SizedTexture::new(self.icons.get($icon).unwrap().0.id(), [14.0, 14.0])).selected($selected))
                               .on_hover_text($tooltip)
                    } }
                    
                    ui.style_mut().spacing.item_spacing.y = 1.0;
                    for info in layer_info
                    {
                        ui.horizontal(|ui|
                        {
                            ui.style_mut().spacing.item_spacing.x = 3.0;
                            if add_button!(ui, if info.6 { "visible" } else { "invisible" }, "Toggle Visibility", false).clicked()
                            {
                                let mut layer = self.layers.find_layer_mut(info.1);
                                let layer = layer.as_mut().unwrap();
                                layer.visible = !layer.visible;
                                let id = layer.uuid;
                                self.full_rerender_with(id);
                                self.log_layer_info_change();
                            }
                            ui.allocate_space([info.2 as f32 * 8.0, 0.0].into());
                            let mut name = info.0.clone();
                            
                            let active = self.current_layer == info.1;
                            ui.allocate_ui([150.0, 28.0].into(), |ui|
                            {
                                let mut stroke : egui::Stroke = (1.0, ui.style().visuals.widgets.active.weak_bg_fill).into();
                                if active
                                {
                                    //stroke = egui::Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(64, 64, 192, 255));
                                    stroke = (2.0, ui.style().visuals.widgets.active.fg_stroke.color).into();
                                }
                                if Frame::group(ui.style()).corner_radius(1.0).inner_margin(Margin::symmetric(4, 0))
                                .stroke(stroke)
                                .show(ui, |ui|
                                {
                                    let mut clicked = false;
                                    if info.4
                                    {
                                        let mut rect = ui.max_rect();
                                        rect.min.y -= 1.0;
                                        rect.max.y += 1.0;
                                        rect.min.x -= 4.0;
                                        rect.max.x = rect.min.x + 2.0;
                                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(0, 127, 255, 255));
                                    }
                                    if info.5 > 0
                                    {
                                        clicked |= ui.add(egui::widgets::Image::new(egui::load::SizedTexture::new(
                                            self.icons.get("icon group").unwrap().0.id(), [14.0, 14.0]
                                        )).sense(egui::Sense::click_and_drag())).clicked();
                                    }
                                    
                                    if info.3
                                    {
                                        name += "[m]";
                                    }
                                    
                                    if let Some(tx) = info.7
                                    {
                                        let (r, painter) = ui.allocate_painter((24.0, 24.0).into(), egui::Sense::click());
                                        let mut rect = r.rect;
                                        rect.max.x = rect.min.x + 6.0;
                                        rect.max.y = rect.min.y + 6.0;
                                        for y in 0..4
                                        {
                                            for x in 0..4
                                            {
                                                let c = ((x^y)&1) == 0;
                                                let c = if c { egui::Color32::GRAY } else { egui::Color32::LIGHT_GRAY };
                                                painter.rect_filled(rect.translate((x as f32 * 6.0, y as f32 * 6.0).into()), 0.0, c);
                                            }
                                        }
                                        let mut rect = r.rect;
                                        rect.max.x = rect.min.x + 24.0;
                                        rect.max.y = rect.min.y + 24.0;
                                        painter.image(tx.id(), rect, [(0.0, 0.0).into(), (1.0, 1.0).into()].into(), egui::Color32::WHITE);
                                        clicked |= r.clicked();
                                    }
                                    clicked |= ui.add(egui::Label::new(egui::RichText::new(&name).size(10.0)).selectable(false).sense(egui::Sense::click_and_drag())).clicked();
                                    
                                    let response = ui.response();
                                    clicked |= response.clicked();
                                    
                                    if clicked
                                    {
                                        self.current_layer = info.1;
                                    }
                                    
                                    response
                                }).response.interact(egui::Sense::click_and_drag()).clicked()
                                {
                                    self.current_layer = info.1;
                                }
                            });
                        });
                    }
                });
            }
        }}
        
        let layers_on_right = window_size.size().x >= 900.0;
        let sidebars_on_bottom = window_size.size().x < 700.0;
        
        if layers_on_right
        {
            egui::SidePanel::right("RightPanel").show(ctx, |ui|
            {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, layer_panel!(false));
            });
        }
        
        egui::SidePanel::left("ToolPanel").min_width(22.0).default_width(22.0).show(ctx, |ui|
        {
            macro_rules! add_button { ($ui:expr, $icon:expr, $tooltip:expr, $selected:expr) => {
                    $ui.add(egui::widgets::ImageButton::new(egui::load::SizedTexture::new(self.icons.get($icon).unwrap().0.id(), [22.0, 22.0])).selected($selected))
                       .on_hover_text($tooltip)
            } }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui|
            {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT).with_main_wrap(true), |ui|
                {
                    ui.spacing_mut().button_padding = [0.0, 0.0].into();
                    let prev_tool = self.current_tool;
                    if add_button!(ui, "tool pencil", "Pencil Tool", self.current_tool == 0).clicked()
                    {
                        self.current_tool = 0;
                    }
                    if add_button!(ui, "tool eraser", "Eraser Tool", self.current_tool == 1).clicked()
                    {
                        self.current_tool = 1;
                    }
                    if add_button!(ui, "tool line", "Line Tool", self.current_tool == 2).clicked()
                    {
                        self.current_tool = 2;
                    }
                    if add_button!(ui, "tool fill", "Fill Tool", self.current_tool == 3).clicked()
                    {
                        self.current_tool = 3;
                    }
                    if add_button!(ui, "tool eyedropper", "Eyedropper Tool", self.current_tool == 4).clicked()
                    {
                        self.current_tool = 4;
                    }
                    if add_button!(ui, "tool select", "Selection Tool", self.current_tool == 5).clicked()
                    {
                        self.current_tool = 5;
                    }
                    if add_button!(ui, "tool move", "Move Tool", self.current_tool == 6).clicked()
                    {
                        self.current_tool = 6;
                    }
                    if self.current_tool != prev_tool
                    {
                        self.tool_notify_changed(prev_tool);
                    }
                });
            });
        });
        
        macro_rules! toolsettings { () => { |ui|
        {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui|
            {
                let rgbainfotext = format!("{} {} {} {} / {} {} {}",
                    (self.main_color_rgb[0] * 255.0) as u8,
                    (self.main_color_rgb[1] * 255.0) as u8,
                    (self.main_color_rgb[2] * 255.0) as u8,
                    (self.main_color_rgb[3] * 255.0) as u8,
                    (self.main_color_hsv[0]) as u8,
                    (self.main_color_hsv[1] * 100.0) as u8,
                    (self.main_color_hsv[2] * 100.0) as u8,
                );
                if !sidebars_on_bottom
                {
                    ui.add(egui::Label::new(egui::RichText::new(&rgbainfotext).size(9.0)).selectable(false)).clicked();
                    
                    let mut a = self.main_color_rgb[3];
                    ui.add(|ui : &mut egui::Ui| bar_picker(ui, self, sidebars_on_bottom, 0.0, self.main_color_rgb, &mut a));
                    a = a.clamp(0.0, 1.0);
                    self.main_color_rgb[3] = a;
                    
                    ui.add(|ui : &mut egui::Ui| color_picker(ui, self, sidebars_on_bottom));
                    ui.separator();
                }
                
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui|
                {
                    egui::ScrollArea::vertical().show(ui, |ui|
                    {
                        if !layers_on_right
                        {
                            egui::ScrollArea::horizontal().show(ui, |ui|
                            {
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::LEFT), |ui|
                                {
                                    if sidebars_on_bottom
                                    {
                                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui|
                                        {
                                            ui.add(|ui : &mut egui::Ui| color_picker(ui, self, sidebars_on_bottom));
                                            
                                            let mut a = self.main_color_rgb[3];
                                            ui.add(|ui : &mut egui::Ui| bar_picker(ui, self, sidebars_on_bottom, 0.0, self.main_color_rgb, &mut a));
                                            a = a.clamp(0.0, 1.0);
                                            self.main_color_rgb[3] = a;
                                            
                                            ui.add(egui::Label::new(egui::RichText::new(&rgbainfotext).size(9.0)).selectable(false)).clicked();
                                        });
                                    }
                                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui|
                                    {
                                        ui.label("Tool Settings");
                                        ui.separator();
                                    
                                        self.tool_panel(ui);
                                    });
                                });
                            });
                        }
                        else
                        {
                            self.tool_panel(ui);
                        }
                        
                        if !layers_on_right
                        {
                            ui.separator();
                            ui.label("Layers");
                            ui.separator();
                            layer_panel!(true)(ui);
                        }
                    });
                });
            });
        }}}
        
        egui::TopBottomPanel::bottom("DebugText").resizable(true).min_height(16.0).max_height(150.0).show(ctx, |ui|
        {
            egui::ScrollArea::vertical().auto_shrink([false, false]).min_scrolled_height(16.0).stick_to_bottom(true).show(ui, |ui|
            {
                if self.debug_text.len() > 500
                {
                    self.debug_text.drain(0..self.debug_text.len()-500);
                }
                let mut text = self.debug_text.join("\n");
                ui.add_enabled(false, egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).desired_rows(1).min_size([16.0, 16.0].into()).hint_text("debug output"));
            });
        });
        
        if !sidebars_on_bottom
        {
            egui::SidePanel::left("ToolSettings").show(ctx, toolsettings!());
        }
        else
        {
            egui::TopBottomPanel::bottom("ToolSettingsTB").resizable(true).min_height(16.0).max_height(500.0).default_height(180.0).show(ctx, toolsettings!());
        }
        
        if focus_is_global
        {
            ctx.input_mut(|state|
            {
                let shortcut_undo = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Z);
                let shortcut_redo_a = egui::KeyboardShortcut::new(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::Z);
                let shortcut_redo_b = egui::KeyboardShortcut::new(egui::Modifiers::CTRL, egui::Key::Y);
                
                if state.consume_shortcut(&shortcut_undo)
                {
                    self.perform_undo();
                }
                if state.consume_shortcut(&shortcut_redo_a)
                {
                    self.perform_redo();
                }
                if state.consume_shortcut(&shortcut_redo_b)
                {
                    self.perform_redo();
                }
                
                // FIXME support clipboard on web
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // FIXME go back to ctrl when egui adds a way to not block clipboard shortcuts
                    let shortcut_paste = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::V);
                    if state.consume_shortcut(&shortcut_paste)
                    {
                        println!("b");
                        if let Ok(mut clipboard) = arboard::Clipboard::new()
                        {
                            println!("c");
                            if let Ok(image_data) = clipboard.get().image()
                            {
                                if self.is_editing()
                                {
                                    self.commit_edit();
                                }
                                
                                // FIXME undo/redo for layer operations
                                
                                println!("d");
                                self.new_layer();
                                let data = self.get_current_layer_data().unwrap();
                                
                                let w = image_data.width;
                                let h = image_data.height;
                                let pixels = image_data.bytes.chunks(4).map(|x| [x[0], x[1], x[2], x[3]]).collect::<Vec<_>>();
                                for y in 0..h
                                {
                                    for x in 0..w
                                    {
                                        data.set_pixel(x as isize, y as isize, pixels[y*w + x]);
                                    }
                                }
                                self.full_rerender();
                                
                                // FIXME undo/redo for layer operations
                            }
                        }
                    }
                    let shortcut_paste = egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::C);
                    if state.consume_shortcut(&shortcut_paste)
                    {
                        println!("b");
                        if let Ok(mut clipboard) = arboard::Clipboard::new()
                        {
                            println!("c");
                            if let Ok(image_data) = clipboard.get().image()
                            {
                                if self.is_editing()
                                {
                                    self.commit_edit();
                                }
                                
                                // FIXME undo/redo for layer operations
                                
                                println!("d");
                                self.new_layer();
                                let data = self.get_current_layer_data().unwrap();
                                
                                let w = image_data.width;
                                let h = image_data.height;
                                let pixels = image_data.bytes.chunks(4).map(|x| [x[0], x[1], x[2], x[3]]).collect::<Vec<_>>();
                                for y in 0..h
                                {
                                    for x in 0..w
                                    {
                                        data.set_pixel(x as isize, y as isize, pixels[y*w + x]);
                                    }
                                }
                                self.full_rerender();
                                
                                // FIXME undo/redo for layer operations
                            }
                        }
                    }
                }
            });
        }
        
        let frame = egui::Frame {
            inner_margin: egui::Margin::same(0),
            //rounding: egui::Rounding::ZERO,
            fill: ctx.style().visuals.window_fill(),
            stroke: Default::default(),
            ..Default::default()
        };
        
        let mut input_state = None;
        egui::CentralPanel::default().frame(frame).show(ctx, |ui|
        {
            ui.spacing_mut().window_margin = 0.0.into();
            ui.add(|ui : &mut egui::Ui|
            {
                let (response, state) = canvas(ui, self, focus_is_global);
                input_state = Some(state);
                response
            });
        });
        
        // set cursor (hardware on web, software on desktop)
        
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let root : web_sys::HtmlElement = document.get_element_by_id("the_canvas_id").unwrap().dyn_into().unwrap();
            
            root.style().set_property("cursor", "unset").unwrap();
        }
        
        if let (Some(tool), Some(input_state)) = (self.get_tool(), input_state)
        {
            if input_state.mouse_in_canvas_area
            {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some((cursor, offset)) = tool.get_cursor(self)
                    {
                        ctx.set_cursor_icon(egui::CursorIcon::None);
                        let painter = ctx.debug_painter();
                        let uv = [[0.0, 0.0].into(), [1.0, 1.0].into()].into();
                        let mut pos : egui::Rect = [[0.0, 0.0].into(), cursor.0.size_vec2().to_pos2()].into();
                        pos = pos.translate(input_state.window_mouse_coord.into());
                        pos = pos.translate([-offset[0], -offset[1]].into());
                        painter.image(cursor.0.id(), pos, uv, egui::Color32::WHITE);
                    }
                }
                
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some((cursor, offset)) = tool.get_cursor(self)
                    {
                        let image = cursor.1.to_imagebuffer();
                        
                        let mut bytes = Vec::new();
                        
                        use image::ImageEncoder;
                        image::codecs::png::PngEncoder::new(&mut bytes).write_image(
                            image::DynamicImage::from(image).as_flat_samples_u8().unwrap().samples,
                            cursor.1.width as u32,
                            cursor.1.height as u32,
                            image::ColorType::Rgba8,
                        ).unwrap();
                        
                        use base64::Engine;
                        let encoded : String = base64::engine::general_purpose::STANDARD_NO_PAD.encode(bytes);
                        
                        use wasm_bindgen::JsCast;
                        
                        let window = web_sys::window().unwrap();
                        let document = window.document().unwrap();
                        let root : web_sys::HtmlElement = document.get_element_by_id("the_canvas_id").unwrap().dyn_into().unwrap();
                        
                        root.style().set_property("cursor", &format!("url(data:image/png;base64,{}) {} {}, crosshair", encoded, offset[0] as usize, offset[1] as usize)).unwrap();
                    }
                }
            }
        }
        // DON'T USE; BUGGY / REENTRANT / CAUSES CRASH (in egui/eframe 0.19.0 at least)
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
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

#[allow(clippy::field_reassign_with_default)]
#[cfg(not(target_arch = "wasm32"))]
fn main()
{
    let mut options = eframe::NativeOptions::default();
    
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("data/warpaint logo.png")).unwrap();
    
    // eframe 0.19.0 is borked on windows 10, the window flickers violently when you resize it, flashing white
    // this is a seizure hazard when using the dark theme, so force the light theme instead
    
    //options.follow_system_theme = false;
    //options.default_theme = Theme::Light;
    //options.initial_window_size = Some([1280.0, 720.0].into());
    options.viewport = egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]).with_drag_and_drop(true);
    options.viewport.icon = Some(Arc::new(icon));
    options.multisampling = 4;
    
    let mut wp = Box::<Warpainter>::default();
    
    let fname = std::env::args().nth(1).unwrap_or_default();
    
    if fname.ends_with(".psd")
    {
        let bytes = std::fs::read(fname).unwrap();
        wpsd_open(&mut *wp, &bytes);
    }
    else if fname.ends_with(".wpp")
    {
        let bytes = std::fs::read(&fname).unwrap();
        let start = web_time::Instant::now();
        
        fn load(path : &String) -> Result<Warpainter, String>
        {
            let file = std::fs::File::open(path).map_err(|x| x.to_string())?;
            let reader = std::io::BufReader::new(file);
            let data : Warpainter = cbor4ii::serde::from_reader(reader).map_err(|x| x.to_string())?;
            Ok(data)
        }
        
        let new = load(&fname).unwrap();
        wp.load_from(new);
        println!("WPP load time: {:.3}", start.elapsed().as_secs_f64() * 1000.0);
    }
    else if fname != ""
    {
        // FIXME handle error
        let img = image::io::Reader::open(fname).unwrap().decode().unwrap().to_rgba8();
        let img = Image::<4>::from_rgbaimage(&img);
        wp.load_from_img(img);
    }
    
    eframe::run_native (
        "Warpainter",
        options,
        Box::new(|_ctx|
        {
            //_ctx.egui_ctx.set_theme(egui::Theme::Dark);
            Ok(wp)
        }),
    ).unwrap();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn test(data : String, mime : String) {
    alert(&format!("attempted paste. data: {} mime: {}", data, mime));
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main()
{
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();
    
    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();
    
    web_sys::console::log_1(&format!("hello from non-async land!").into());
    
    async fn realmain()
    {
        web_sys::console::log_1(&format!("hello from async land!").into());
        
        #[wasm_bindgen]
        pub async fn init_threads()
        {
            web_sys::console::log_1(&format!("hello from super async land!").into());
            
            use wasm_bindgen::prelude::*;
            
            #[wasm_bindgen]
            extern "C"
            {
                type Navigator;
                
                #[wasm_bindgen(method, getter, js_name = hardwareConcurrency)]
                fn hardware_concurrency(this : &Navigator) -> f64;
                
                #[wasm_bindgen(thread_local_v2, js_namespace = globalThis, js_name = navigator)]
                static NAV: Navigator;
            }

            #[wasm_bindgen]
            pub fn get_cores() -> f64
            {
                NAV.with(|nav| nav.hardware_concurrency())
            }
            
            let count = get_cores();
            web_sys::console::log_1(&format!("initializing threadpool with count: {}", count).into());
            use wasm_bindgen_rayon::init_thread_pool;
            wasm_bindgen_futures::JsFuture::from(init_thread_pool(count as usize)).await.unwrap();
            web_sys::console::log_1(&format!("threadpool initialized! threads: {}", count).into());
        }
        init_threads().await;
        
        let mut web_options = eframe::WebOptions::default();
        
        web_sys::console::log_1(&format!("event received").into());
        
        wasm_bindgen_futures::spawn_local(async
        {
            use wasm_bindgen::JsCast;
            
            let document = web_sys::window().unwrap().document().unwrap();
            
            r#"import { greet } from "./hello_world";
                greet("World!"); "#;
            
            let canvas = document
                .get_element_by_id("the_canvas_id").unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
            
            let start_result = eframe::WebRunner::new().start(
                canvas,
                web_options,
                Box::new(|_| Ok(Box::new(Warpainter::default()))),
            ).await;
            
            // Remove the loading text and spinner:
            if let Some(loading_text) = document.get_element_by_id("loading_text")
            {
                match start_result {
                    Ok(_) => loading_text.remove(),
                    Err(e) =>
                    {
                        loading_text.set_inner_html("<p> The app has crashed. See the developer console for details. </p>");
                        panic!("Failed to start eframe: {e:?}");
                    }
                }
            }
        });
    }
    
    wasm_bindgen_futures::spawn_local(async {
        realmain().await;
    });
}