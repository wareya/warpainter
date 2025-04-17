use eframe::egui;

use std::collections::HashMap;

use crate::LayerPaint;
use crate::UndoEvent;
use crate::pixelmath::*;
use crate::transform::*;
use crate::wpsd_raw::MaskInfo;
use crate::Adjustment;
use crate::spline::*;

/*
// for flattened slices. not used right now
#[inline]
pub (crate) fn get_pixel<T : Copy>(data : &[T], index : usize) -> [T; 4]
{
    [data[index], data[index+1], data[index+2], data[index+3]]
}
#[inline]
pub (crate) fn set_pixel<T : Copy>(data : &mut [T], index : usize, pixel : [T; 4])
{
    data[index+0] = pixel[0];
    data[index+1] = pixel[1];
    data[index+2] = pixel[2];
    data[index+3] = pixel[3];
}
*/

fn flatten<T : Copy, const N : usize>(a : &[[T; N]]) -> Vec<T>
{
    let mut ret = Vec::with_capacity(N*a.len());
    for sub in a.iter()
    {
        for val in sub.iter()
        {
            ret.push(*val);
        }
    }
    ret
}

use bincode::{Decode, Encode};
use serde::{Serialize, Deserialize};
use serde_with::serde_as;
#[serde_as]
#[derive(Clone, Debug, Decode, Encode, Serialize, Deserialize)]
pub (crate) enum ImageData<const N : usize>
{
    Float(
        #[serde_as(as = "Vec<[_; N]>")]
        Vec<[f32; N]>),
    Int(
        #[serde_as(as = "Vec<[_; N]>")]
        Vec<[u8; N]>),
}

impl<const N: usize> Default for ImageData<N>
{
    fn default() -> Self
    {
        ImageData::Int(Vec::new())
    }
}

impl<const N : usize> ImageData<N>
{
    fn new_float(w : usize, h : usize) -> Self
    {
        Self::Float(vec!([0.0; N]; w*h))
    }
    fn new_int(w : usize, h : usize) -> Self
    {
        Self::Int(vec!([0; N]; w*h))
    }
    fn to_int(&self) -> Vec<u8>
    {
        match self
        {
            Self::Int(data) => flatten(data),
            Self::Float(data) =>
            {
                let mut out = vec!([0; N]; data.len());
                for i in 0..data.len()
                {
                    out[i] = px_to_int(data[i]);
                }
                flatten(&out)
            }
        }
    }
    fn into_int(self) -> Vec<u8>
    {
        self.to_int()
    }
}

// always RGBA
#[derive(Clone, Debug, Default, Decode, Encode, Serialize, Deserialize)]
pub (crate) struct Image<const N : usize>
{
    pub (crate) width : usize,
    pub (crate) height : usize,
    data : ImageData<N>,
}

impl<const N : usize> Image<N>
{
    #[inline]
    pub (crate) fn bytes(&self) -> &[u8]
    {
        use byte_slice_cast::*;
        let bytes = match &self.data
        {
            ImageData::Int(data) => data[..].as_byte_slice(),
            ImageData::Float(data) => data[..].as_byte_slice(),
        };
        bytes
    }
    #[inline]
    pub (crate) fn is_float(&self) -> bool
    {
        match &self.data
        {
            ImageData::Int(_) => false,
            ImageData::Float(_) => true,
        }
    }
    #[inline]
    pub (crate) fn is_int(&self) -> bool
    {
        match &self.data
        {
            ImageData::Int(_) => true,
            ImageData::Float(_) => false,
        }
    }
}

impl<const N : usize> Image<N>
{
    #[inline]
    pub (crate) fn set_pixel_wrapped(&mut self, x : isize, y : isize, px : [u8; N])
    {
        let x = (x % self.width as isize) as usize;
        let y = (y % self.height as isize) as usize;
        let index = y*self.width + x;
        match &mut self.data
        {
            ImageData::Int(data) =>
                data[index] = px,
            ImageData::Float(data) =>
                data[index] = px_to_float(px),
        }
    }
    #[inline]
    pub (crate) fn set_pixel(&mut self, x : isize, y : isize, px : [u8; N])
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return;
        }
        self.set_pixel_wrapped(x, y, px)
    }
    #[inline]
    pub (crate) fn set_pixel_float_wrapped(&mut self, x : isize, y : isize, px : [f32; N])
    {
        let x = (x % self.width as isize) as usize;
        let y = (y % self.height as isize) as usize;
        let index = y*self.width + x;
        match &mut self.data
        {
            ImageData::Int(data) =>
                data[index] = px_to_int(px),
            ImageData::Float(data) =>
                data[index] = px,
        }
    }
    #[inline]
    pub (crate) fn set_pixel_float(&mut self, x : isize, y : isize, px : [f32; N])
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return;
        }
        self.set_pixel_float_wrapped(x, y, px)
    }
    
    
    #[inline]
    pub (crate) fn get_pixel_wrapped(&self, x : isize, y : isize) -> [u8; N]
    {
        let x = (x % self.width as isize) as usize;
        let y = (y % self.height as isize) as usize;
        let index = y*self.width + x;
        match &self.data
        {
            ImageData::Int(data) => data[index],
            ImageData::Float(data) => px_to_int(data[index]),
        }
    }
    #[inline]
    pub (crate) fn get_pixel(&self, x : isize, y : isize) -> [u8; N]
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return [0; N];
        }
        self.get_pixel_wrapped(x, y)
    }
    #[inline]
    pub (crate) fn get_pixel_float_wrapped(&self, x : isize, y : isize) -> [f32; N]
    {
        if self.width == 0 || self.height == 0
        {
            return [0.0; N];
        }
        let x = (x % self.width as isize) as usize;
        let y = (y % self.height as isize) as usize;
        let index = y*self.width + x;
        match &self.data
        {
            ImageData::Int(data) => px_to_float(data[index]),
            ImageData::Float(data) => data[index],
        }
    }
    #[inline]
    pub (crate) fn get_pixel_float_default(&self, x : isize, y : isize, default : f32) -> [f32; N]
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return [default; N];
        }
        let index = y as usize*self.width + x as usize;
        match &self.data
        {
            ImageData::Int(data) => px_to_float(data[index]),
            ImageData::Float(data) => data[index],
        }
    }
    #[inline]
    pub (crate) fn get_pixel_float(&self, x : isize, y : isize) -> [f32; N]
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return [0.0; N];
        }
        self.get_pixel_float_wrapped(x, y)
    }
}

fn nop<T>(t : T) -> T
{
    t
}

fn get_thread_count() -> usize
{
    let mut thread_count = 4;
    if let Some(count) = std::thread::available_parallelism().ok()
    {
        thread_count = count.get();
    }
    thread_count
}
use std::sync::OnceLock;
static THREAD_POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
fn get_pool() -> &'static rayon::ThreadPool
{
    THREAD_POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(get_thread_count()).build().unwrap())
}

