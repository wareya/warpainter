
use eframe::egui;


#[derive(Debug, Clone)]
pub (crate) enum ImageData
{
    Float(Vec<f32>),
    Int(Vec<u8>),
}

pub (crate) fn to_float(x : u8) -> f32
{
    (x as f32)*255.0
}
pub (crate) fn to_int(x : f32) -> u8
{
    (x*255.0).round().clamp(0.0, 255.0) as u8
}

pub (crate) fn px_to_float(x : [u8; 4]) -> [f32; 4]
{
    let mut r = [0.0; 4];
    for i in 0..4
    {
        r[i] = to_float(x[i]);
    }
    r
}
pub (crate) fn px_to_int(x : [f32; 4]) -> [u8; 4]
{
    let mut r = [0; 4];
    for i in 0..4
    {
        r[i] = to_int(x[i]);
    }
    r
}

pub (crate) fn rgb_to_hsv(rgba : [f32; 4]) -> [f32; 4]
{
    let v = rgba[0].max(rgba[1]).max(rgba[2]);
    let c = v - rgba[0].min(rgba[1]).min(rgba[2]);
    let s = if v > 0.0 { c / v } else { 0.0 };
    let h;
    if c == 0.0
    {
        h = 0.0;
    }
    else if v == rgba[0]
    {
        h = 60.0 * (rgba[1] - rgba[2])/c;
    }
    else if v == rgba[1]
    {
        h = 60.0 * (rgba[2] - rgba[0])/c + 120.0;
    }
    else
    {
        h = 60.0 * (rgba[0] - rgba[1])/c + 240.0;
    }
    [h, s, v, rgba[3]]
}
pub (crate) fn hsv_to_rgb(hsva : [f32; 4]) -> [f32; 4]
{
    let c = hsva[2] * hsva[1];
    let h2 = hsva[0] / 60.0;
    let x = c * (1.0 - ((h2%2.0) - 1.0).abs());
    
    let triad = [
        [c, x, 0.0],
        [x, c, 0.0],
        [0.0, c, x],
        [0.0, x, c],
        [x, 0.0, c],
        [c, 0.0, x],
    ][h2.floor() as usize % 6];
    
    let m = hsva[2] - c;
    [triad[0] + m, triad[1] + m, triad[2] + m, hsva[3]]
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
#[derive(Debug, Clone)]
pub (crate) struct Image
{
    pub (crate) width : usize,
    pub (crate) height : usize,
    data : ImageData,
}

impl Image
{
    pub (crate) fn blank(w : usize, h : usize) -> Self
    {
        let data = ImageData::new_int(w as usize, h as usize);
        let mut ret = Self { width : w as usize, height : h as usize, data };
        ret
    }
    pub (crate) fn from_rgbaimage(input : &image::RgbaImage) -> Self
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
                ret.set_pixel(x as isize, y as isize, px);
            }
        }
        ret
    }
    pub (crate) fn to_egui(&self) -> egui::ColorImage
    {
        match &self.data
        {
            ImageData::Int(data) =>
                egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &data),
            _ =>
                egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &self.data.to_int()),
        }
    }
    pub (crate) fn set_pixel(&mut self, x : isize, y : isize, px : [u8; 4])
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return;
        }
        let x = x as usize;
        let y = y as usize;
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
    pub (crate) fn set_pixel_float(&mut self, x : isize, y : isize, px : [f32; 4])
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return;
        }
        let x = x as usize;
        let y = y as usize;
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