impl<const N : usize> Image<N>
{
    pub (crate) fn clone_grown(&self, px : usize) -> Self
    {
        match &self.data
        {
            ImageData::Int(data) =>
            {
                let mut outdata = Vec::new();
                for _ in 0..self.height + px
                {
                    for _ in 0..self.width + px * 2
                    {
                        outdata.push([0; N]);
                    }
                }
                for y in 0..self.height
                {
                    for _ in 0..px
                    {
                        outdata.push([0; N]);
                    }
                    let iy = y * self.width;
                    for x in 0..self.width
                    {
                        outdata.push(data[iy+x]);
                    }
                    for _ in 0..px
                    {
                        outdata.push([0; N]);
                    }
                }
                for _ in 0..self.height + px
                {
                    for _ in 0..self.width + px * 2
                    {
                        outdata.push([0; N]);
                    }
                }
                Image::<N> { width : self.width + px*2, height : self.height + px*2, data : ImageData::Int(outdata) }
            }
            ImageData::Float(data) =>
            {
                let mut outdata = Vec::new();
                for _ in 0..px
                {
                    for _ in 0..px * 2
                    {
                        outdata.push([0.0; N]);
                    }
                }
                for y in 0..self.height
                {
                    for _ in 0..px
                    {
                        outdata.push([0.0; N]);
                    }
                    let iy = y * self.width;
                    for x in 0..self.width
                    {
                        outdata.push(data[iy+x]);
                    }
                    for _ in 0..px
                    {
                        outdata.push([0.0; N]);
                    }
                }
                for _ in 0..px
                {
                    for _ in 0..px * 2
                    {
                        outdata.push([0.0; N]);
                    }
                }
                Image::<N> { width : self.width + px*2, height : self.height + px*2, data : ImageData::Float(outdata) }
            }
        }
    }
    pub (crate) fn alike_grown(&self, px : usize) -> Self
    {
        match &self.data
        {
            ImageData::Int(_) =>
            {
                let outdata = vec!([0; N]; (self.height + px * 2) * (self.width + px * 2));
                Image::<N> { width : self.width + px*2, height : self.height + px*2, data : ImageData::Int(outdata) }
            }
            ImageData::Float(_) =>
            {
                let outdata = vec!([0.0; N]; (self.height + px * 2) * (self.width + px * 2));
                Image::<N> { width : self.width + px*2, height : self.height + px*2, data : ImageData::Float(outdata) }
            }
        }
    }
    pub (crate) fn alike(&self) -> Self
    {
        self.alike_grown(0)
    }
    pub (crate) fn clone_cleared_outside(&self, rect : [[f32; 2]; 2]) -> Self
    {
        let mut ret = self.alike();
        match (&self.data, &mut ret.data)
        {
            (ImageData::Int(selfdat), ImageData::Int(ref mut retdat)) =>
            {
                for y in rect[0][1] as usize..rect[1][1] as usize
                {
                    for x in rect[0][0] as usize..rect[1][0] as usize
                    {
                        retdat[y*self.width + x] = selfdat[y*self.width + x];
                    }
                }
            }
            (ImageData::Float(selfdat), ImageData::Float(ref mut retdat)) =>
            {
                for y in rect[0][1] as usize..rect[1][1] as usize
                {
                    for x in rect[0][0] as usize..rect[1][0] as usize
                    {
                        retdat[y*self.width + x] = selfdat[y*self.width + x];
                    }
                }
            }
            _ => panic!()
        }
        ret
    }
    pub (crate) fn make_thumbnail(&self) -> Self
    {
        let size = 24;
        let data = ImageData::<N>::new_int(size, size);
        let mut ret = Self { width : size, height : size, data };
        let d = 1.0 / (size + 1) as f32;
        let q = self.height.max(self.width) as f32;
        let d2 = (d*q*0.5) as isize;
        for y in 0..size
        {
            for x in 0..size
            {
                let fy = (y as f32 + 0.25) / (size) as f32;
                let fx = (x as f32 + 0.25) / (size) as f32;
                
                let mut y2 = (fy * q) as isize;
                let mut x2 = (fx * q) as isize;
                y2 += (self.height - self.height.max(self.width)) as isize / 2;
                x2 += (self.width  - self.height.max(self.width)) as isize / 2;
                
                let mut c = self.get_pixel(x2, y2);
                let c2 = self.get_pixel(x2 + d2, y2);
                let c3 = self.get_pixel(x2, y2 + d2);
                let c4 = self.get_pixel(x2 + d2, y2 + d2);
                
                for i in 0..N
                {
                    c[i] = ((c[i] as u32 + c2[i] as u32 + c3[i] as u32 + c4[i] as u32) / 4) as u8;
                }
                
                ret.set_pixel(x as isize, y as isize, c);
            }
        }
        ret
    }
}

impl Image<1>
{
    pub (crate) fn from_yimage(input : &image::GrayImage, inverted : bool) -> Self
    {
        let (w, h) = input.dimensions();
        let data = ImageData::<1>::new_int(w as usize, h as usize);
        let mut ret = Self { width : w as usize, height : h as usize, data };
        for y in 0..ret.height
        {
            for x in 0..ret.width
            {
                let mut px = input.get_pixel(x as u32, y as u32).0;
                if inverted
                {
                    px[0] = 255 - px[0];
                }
                ret.set_pixel(x as isize, y as isize, px);
            }
        }
        ret
    }
}

pub (crate) fn fx_get_radius(fx : &(String, HashMap<String, Vec<crate::FxData>>)) -> f32
{
    match fx.0.as_str()
    {
        "stroke" => fx.1["size"][0].f() as f32 + 2.0,
        "colorfill" => 0.0,
        "gradfill" => 0.0,
        "dropshadow" => fx.1["distance"][0].f() as f32,
        _ => panic!()
    }
}

pub (crate) fn fx_get_mask_func(fx : &(String, HashMap<String, Vec<crate::FxData>>)) -> String
{
    match fx.0.as_str()
    {
        "stroke" => "Weld".to_string(),
        "colorfill" => "None".to_string(),
        "gradfill" => "None".to_string(),
        _ => "Weld".to_string()
    }
}

pub (crate) fn fx_opacity_is_erasure(fx : &(String, HashMap<String, Vec<crate::FxData>>)) -> bool
{
    match fx.0.as_str()
    {
        "stroke" => fx.1["style"][0].s() == "center" || fx.1["style"][0].s() == "inside",
        //("stroke", _) => true,
        _ => false,
    }
}
pub (crate) fn fx_get_early_blend_mode(fx : &(String, HashMap<String, Vec<crate::FxData>>)) -> String
{
    match fx.0.as_str()
    {
        "stroke" => fx.1["mode"][0].s(),
        "colorfill" => fx.1["mode"][0].s(),
        "gradfill" => fx.1["mode"][0].s(),
        "dropshadow" => fx.1["mode"][0].s(),
        _ => "Copy".to_string(),
    }
}
pub (crate) fn fx_is_fill(fx : &(String, HashMap<String, Vec<crate::FxData>>)) -> bool
{
    match fx.0.as_str()
    {
        "stroke" => false,
        "colorfill" => true,
        "gradfill" => true,
        "dropshadow" => false,
        _ => false
    }
}
pub (crate) fn fx_update_metadata(fx : &mut (String, HashMap<String, Vec<crate::FxData>>), layer : &crate::Layer, img : &Image<4>)
{
    match fx.0.as_str()
    {
        "gradfill" =>
        {
            fx.1.insert("_x0".to_string(), vec!((layer.offset[0] as f64).into()));
            fx.1.insert("_y0".to_string(), vec!((layer.offset[1] as f64).into()));
            fx.1.insert("_x1".to_string(), vec!((layer.offset[0] as f64 + img.width as f64).into()));
            fx.1.insert("_y1".to_string(), vec!((layer.offset[1] as f64 + img.height as f64).into()));
        }
        _ => {}
    }
}
pub (crate) fn fx_get_weld_func(fx : &(String, HashMap<String, Vec<crate::FxData>>)) -> String
{
    match fx.0.as_str()
    {
        "stroke" =>
        {
            if fx.1["size"][0].f() == 1.0 && fx.1["style"][0].s() == "center"
            {
                "Normal".to_string()
            }
            else if fx.1["style"][0].s() == "outside"
            {
                "Sum Weld".to_string()
            }
            else if fx.1["style"][0].s() == "inside"
            {
                "Clip Weld".to_string()
            }
            else
            {
                "Soft Weld".to_string()
            }
        }
        "colorfill" => "Copy".to_string(),
        "gradfill" => "Copy".to_string(),
        "dropshadow" => "Interpolate".to_string(),
        //"dropshadow" => fx.1["mode"][0].s(),
        _ => "Weld".to_string()
    }
}

impl Image<4>
{
    pub (crate) fn from_rgbaimage(input : &image::RgbaImage) -> Self
    {
        let (w, h) = input.dimensions();
        let data = ImageData::<4>::new_int(w as usize, h as usize);
        let mut ret = Self { width : w as usize, height : h as usize, data };
        for y in 0..ret.height
        {
            for x in 0..ret.width
            {
                let px = input.get_pixel(x as u32, y as u32).0;
                ret.set_pixel(x as isize, y as isize, px);
            }
        }
        ret
    }
    pub (crate) fn blank_white_transparent(w : usize, h : usize) -> Self
    {
        let mut data = ImageData::new_int(w, h);
        match &mut data
        {
            ImageData::Int(ref mut data) =>
            {
                for px in data.iter_mut()
                {
                    *px = [255, 255, 255, 0];
                }
            }
            ImageData::Float(ref mut data) =>
            {
                for px in data.iter_mut()
                {
                    *px = [1.0, 1.0, 1.0, 0.0];
                }
            }
        }
        Self { width : w, height : h, data }
    }
    pub (crate) fn apply_fx_dummy_outline(&mut self, rect : [[f32; 2]; 2], source : Option<&Self>, mask : Option<&Image<1>>, mask_info : Option<&MaskInfo>, top_opacity : f32, top_alpha_modifier : f32, top_funny_flag : bool, top_offset : [isize; 2], blend_mode : &str)
    {
        if blend_mode == "None" { return; }
        //println!("----evil");
        let adj = Box::new(move |_c : [f32; 4], x : usize, y : usize, img : Option<&Self>| -> [f32; 4]
        {
            let img = img.unwrap();
            let x = x as isize;
            let y = y as isize;
            let c = img.get_pixel(x, y);
            if c[3] < 128
            {
                let mut maxa = 0;
                for oy in -3..=3
                {
                    for ox in -3..=3
                    {
                        let ma = ((oy*oy + ox*ox) as f32).sqrt();
                        let ma = (4.0 - ma).clamp(0.0, 1.0);
                        maxa = maxa.max((img.get_pixel(x+ox, y+oy)[3] as f32 * ma) as u8);
                    }
                }
                if maxa >= 128
                {
                    return [0.4, 0.2, 0.0, (maxa - 128) as f32 / 127.0];
                }
                [0.0, 0.0, 0.0, 0.0]
            }
            else
            {
                let mut maxa = 0;
                for oy in -3..=3
                {
                    for ox in -3..=3
                    {
                        let ma = ((oy*oy + ox*ox) as f32).sqrt();
                        let ma = (4.0 - ma).clamp(0.0, 1.0);
                        maxa = maxa.max((img.get_pixel(x+ox, y+oy)[3] as f32 * ma) as u8);
                    }
                }
                if maxa < 128
                {
                    return [0.4, 0.2, 0.0, (maxa - 128) as f32 / 127.0];
                }
                [0.0, 0.0, 0.0, 0.0]
            }
        });
        self.apply_modifier(rect, adj, source, false, mask, mask_info, top_opacity, top_alpha_modifier, top_funny_flag, top_offset, blend_mode);
    }
    
    pub (crate) fn apply_fx(&mut self, rect : [[f32; 2]; 2], fx : &(String, HashMap<String, Vec<crate::FxData>>), source : Option<&Self>, mask : Option<&Image<1>>, mask_info : Option<&MaskInfo>, top_opacity : f32, top_alpha_modifier : f32, top_funny_flag : bool, top_offset : [isize; 2], blend_mode : &str)
    {
        if blend_mode == "None" { return; }
        let adj : Box<dyn Fn([f32; 4], usize, usize, Option<&Self>) -> [f32; 4] + Send + Sync> = match fx.0.as_str()
        {
            "colorfill" =>
            {
                let r = fx.1["color"][0].f() as f32;
                let g = fx.1["color"][1].f() as f32;
                let b = fx.1["color"][2].f() as f32;
                Box::new(move |_c : [f32; 4], x : usize, y : usize, _img : Option<&Self>| -> [f32; 4]
                {
                    //let img = img.unwrap();
                    let x = x as isize;
                    let y = y as isize;
                    //let c = img.get_pixel_float(x, y);
                    //[r, g, b, c[3]]
                    [r, g, b, 1.0]
                })
            }
            "dropshadow" =>
            {
                let _r = fx.1["color"][0].f() as f32;
                let _g = fx.1["color"][1].f() as f32;
                let _b = fx.1["color"][2].f() as f32;
                
                let angle = fx.1["angle"][0].f() * (std::f64::consts::PI / 180.0);
                let (mut b, mut a) = angle.sin_cos();
                a *= fx.1["distance"][0].f();
                let a = a.round() as isize;
                b *= -fx.1["distance"][0].f();
                let b = b.round() as isize;
                //a /= n;
                //b /= n;
                println!("---- {} {}", a, b);
                
                //let wdp = wd/(wd+hd);
                //let wdp = wd/wd.max(hd);
                let wdp = 1.0;
                
                Box::new(move |_c : [f32; 4], x : usize, y : usize, img : Option<&Self>| -> [f32; 4]
                {
                    let img = img.unwrap();
                    let x = x as isize + a;
                    let y = y as isize + b;
                    let c = img.get_pixel_float(x, y);
                    
                    [_r, _g, _b, c[3]]
                })
            }
            "gradfill" =>
            {
                // FIXME: not pixel perfect. doesn't handle transparency sizing properly. alpha cutoff is 50%. doesn't support "smooth" gradients (cubic hermite spline...?)
                
                //println!("{:?}", fx.1);
                let colors = fx.1["gradient"][0].vvf().iter().map(|x| x.clone()).collect::<Vec<_>>();
                let alphas = fx.1["gradient"][1].vvf().iter().map(|x| x.clone()).collect::<Vec<_>>();
                
                let read_gradient = |colors : &Vec<Vec<f64>>, alphas : &Vec<Vec<f64>>, t|
                {
                    let mut nc = 0;
                    while nc + 1 < colors.len() && (colors[nc+1][3] as f32) < t
                    {
                        nc += 1;
                    }
                    let nc2 = (nc+1).min(colors.len()-1);
                    let mut tc = unlerp(colors[nc][3] as f32, colors[nc2][3] as f32, t);
                    let biascraw = (colors[nc2][3+1] as f32).clamp(0.0001, 0.9999);
                    let biasc = biascraw * 2.0 - 1.0;
                    if tc > biascraw
                    {
                        tc = (tc - 1.0) / (1.0 - biasc) + 1.0;
                    }
                    else
                    {
                        tc /= 1.0 + biasc;
                    }
                    
                    let mut c = [colors[nc][0] as f32, colors[nc][1] as f32, colors[nc][2] as f32];
                    for i in 0..3
                    {
                        c[i] = lerp(c[i], colors[nc2][i] as f32, tc);
                    }
                    
                    let mut na = 0;
                    while na + 1 < alphas.len() && (alphas[na+1][1] as f32) < t
                    {
                        na += 1;
                    }
                    let na2 = (na+1).min(alphas.len()-1);
                    let mut ta = unlerp(alphas[na][1] as f32, alphas[na2][1] as f32, t);
                    let biasaraw = (alphas[na2][1+1] as f32).clamp(0.0001, 0.9999);
                    let biasa = biasaraw * 2.0 - 1.0;
                    if ta > biasaraw
                    {
                        ta = (ta - 1.0) / (1.0 - biasa) + 1.0;
                    }
                    else
                    {
                        ta /= 1.0 + biasa;
                    }
                    
                    let mut a = [alphas[na][0] as f32];
                    for i in 0..1
                    {
                        a[i] = lerp(a[i], alphas[na2][i] as f32, ta);
                    }
                    
                    [c[0], c[1], c[2], a[0]]
                };
                
                let x0 = fx.1["_x0"][0].f();
                let x1 = fx.1["_x1"][0].f();
                let y0 = fx.1["_y0"][0].f();
                let y1 = fx.1["_y1"][0].f();
                
                let angle = fx.1["angle"][0].f() * (std::f64::consts::PI / 180.0);
                let (a, b) = angle.sin_cos();
                //a /= n;
                //b /= n;
                println!("---- {} {}", a, b);
                
                let w = x1 - x0;
                let h = y1 - y0;
                let wr = 1.0 / w;
                let hr = 1.0 / h;
                let wh = w * 0.5;
                let hh = h * 0.5;
                
                let n = (a/h).abs().max((b/w).abs());
                let asdf = 1.0 / w.min(h);
                
                let wd = (w * a + h * b).abs();
                let hd = (h * a - h * b).abs();
                let asdf2 = 1.0 / wd.min(hd);
                
                let s = fx.1["scale"][0].f() / 100.0;
                
                let s = (n/b.abs()).min(n/a.abs())/s;
                
                //let wdp = wd/(wd+hd);
                //let wdp = wd/wd.max(hd);
                let wdp = 1.0;
                
                Box::new(move |_c : [f32; 4], x : usize, y : usize, _img : Option<&Self>| -> [f32; 4]
                {
                    //let img = img.unwrap();
                    let x = x as isize;
                    let y = y as isize;
                    //let c = img.get_pixel_float(x, y);
                    
                    let xd = x as f64 - wh;
                    let yd = y as f64 - hh;
                    
                    let mut xd2 = xd * b - yd * a;
                    //xd2 *= asdf;
                    xd2 *= s;
                    
                    let xd2 = xd2 as f32 + 0.5;
                    let xd2 = xd2.clamp(0.0, 1.0);
                    let c2 = read_gradient(&colors, &alphas, xd2);
                    //c2[3] *= c[3];
                    c2
                    
                    //[xd2 as f32 + 0.5, xd2 as f32 + 0.5, xd2 as f32 + 0.5, c[3]]
                })
            }
            "stroke" =>
            {
                let r = fx.1["color"][0].f() as f32;
                let g = fx.1["color"][1].f() as f32;
                let b = fx.1["color"][2].f() as f32;
                let osize = fx.1["size"][0].f() as f32;
                let osint = osize.ceil() as isize;
                let size = (osize * 0.5).max(1.0);
                let sint = size.ceil() as isize;
                
                match fx.1["style"][0].s().as_str()
                {
                    "center" =>
                    {
                        if osize != 1.0
                        {
                            Box::new(move |_c : [f32; 4], x : usize, y : usize, img : Option<&Self>| -> [f32; 4]
                            {
                                let img = img.unwrap();
                                let x = x as isize;
                                let y = y as isize;
                                let c = img.get_pixel(x, y);
                                let mut maxa = 0;
                                if osint != 1 &&  c[3] > 0 && c[3] < 255
                                {
                                    return [r, g, b, 1.0];
                                }
                                for oy in -sint-1..=sint+1
                                {
                                    for ox in -sint-1..=sint+1
                                    {
                                        let da = img.get_pixel(x+ox, y+oy)[3];
                                        let daf = img.get_pixel_float(x+ox, y+oy)[3];
                                        let ma = ((oy*oy + ox*ox) as f32).sqrt();
                                        let add1 = daf;
                                        let add2 = 1.0 - daf;
                                        let ma = (size - ma + if c[3] < 255 { add1 } else { add2 }).clamp(0.0, 1.0);
                                        if (c[3] < 255 && da > 0)
                                            || (c[3] >= 255 && da < 255)
                                        {
                                            maxa = maxa.max((255.0 * ma) as u8);
                                        }
                                    }
                                }
                                if maxa >= 1
                                {
                                    let a = (maxa) as f32 / 255.0;
                                    return [r, g, b, a];
                                }
                                [0.0, 0.0, 0.0, 0.0]
                            })
                        }
                        else
                        {
                            Box::new(move |_c : [f32; 4], x : usize, y : usize, img : Option<&Self>| -> [f32; 4]
                            {
                                let img = img.unwrap();
                                let x = x as isize;
                                let y = y as isize;
                                let c = img.get_pixel(x, y);
                                let mut maxa = 0;
                                if osint != 1 &&  c[3] > 0 && c[3] < 255
                                {
                                    return [r, g, b, 1.0];
                                }
                                for oy in -sint-1..=sint+1
                                {
                                    for ox in -sint-1..=sint+1
                                    {
                                        let da = img.get_pixel(x+ox, y+oy)[3];
                                        let daf = img.get_pixel_float(x+ox, y+oy)[3];
                                        let ma = ((oy*oy + ox*ox) as f32).sqrt();
                                        let add1 = (daf * 2.0 - 1.0).clamp(0.0, 1.0);
                                        let add2 = (1.0 - daf * 2.0).clamp(0.0, 1.0);
                                        let ma = ((size - ma) * 2.0 + if c[3] < 255 { add1 } else { add2 }).clamp(0.0, 1.0);
                                        if (c[3] < 255 && da > 0)
                                            || (c[3] >= 255 && da < 255)
                                        {
                                            maxa = maxa.max((255.0 * ma) as u8);
                                        }
                                    }
                                }
                                if maxa >= 1
                                {
                                    let mut a = (maxa) as f32 / 255.0;
                                    a *= 1.0 - ((0.5 - img.get_pixel_float(x, y)[3])).abs();
                                    return [r, g, b, a];
                                }
                                [0.0, 0.0, 0.0, 0.0]
                            })
                        }
                    }
                    "inside" => Box::new(move |_c : [f32; 4], x : usize, y : usize, img : Option<&Self>| -> [f32; 4]
                    {
                        let img = img.unwrap();
                        let x = x as isize;
                        let y = y as isize;
                        let c = img.get_pixel(x, y);
                        let mut maxa = 0;
                        if c[3] > 0
                        {
                            for oy in -osint-1..=osint+1
                            {
                                for ox in -osint-1..=osint+1
                                {
                                    let ma = ((oy*oy + ox*ox) as f32).sqrt();
                                    let ma = (osize - ma + 1.0 - img.get_pixel_float(x+ox, y+oy)[3]).clamp(0.0, 1.0);
                                    if img.get_pixel(x+ox, y+oy)[3] < 255
                                    {
                                        maxa = maxa.max((255.0 * ma) as u8);
                                    }
                                }
                            }
                            if maxa >= 1
                            {
                                let a = (maxa) as f32 / 255.0;
                                return [r, g, b, a];
                            }
                        }
                        [0.0, 0.0, 0.0, 0.0]
                    }),
                    // "outside", default
                    _ => Box::new(move |_c : [f32; 4], x : usize, y : usize, img : Option<&Self>| -> [f32; 4]
                    {
                        let img = img.unwrap();
                        let x = x as isize;
                        let y = y as isize;
                        let c = img.get_pixel(x, y);
                        let mut maxa = 0;
                        //if c[3] > 0 && c[3] < 255
                        //{
                        //    return [r, g, b, 1.0];
                        //}
                        if c[3] < 255
                        {
                            for oy in -osint-1..=osint+1
                            {
                                for ox in -osint-1..=osint+1
                                {
                                    let ma = ((oy*oy + ox*ox) as f32).sqrt();
                                    let ma = (osize - ma + img.get_pixel_float(x+ox, y+oy)[3]).clamp(0.0, 1.0);
                                    if img.get_pixel(x+ox, y+oy)[3] > 0
                                    {
                                        maxa = maxa.max((255.0 * ma) as u8);
                                    }
                                }
                            }
                            if maxa >= 1
                            {
                                let mut a = (maxa) as f32 / 255.0;
                                a *= 1.0 - to_float(c[3]);
                                return [r, g, b, a];
                            }
                        }
                        [0.0, 0.0, 0.0, 0.0]
                    }),
                }
            }
            _ => panic!()
        };
        //println!("{:?}", rect_translate(rect, vec_neg(&rect[0])));
        self.apply_modifier(rect, adj, source, false, mask, mask_info, top_opacity, top_alpha_modifier, top_funny_flag, top_offset, blend_mode);
    }
    pub (crate) fn apply_adjustment(&mut self, rect : [[f32; 2]; 2], adjustment : &Adjustment, mask : Option<&Image<1>>, mask_info : Option<&MaskInfo>, top_opacity : f32, top_alpha_modifier : f32, top_funny_flag : bool, top_offset : [isize; 2], blend_mode : &str)
    {
        if blend_mode == "None" { return; }
        let adj = Self::find_adjustment(adjustment);
        self.apply_modifier(rect, adj, None, true, mask, mask_info, top_opacity, top_alpha_modifier, top_funny_flag, top_offset, blend_mode);
    }
    #[inline(never)]
    pub (crate) fn apply_modifier(&mut self, rect : [[f32; 2]; 2], modifier : Box<dyn Fn([f32; 4], usize, usize, Option<&Self>) -> [f32; 4] + Send + Sync>,
        source : Option<&Self>, flush_opacity : bool,
        mask : Option<&Image<1>>, mask_info : Option<&MaskInfo>,
        top_opacity : f32, top_alpha_modifier : f32, top_funny_flag : bool, top_offset : [isize; 2], blend_mode : &str)
    {
        if blend_mode == "None" { return; }
        let min_x = 0.max(rect[0][0].floor() as isize) as usize;
        let max_x = (self.width  as isize).min(rect[1][0].ceil() as isize + 1).max(0) as usize;
        let min_y = 0.max(rect[0][1].floor() as isize) as usize;
        let max_y = (self.height as isize).min(rect[1][1].ceil() as isize + 1).max(0) as usize;
        
        //println!("{:?}, {}, {}", top_offset, self.height, max_y);
        
        let self_width = self.width;
        if self_width == 0
        {
            return;
        }
        let _info = mask_info.cloned();
        let info = mask_info.as_ref();
        let xoffs = info.map(|n| n.x).unwrap_or(0) as isize + top_offset[0];
        let yoffs = info.map(|n| n.y).unwrap_or(0) as isize + top_offset[1];
        let default = mask_info.map(|n| n.default_color as f32 / 255.0).unwrap_or(0.0);
        
        //println!("{:?}", mask);
        //println!("???????? {:?}", mask_info);
        let get_opacity : Box<dyn Fn(usize, usize) -> f32 + Send + Sync> = if let (Some(mask), false) = (mask, mask_info.map(|x| x.disabled).unwrap_or(false))
        //let get_opacity : Box<dyn Fn(usize, usize) -> f32 + Send + Sync> = if let Some(mask) = mask
        {
            Box::new(|x : usize, y : usize| mask.get_pixel_float_default(x as isize - xoffs, y as isize - yoffs, default)[0] * top_opacity)
        }
        else
        {
            Box::new(|_x : usize, _y : usize| top_opacity)
        };
        
        macro_rules! do_loop
        {
            ($bottom:expr, $find_blend_func:expr, $find_post_func:expr, $do_adjustment:expr, $maxval:expr) =>
            {
                {
                    let thread_count = get_thread_count();
                    let bottom = $bottom.get_mut(min_y*self_width..max_y*self_width);
                    if !bottom.is_some()
                    {
                        return;
                    }
                    let mut bottom = bottom.unwrap();
                    let infos =
                    {
                        let row_count = max_y - min_y + 1;
                        if row_count < thread_count { vec!((bottom, min_y, blend_mode.clone())) }
                        else
                        {
                            let chunk_size_rows = row_count/thread_count;
                            let chunk_size_pixels = chunk_size_rows*self_width;
                            let mut ret = Vec::new();
                            for i in 0..thread_count
                            {
                                if i+1 < thread_count
                                {
                                    let (split, remainder) = bottom.split_at_mut(chunk_size_pixels);
                                    bottom = remainder;
                                    ret.push((split, min_y + chunk_size_rows*i, blend_mode.clone()));
                                }
                            }
                            if bottom.len() > 0
                            {
                                ret.push((bottom, min_y + chunk_size_rows*(thread_count-1), blend_mode.clone()));
                            }
                            ret
                        }
                    };
                    
                    let modifier = &$do_adjustment;
                    
                    macro_rules! apply_info { ($info:expr, $get_opacity:expr) =>
                    {
                        let blend_mode = $info.2;
                        
                        let blend_f = $find_blend_func(blend_mode);
                        let post_f = $find_post_func(blend_mode);
                        
                        let bottom = $info.0;
                        let offset = $info.1;
                        let min_y = 0;
                        let max_y = bottom.len()/self_width;
                        
                        for y in min_y..max_y
                        {
                            let self_index_y_part = y*self_width;
                            
                            for x in min_x..max_x
                            {
                                let bottom_index = self_index_y_part + x;
                                
                                let mut bottom_pixel = bottom[bottom_index];
                                let a = bottom_pixel[3];
                                if flush_opacity
                                {
                                    bottom_pixel[3] = $maxval;
                                }
                                let opacity = $get_opacity(x, y + offset);
                                
                                let top_pixel = modifier(bottom_pixel, (x as isize - top_offset[0]) as usize, ((y + offset) as isize - top_offset[1]) as usize, source);
                                
                                let mut c = top_pixel;
                                if flush_opacity
                                {
                                    c = blend_f(top_pixel, bottom_pixel, opacity, top_alpha_modifier, top_funny_flag);
                                    c = post_f(c, top_pixel, bottom_pixel, opacity, top_alpha_modifier, top_funny_flag, [x, y + offset]);
                                    c[3] = a;
                                }
                                
                                bottom[bottom_index] = c;
                            }
                        }
                    } }
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // FEARLESS CONCURRENCY
                        get_pool().install(||
                        {
                            rayon::scope(|s|
                            {
                                for info in infos
                                {
                                    let get_opacity = &get_opacity;
                                    s.spawn(move |_|
                                    {
                                        apply_info!(info, get_opacity);
                                    })
                                }
                            })
                        })
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        for info in infos
                        {
                            apply_info!(info, get_opacity);
                        }
                    }
                }
            }
        }
        match &mut self.data
        {
            ImageData::<4>::Float(bottom) =>
                do_loop!(bottom, find_blend_func_float, find_post_func_float, modifier, 1.0),
            ImageData::<4>::Int(bottom) =>
                do_loop!(bottom, find_blend_func, find_post_func, |c, x, y, img| px_to_int(modifier(px_to_float(c), x, y, img)), 255),
        }
        
    }
    
    fn find_adjustment(adjustment : &Adjustment) -> Box<dyn Fn([f32; 4], usize, usize, Option<&Self>) -> [f32; 4] + Send + Sync>
    {
        let adjustment = adjustment.clone();
        match adjustment
        {
            Adjustment::Invert => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                for i in 0..3
                {
                    c[i] = 1.0 - c[i];
                }
                c
            }),
            Adjustment::Posterize(n) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                for i in 0..3
                {
                    c[i] = (c[i] * n * 0.99999).floor() / (n-1.0);
                    //c[i] = (c[i] * (n)).round() / (n);
                    //c[i] = (c[i] * (n-1.0)).round() / (n-1.0);
                }
                c
            }),
            Adjustment::Threshold(n) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                let mut n = n;
                n = n * (1.0/255.0);
                let v = calc_y([c[0], c[1], c[2]]);
                let v = if v >= n { 1.0 } else { 0.0 };
                c[0] = v;
                c[1] = v;
                c[2] = v;
                c
            }),
            Adjustment::BrightContrast(n) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                //println!("-------{:?}", n);
                let b = n[0] / 100.0 * 0.4*0.99;
                let mut cx = n[1] / 100.0 + 1.0;
                let m = n[2] / 255.0;
                let _is_legacy = n[3] == 1.0;
                if cx > 1.0
                {
                    cx = 1.0/(1.0 - (cx-1.0)*0.99);
                }
                for i in 0..3
                {
                    c[i] += b;
                    c[i] = (c[i] - m) * cx + m;// + b;
                }
                c
            }),
            Adjustment::HueSatLum(n) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                let l = n[2] / 100.0;
                for i in 0..3
                {
                    c[i] = c[i] * (1.0 - l.abs()) + if l > 0.0 { l } else { 0.0 };
                    c[i] = c[i].clamp(0.0, 1.0);
                }
                
                let mut hsl = rgb_to_hsl(c);
                let h = n[0];
                hsl[0] = ((hsl[0] + h) % 360.0 + 360.0) % 360.0;
                let s = n[1] / 100.0;
                if s <= 0.0
                {
                    hsl[1] *= s + 1.0;
                }
                else
                {
                    hsl[1] /= 1.0 - s*0.99;
                }
                hsl[1] = hsl[1].clamp(0.0, 1.0);
                c = hsl_to_rgb(hsl);
                c
            }),
            Adjustment::Curves(v) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                let points = &v[0];
                let tans = compute_spline_tangents(points);
                //println!("----- {:?}", points);
                for i in 0..3
                {
                    let n = binary_search_last_lt(points, c[i]);
                    c[i] = interpolate_spline(c[i], points, &tans, n);
                }
                c
            }),
            Adjustment::Levels(v) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                let apply_levels = |c : f32, data : &[f32; 5]|
                {
                    let mut c = c;
                    c -= data[0];
                    c /= data[1] - data[0];
                    c = c.clamp(0.0, 1.0);
                    c = c.powf(1.0/data[4]);
                    c *= data[3] - data[2];
                    c += data[2];
                    c
                };
                c[0] = apply_levels(c[0], &v[0]);
                c[1] = apply_levels(c[1], &v[0]);
                c[2] = apply_levels(c[2], &v[0]);
                for i in 0..3
                {
                    c[i] = apply_levels(c[i], &v[i+1]);
                }
                c
            }),
            Adjustment::BlackWhite((v, _colorized, _color)) => Box::new(move |mut c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4]
            {
                fn rgb_to_rygcbml(rgb: &[f32]) -> [f32; 7]
                {
                    let [r, g, b] = [rgb[0], rgb[1], rgb[2]];

                    let l = r.min(g).min(b);

                    let r = r - l;
                    let g = g - l;
                    let b = b - l;

                    let y = r.min(g);
                    let c = g.min(b);
                    let m = r.min(b);
                    let r2 = r - y - m;
                    let g2 = g - y - c;
                    let b2 = b - c - m;

                    [r2, y, g2, c, b2, m, l]
                }
                
                let sept = rgb_to_rygcbml(&c);
                let mut l = sept[6];
                for i in 0..6
                {
                    let a = sept[i] * (v[i] * 0.01);
                    l += a;
                }
                
                c[0] = l;
                c[1] = l;
                c[2] = l;
                c
            }),
            _ => Box::new(|c : [f32; 4], _x : usize, _y : usize, _img : Option<&Self>| -> [f32; 4] { c }),
        }
    }
    #[inline(never)]
    pub (crate) fn blend_rect_from(&mut self, rect : [[f32; 2]; 2], top : &Image<4>, mask : Option<&Image<1>>, mask_info : Option<&MaskInfo>, top_opacity : f32, top_alpha_modifier : f32, top_funny_flag : bool, top_offset : [isize; 2], blend_mode : &str)
    {
        if blend_mode == "None" { return; }
        //rect[0][0] += top_offset[0] as f32;
        //rect[1][0] += top_offset[0] as f32;
        //rect[0][1] += top_offset[1] as f32;
        //rect[1][1] += top_offset[1] as f32;
        
        let min_x = 0.max(rect[0][0].floor() as isize).max(top_offset[0]) as usize;
        let max_x = ((self.width  as isize).min(top.width  as isize + top_offset[0])).min(rect[1][0].ceil() as isize + 1).max(0) as usize;
        let min_y = 0.max(rect[0][1].floor() as isize).max(top_offset[1]) as usize;
        let max_y = ((self.height as isize).min(top.height as isize + top_offset[1])).min(rect[1][1].ceil() as isize + 1).max(0) as usize;
        
        //println!("{:?}, {}, {}", top_offset, self.height, max_y);
        
        let self_width = self.width;
        if self_width == 0
        {
            return;
        }
        let top_width = top.width;
        
        let _info = mask_info.cloned();
        let info = mask_info.as_ref();
        let xoffs = info.map(|n| n.x).unwrap_or(0) as isize + top_offset[0];
        let yoffs = info.map(|n| n.y).unwrap_or(0) as isize + top_offset[1];
        let default = mask_info.map(|n| n.default_color as f32 / 255.0).unwrap_or(0.0);
        
        //println!("{:?}", mask);
        //println!("???????? {:?}", mask_info);
        let get_opacity : Box<dyn Fn(usize, usize) -> f32 + Send + Sync> = if let (Some(mask), false) = (mask, mask_info.map(|x| x.disabled).unwrap_or(false))
        //let get_opacity : Box<dyn Fn(usize, usize) -> f32 + Send + Sync> = if let Some(mask) = mask
        {
            Box::new(|x : usize, y : usize| mask.get_pixel_float_default(x as isize - xoffs, y as isize - yoffs, default)[0] * top_opacity)
        }
        else
        {
            Box::new(|_x : usize, _y : usize| top_opacity)
        };
        
        // separate from loop_rect_threaded because this is used by layer stack flattening, and needs to be as fast as possible
        // so we do everything purely with a macro to ensure that as much inlining can be done as the compiler is capable of
        
        macro_rules! do_loop
        {
            ($bottom:expr, $top:expr, $bottom_read_f:expr, $top_read_f:expr, $bottom_write_f:expr, $find_blend_func:expr, $find_post_func:expr) =>
            {
                {
                    let thread_count = get_thread_count();
                    //println!("threadcount {}", thread_count);
                    let bottom = $bottom.get_mut(min_y*self_width..max_y*self_width);
                    if !bottom.is_some()
                    {
                        return;
                    }
                    let mut bottom = bottom.unwrap();
                    let infos =
                    {
                        let row_count = max_y - min_y + 1;
                        if row_count < thread_count { vec!((bottom, min_y, blend_mode.clone())) }
                        else
                        {
                            let chunk_size_rows = row_count/thread_count;
                            let chunk_size_pixels = chunk_size_rows*self_width;
                            let mut ret = Vec::new();
                            for i in 0..thread_count
                            {
                                if i+1 < thread_count
                                {
                                    let (split, remainder) = bottom.split_at_mut(chunk_size_pixels);
                                    bottom = remainder;
                                    ret.push((split, min_y + chunk_size_rows*i, blend_mode.clone()));
                                }
                            }
                            if bottom.len() > 0
                            {
                                ret.push((bottom, min_y + chunk_size_rows*(thread_count-1), blend_mode.clone()));
                            }
                            ret
                        }
                    };
                    
                    macro_rules! apply_info { ($info:expr, $get_opacity:expr) =>
                    {
                        let blend_mode = $info.2;
                        
                        let blend_f = $find_blend_func(blend_mode);
                        let post_f = $find_post_func(blend_mode);
                        
                        let bottom = $info.0;
                        let offset = $info.1;
                        let min_y = 0;
                        let max_y = bottom.len()/self_width;
                        
                        for y in min_y..max_y
                        {
                            let self_index_y_part = y*self_width;
                            let top_index_y_part = (y as isize + offset as isize - top_offset[1]) as usize * top_width;
                            
                            for x in min_x..max_x
                            {
                                let bottom_index = self_index_y_part + x;
                                let top_index = (top_index_y_part as isize + x as isize - top_offset[0]) as usize;
                                
                                let bottom_pixel = $bottom_read_f(bottom[bottom_index]);
                                let top_pixel = $top_read_f($top[top_index]);
                                let opacity = $get_opacity(x, y + offset);
                                
                                let c = blend_f(top_pixel, bottom_pixel, opacity, top_alpha_modifier, top_funny_flag);
                                let c = post_f(c, top_pixel, bottom_pixel, opacity, top_alpha_modifier, top_funny_flag, [x, y + offset]);
                                
                                bottom[bottom_index] = $bottom_write_f(c);
                            }
                        }
                    } }
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // FEARLESS CONCURRENCY
                        get_pool().install(||
                        {
                            rayon::scope(|s|
                            {
                                for info in infos
                                {
                                    let get_opacity = &get_opacity;
                                    s.spawn(move |_|
                                    {
                                        apply_info!(info, get_opacity);
                                    });
                                }
                            });
                        });
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        for info in infos
                        {
                            apply_info!(info, get_opacity);
                        }
                    }
                }
            }
        }
        
        //use std::time::Instant;
        //let start = Instant::now();

        match (&mut self.data, &top.data)
        {
            (ImageData::<4>::Float(bottom), ImageData::<4>::Float(top)) =>
                do_loop!(bottom, top,         nop,         nop,       nop, find_blend_func_float, find_post_func_float),
            (ImageData::<4>::Float(bottom), ImageData::<4>::Int(top)) =>
                do_loop!(bottom, top,         nop, px_to_float,       nop, find_blend_func_float, find_post_func_float),
            (ImageData::<4>::Int(bottom), ImageData::<4>::Float(top)) =>
                do_loop!(bottom, top, px_to_float,         nop, px_to_int, find_blend_func_float, find_post_func_float),
            (ImageData::<4>::Int(bottom), ImageData::<4>::Int(top)) =>
                do_loop!(bottom, top, nop, nop, nop, find_blend_func, find_post_func),
        }
        
        //let elapsed = start.elapsed().as_secs_f32();
        //println!("Blended in {:.6} seconds", elapsed);
    }
    pub (crate) fn blend_from(&mut self, top : &Image<4>, mask : Option<&Image<1>>, mask_info : Option<&MaskInfo>, top_opacity : f32, top_offset : [isize; 2], blend_mode : &str)
    {
        if blend_mode == "None" { return; }
        self.blend_rect_from([[0.0, 0.0], [self.width as f32, self.height as f32]], top, mask, mask_info, top_opacity, 1.0, false, top_offset, blend_mode)
    }
    
    pub (crate) fn analyze_edit(old_data : &Image<4>, new_data : &Image<4>, uuid : u128, rect : Option<[[f32; 2]; 2]>) -> UndoEvent
    {
        let mut min_x = 0;
        let mut max_x = new_data.width;
        let mut min_y = 0;
        let mut max_y = new_data.height;
        if let Some(rect) = rect
        {
            min_x = min_x.max(rect[0][0].floor() as usize);
            min_y = min_y.max(rect[0][1].floor() as usize);
            max_x = max_x.min(rect[1][0].ceil() as usize);
            max_y = max_y.min(rect[1][1].ceil() as usize);
        }
        //macro_rules! do_loop { ($y_outer:expr, $outer_range:expr, $inner_range:expr, $target:expr, $f:expr) =>
        //{
        //    for outer in $outer_range
        //    {
        //        for inner in $inner_range
        //        {
        //            let first = if $y_outer { inner } else { outer } as isize;
        //            let second = if $y_outer { outer } else { inner } as isize;
        //            let old_c = old_data.get_pixel_float_wrapped(first, second);
        //            let new_c = new_data.get_pixel_float_wrapped(first, second);
        //            if !vec_eq(&old_c, &new_c)
        //            {
        //                *$target = $f(*$target, outer);
        //            }
        //        }
        //    }
        //} }
        //do_loop!(true , 0..new_data.height            , 0..new_data.width, &mut min_y, usize::min);
        //do_loop!(true , (min_y..new_data.height).rev(), 0..new_data.width, &mut max_y, usize::max);
        //do_loop!(false, 0..new_data.width             , min_y..=max_y    , &mut min_x, usize::min);
        //do_loop!(false, (min_x..new_data.width).rev() , min_y..=max_y    , &mut max_x, usize::max);
        
        //println!("{} {} {} {} {:?}", min_x, max_x, min_y, max_y, rect);
        
        if max_y >= min_y && max_x >= min_x
        {
            let w = max_x - min_x + 1;
            let h = max_y - min_y + 1;
            
            let mut old_copy = if old_data.is_int() { Image::<4>::blank(w, h) } else { Image::<4>::blank_float(w, h) };
            let mut new_copy = if old_data.is_int() { Image::<4>::blank(w, h) } else { Image::<4>::blank_float(w, h) };
            let mut mask = vec!(false; w*h);
            
            if old_data.is_int()
            {
                for y in min_y..=max_y
                {
                    for x in min_x..=max_x
                    {
                        let old_c = old_data.get_pixel(x as isize, y as isize);
                        let new_c = new_data.get_pixel(x as isize, y as isize);
                        if !vec_eq_u8(&old_c, &new_c)
                        {
                            let x2 = x - min_x;
                            let y2 = y - min_y;
                            old_copy.set_pixel(x2 as isize, y2 as isize, old_c);
                            new_copy.set_pixel(x2 as isize, y2 as isize, new_c);
                            mask[y2 * w + x2] = true;
                        }
                    }
                }
            }
            else
            {
                for y in min_y..=max_y
                {
                    for x in min_x..=max_x
                    {
                        let old_c = old_data.get_pixel_float_wrapped(x as isize, y as isize);
                        let new_c = new_data.get_pixel_float_wrapped(x as isize, y as isize);
                        if !vec_eq(&old_c, &new_c)
                        {
                            let x2 = x - min_x;
                            let y2 = y - min_y;
                            old_copy.set_pixel_float_wrapped(x2 as isize, y2 as isize, old_c);
                            new_copy.set_pixel_float_wrapped(x2 as isize, y2 as isize, new_c);
                            mask[y2 * w + x2] = true;
                        }
                    }
                }
            }
            
            return UndoEvent::LayerPaint(LayerPaint {
                uuid,
                rect : [[min_x, min_y], [max_x+1, max_y+1]], 
                old : old_copy,
                new : new_copy,
                mask
            });
        }
        UndoEvent::Null
    }
    pub (crate) fn apply_edit(&mut self, event : &LayerPaint, is_undo : bool)
    {
        let rect = event.rect;
        let w = rect[1][0] - rect[0][0];
        
        let source = if is_undo { &event.old } else { &event.new };
        
        for y in rect[0][1]..rect[1][1]
        {
            for x in rect[0][0]..rect[1][0]
            {
                let x2 = x - rect[0][0];
                let y2 = y - rect[0][1];
                if event.mask[y2 * w + x2]
                {
                    let c = source.get_pixel_float_wrapped(x2 as isize, y2 as isize);
                    self.set_pixel_float_wrapped(x as isize, y as isize, c);
                }
            }
        }
    }
    pub (crate) fn undo_edit(&mut self, event : &LayerPaint)
    {
        self.apply_edit(event, true)
    }
    pub (crate) fn redo_edit(&mut self, event : &LayerPaint)
    {
        self.apply_edit(event, false)
    }
}

impl<const N : usize> Image<N>
{
    pub (crate) fn blank(w : usize, h : usize) -> Self
    {
        let data = ImageData::new_int(w, h);
        Self { width : w, height : h, data }
    }
    pub (crate) fn blank_float(w : usize, h : usize) -> Self
    {
        let data = ImageData::new_float(w, h);
        Self { width : w, height : h, data }
    }
    pub (crate) fn blank_with_same_size(&self) -> Self
    {
        Self::blank(self.width, self.height)
    }
    // for icons etc. too slow to use for anything else.
    pub (crate) fn to_egui(&self) -> egui::ColorImage
    {
        egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &self.data.clone().to_int())
    }
    pub (crate) fn to_imagebuffer(&self) -> image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>
    {
        match &self.data
        {
            ImageData::Int(data) =>
            {
                type T = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>;
                T::from_vec(self.width as u32, self.height as u32, flatten(data)).unwrap()
            }
            ImageData::Float(data) =>
            {
                type T = image::ImageBuffer::<image::Rgba<f32>, Vec<f32>>;
                let img = T::from_vec(self.width as u32, self.height as u32, flatten(data)).unwrap();
                image::DynamicImage::from(img).to_rgba8()
            }
        }
    }
    
    pub (crate) fn resized(&mut self, new_w : usize, new_h : usize) -> Image<N>
    {
        let mut ret = Self::blank(new_w, new_h);
        
        for y in 0..new_h as isize
        {
            for x in 0..new_w as isize
            {
                let s_x = (x as f32*self.width as f32/new_w as f32) as isize;
                let s_y = (y as f32*self.height as f32/new_h as f32) as isize;
                let c = self.get_pixel_float_wrapped(s_x, s_y);
                ret.set_pixel_float_wrapped(x, y, c);
            }
        }
        ret
    }
    
    pub (crate) fn loop_rect_threaded(&mut self, rect : [[f32; 2]; 2], func : &(dyn Fn(usize, usize, [f32; N]) -> [f32; N] + Sync + Send))
    {
        let min_x = 0.max(rect[0][0].floor() as isize) as usize;
        let max_x = (self.width as isize).min(rect[1][0].ceil() as isize + 1).max(0) as usize;
        let min_y = 0.max(rect[0][1].floor() as isize) as usize;
        let max_y = (self.height as isize).min(rect[1][1].ceil() as isize + 1).max(0) as usize;
        
        let self_width = self.width;
        
        macro_rules! do_loop
        {
            ($bottom:expr, $bottom_read_f:expr, $bottom_write_f:expr) =>
            {
                {
                    let mut thread_count = 16;
                    if let Some(count) = std::thread::available_parallelism().ok()
                    {
                        thread_count = count.get();
                    }
                    let bottom = $bottom.get_mut(min_y*self_width..max_y*self_width);
                    if !bottom.is_some()
                    {
                        return;
                    }
                    let mut bottom = bottom.unwrap();
                    let infos =
                    {
                        let row_count = max_y - min_y + 1;
                        if row_count < thread_count { vec!((bottom, min_y)) }
                        else
                        {
                            let chunk_size_rows = row_count/thread_count;
                            let chunk_size_pixels = chunk_size_rows*self_width;
                            let mut ret = Vec::new();
                            for i in 0..thread_count
                            {
                                if i+1 < thread_count
                                {
                                    let (split, remainder) = bottom.split_at_mut(chunk_size_pixels);
                                    bottom = remainder;
                                    ret.push((split, min_y + chunk_size_rows*i));
                                }
                            }
                            if bottom.len() > 0
                            {
                                ret.push((bottom, min_y + chunk_size_rows*(thread_count-1)));
                            }
                            ret
                        }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        // FEARLESS CONCURRENCY
                        std::thread::scope(|s|
                        {
                            for info in infos
                            {
                                let func = &func;
                                s.spawn(move ||
                                {
                                    let bottom = info.0;
                                    let offset = info.1;
                                    let min_y = 0;
                                    let max_y = bottom.len()/self_width;
                                    for y in min_y..max_y
                                    {
                                        let self_index_y_part = y*self_width;
                                        for x in min_x..max_x
                                        {
                                            let bottom_index = self_index_y_part + x;
                                            let mut bottom_pixel = $bottom_read_f(bottom[bottom_index]);
                                            bottom_pixel = func(x, y + offset, bottom_pixel);
                                            bottom[bottom_index] = $bottom_write_f(bottom_pixel);
                                        }
                                    }
                                });
                            }
                        });
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        for info in infos
                        {
                            let bottom = info.0;
                            let offset = info.1;
                            let min_y = 0;
                            let max_y = bottom.len()/self_width;
                            for y in min_y..max_y
                            {
                                let self_index_y_part = y*self_width;
                                for x in min_x..max_x
                                {
                                    let bottom_index = self_index_y_part + x;
                                    let mut bottom_pixel = $bottom_read_f(bottom[bottom_index]);
                                    bottom_pixel = func(x, y + offset, bottom_pixel);
                                    bottom[bottom_index] = $bottom_write_f(bottom_pixel);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        match &mut self.data
        {
            ImageData::<N>::Float(bottom) => do_loop!(bottom, nop, nop),
            ImageData::<N>::Int(bottom)   => do_loop!(bottom, px_to_float, px_to_int),
        }
    }
    
    pub (crate) fn clear_rect_with_color_float(&mut self, rect : [[f32; 2]; 2], color : [f32; N])
    {
        self.loop_rect_threaded(rect,
            &|_x, _y, _color : [f32; N]|
            {
                color
            }
        );
    }
    pub (crate) fn blend_rect_alpha(&mut self, rect : [[f32; 2]; 2], a : f32)
    {
        self.loop_rect_threaded(rect,
            &|_x, _y, mut color : [f32; N]|
            {
                color[3] *= a;
                color
            }
        );
    }
    pub (crate) fn clear_rect_alpha_float(&mut self, rect : [[f32; 2]; 2], alpha : f32)
    {
        for y in rect[0][1].floor().max(0.0) as isize..=(rect[1][1].ceil() as isize).min(self.height as isize - 1)
        {
            for x in rect[0][0].floor().max(0.0) as isize..=(rect[1][0].ceil() as isize).min(self.width as isize - 1)
            {
                let mut color = self.get_pixel_float_wrapped(x, y);
                color[3] = alpha;
                self.set_pixel_float_wrapped(x, y, color);
            }
        }
    }
    pub (crate) fn clear_outside_with_color_float(&mut self, mut rect : [[f32; 2]; 2], color : [f32; N])
    {
        let w = self.width as f32;
        let h = self.width as f32;
        rect[0][0] = rect[0][0].max(0.0);
        rect[0][1] = rect[0][1].max(0.0);
        rect[1][0] = rect[1][0].min(w);
        rect[1][1] = rect[1][1].min(h);
        
        self.clear_rect_with_color_float([[0.0, 0.0], rect[0]], color); // top left
        self.clear_rect_with_color_float([rect[1], [w, h]], color); // bottom right
        
        self.clear_rect_with_color_float([[0.0, rect[1][1]], [rect[0][0], h]], color); // bottom left
        self.clear_rect_with_color_float([[rect[1][0], 0.0], [w, rect[0][1]]], color); // top right
        
        self.clear_rect_with_color_float([[rect[0][0], 0.0], [rect[1][0], rect[0][1]]], color); // top
        self.clear_rect_with_color_float([[rect[0][0], rect[1][1]], [rect[1][0], h]], color); // bottom
        
        self.clear_rect_with_color_float([[0.0, rect[0][1]], [rect[0][0], rect[1][1]]], color); // left
        self.clear_rect_with_color_float([[rect[1][0], rect[0][1]], [w, rect[1][1]]], color); // right
    }
    pub (crate) fn alpha_rect_copy_from_mask(&mut self, rect : [[f32; 2]; 2], mask : &Image<1>)
    {
        for y in rect[0][1].floor().max(0.0) as isize..=(rect[1][1].ceil() as isize).min(self.height as isize - 1)
        {
            for x in rect[0][0].floor().max(0.0) as isize..=(rect[1][0].ceil() as isize).min(self.width as isize - 1)
            {
                let mut color = self.get_pixel_float_wrapped(x, y);
                let a = mask.get_pixel_float_wrapped(x, y)[0];
                color[3] = a;
                self.set_pixel_float_wrapped(x, y, color);
            }
        }
    }
    pub (crate) fn clear_with_color_float(&mut self, color : [f32; N])
    {
        for y in 0..self.height as isize
        {
            for x in 0..self.width as isize
            {
                self.set_pixel_float_wrapped(x, y, color);
            }
        }
    }
    pub (crate) fn clear_with_color(&mut self, color : [u8; N])
    {
        for y in 0..self.height as isize
        {
            for x in 0..self.width as isize
            {
                self.set_pixel_wrapped(x, y, color);
            }
        }
    }
    pub (crate) fn clear(&mut self)
    {
        self.clear_with_color_float([0.0; N]);
    }
    
    pub (crate) fn analyze_outline(&self) -> Vec<Vec<[f32; 2]>>
    {
        // find bounds of opaque section
        
        let mut min_x = self.width;
        let mut max_x = 0;
        let mut min_y = self.height;
        let mut max_y = 0;
        macro_rules! do_loop { ($y_outer:expr, $outer_range:expr, $inner_range:expr, $target:expr, $f:expr) =>
        {
            for outer in $outer_range
            {
                for inner in $inner_range
                {
                    let first = if $y_outer { inner } else { outer } as isize;
                    let second = if $y_outer { outer } else { inner } as isize;
                    let c = self.get_pixel_float_wrapped(first, second);
                    //println!("testing... {:?}", c);
                    if c[3] > 0.0
                    {
                        //println!("updating...");
                        *$target = $f(*$target, outer);
                    }
                }
            }
        } }
        do_loop!(true , 0..self.height            , 0..self.width, &mut min_y, usize::min);
        do_loop!(true , (min_y..self.height).rev(), 0..self.width, &mut max_y, usize::max);
        do_loop!(false, 0..self.width             , min_y..=max_y, &mut min_x, usize::min);
        do_loop!(false, (min_x..self.width).rev() , min_y..=max_y, &mut max_x, usize::max);
        
        max_x += 1;
        max_y += 1;
        
        let w = max_x - min_x;
        let h = max_y - min_y;
        
        let mut islands = Vec::new();
        
        // pixels that have already been added to an island
        let mut mask = vec!(false; w*h);
        
        //println!("running island analysis... {} {} {} {}", min_x, max_x, min_y, max_y);
        for y in min_y..max_y
        {
            for x in min_x..max_x
            {
                let not_clear = self.get_pixel_float(x as isize, y as isize)[3] > 0.0;
                let not_visited = !mask[(y-min_y)*w + x];
                // if already added to an island, skip
                if !not_clear || !not_visited
                {
                    //println!("continuing, because... {}, {}", not_clear, not_visited);
                    continue;
                }
                
                // we know this coord is part of an island now, identify the island by it
                islands.push([x as isize, y as isize]);
                
                // depth-first island traversal
                let mut frontier = Vec::new();
                let mut process_coord = |coord : [usize; 2], frontier : &mut Vec<_>|
                {
                    let x = coord[0];
                    let y = coord[1];
                    mask[(y-min_y)*w + x] = true;
                    //for add in [[0, -1], [0, 1], [1, 0], [-1, 0]]
                    for add in [[1, 0], [0, 1], [-1, 0], [0, -1]]
                    {
                        let coord = vec_add(&[x as isize, y as isize], &add);
                        if coord[0] < min_x as isize || coord[0] >= max_x as isize
                        || coord[1] < min_y as isize || coord[1] >= max_y as isize
                        {
                            continue;
                        }
                        
                        let x = coord[0] as usize;
                        let y = coord[1] as usize;
                        
                        let not_clear = self.get_pixel_float(x as isize, y as isize)[3] > 0.0;
                        let not_visited = !mask[(y-min_y)*w + x];
                        
                        if not_clear && not_visited
                        {
                            frontier.push([x, y]);
                        }
                    }
                };
                
                process_coord([x, y], &mut frontier);
                while let Some(coord) = frontier.pop()
                {
                    process_coord(coord, &mut frontier);
                }
            }
        }
        
        let mut loops = Vec::new();
        
        // the point that identifies an island is always open to both the top and left
        for coord in &islands
        {
            let mut coord = *coord;
            let occupied = |coord : [isize; 2]| -> bool
            {
                let x = coord[0];
                let y = coord[1];
                x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && mask[(y as usize-min_y)*w + x as usize]
            };
            let start = coord;
            
            let mut points = vec!(coord);
            
            // walk around the perimeter of the island
            // first dir in list is next dir, second is turning right, last is turning left
            // we navigate by rotating the dirs vector by moving its back to its front or vice versa
            let mut dirs = vec!([1, 0], [0, 1], [-1, 0], [0, -1]);
            // used to properly offset the coords in the loops vector
            let mut offset = vec!([0, 0], [1, 0], [1, 1], [0, 1]);
            let mut first = true;
            while first || coord != start || offset[0] != [0, 0]
            {
                //println!("at {:?} going in direction {:?}", coord, dirs[0]);
                first = false;
                // see if we can turn left; if we can, do so
                let left = vec_add(&coord, &dirs[3]);
                if occupied(left)
                {
                    //println!("going left");
                    coord = left;
                    dirs.rotate_right(1);
                    offset.rotate_right(1);
                    points.push(vec_add(&coord, &offset[0]));
                    continue;
                }
                // see if we can move straight; if not, do a right turn
                let straight = vec_add(&coord, &dirs[0]);
                if !occupied(straight)
                {
                    //println!("going right");
                    dirs.rotate_left(1);
                    offset.rotate_left(1);
                    points.push(vec_add(&coord, &offset[0]));
                    continue;
                }
                // otherwise go straight
                //println!("going straight");
                coord = straight;
            }
            loops.push(points);
        }
        
        let mut loops : Vec<Vec<_>> = loops.into_iter().map(|points| points.into_iter().map(|coord| [coord[0] as f32, coord[1] as f32]).collect::<_>()).collect::<_>();
        for points in loops.iter_mut()
        {
            points.push(points[0]);
        }
        
        //println!("{:?}", mask);
        //println!("{:?}", islands);
        //println!("{:?}", loops);
        
        loops
    }
    
}

