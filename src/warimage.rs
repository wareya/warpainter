use eframe::egui;

use crate::pixelmath::*;

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

#[derive(Debug, Clone)]
pub (crate) enum ImageData
{
    Float(Vec<[f32; 4]>),
    Int(Vec<[u8; 4]>),
}

impl ImageData
{
    fn premultiply(mut self) -> Self
    {
        match &mut self
        {
            Self::Int(ref mut data) =>
            {
                for i in 0..data.len()
                {
                    data[i][0] = to_int(to_float(data[i][0]) * to_float(data[i][3]));
                    data[i][1] = to_int(to_float(data[i][1]) * to_float(data[i][3]));
                    data[i][2] = to_int(to_float(data[i][2]) * to_float(data[i][3]));
                }
            }
            Self::Float(ref mut data) =>
            {
                for i in 0..data.len()
                {
                    data[i][0] = data[i][0] * data[i][3];
                    data[i][1] = data[i][1] * data[i][3];
                    data[i][2] = data[i][2] * data[i][3];
                }
            }
        }
        self
    }
    fn unpremultiply(mut self) -> Self
    {
        match &mut self
        {
            Self::Int(ref mut data) =>
            {
                for i in 0..data.len()
                {
                    if data[i][3] == 0
                    {
                        continue;
                    }
                    data[i][0] = to_int(to_float(data[i][0]) / to_float(data[i][3]));
                    data[i][1] = to_int(to_float(data[i][1]) / to_float(data[i][3]));
                    data[i][2] = to_int(to_float(data[i][2]) / to_float(data[i][3]));
                }
            }
            Self::Float(ref mut data) =>
            {
                for i in 0..data.len()
                {
                    if data[i][3] == 0.0
                    {
                        continue;
                    }
                    data[i][0] = data[i][0] / data[i][3];
                    data[i][1] = data[i][1] / data[i][3];
                    data[i][2] = data[i][2] / data[i][3];
                }
            }
        }
        self
    }
    fn new_float(w : usize, h : usize) -> Self
    {
        Self::Float(vec!([0.0; 4]; w*h))
    }
    fn new_int(w : usize, h : usize) -> Self
    {
        Self::Int(vec!([0; 4]; w*h))
    }
    fn to_int(&self) -> Vec<u8>
    {
        match self
        {
            Self::Int(data) => flatten(data),
            Self::Float(data) =>
            {
                let mut out = vec!([0; 4]; data.len());
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
#[derive(Debug, Clone)]
pub (crate) struct Image
{
    pub (crate) width : usize,
    pub (crate) height : usize,
    data : ImageData,
}

impl Image
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

impl Image
{
    #[inline]
    pub (crate) fn set_pixel_wrapped(&mut self, x : isize, y : isize, px : [u8; 4])
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
    pub (crate) fn set_pixel(&mut self, x : isize, y : isize, px : [u8; 4])
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return;
        }
        self.set_pixel_wrapped(x, y, px)
    }
    #[inline]
    pub (crate) fn set_pixel_float_wrapped(&mut self, x : isize, y : isize, px : [f32; 4])
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
    pub (crate) fn set_pixel_float(&mut self, x : isize, y : isize, px : [f32; 4])
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return;
        }
        self.set_pixel_float_wrapped(x, y, px)
    }
    
    
    #[inline]
    pub (crate) fn get_pixel_wrapped(&self, x : isize, y : isize) -> [u8; 4]
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
    pub (crate) fn get_pixel(&self, x : isize, y : isize) -> [u8; 4]
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return [0; 4];
        }
        self.get_pixel_wrapped(x, y)
    }
    #[inline]
    pub (crate) fn get_pixel_float_wrapped(&self, x : isize, y : isize) -> [f32; 4]
    {
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
    pub (crate) fn get_pixel_float(&self, x : isize, y : isize) -> [f32; 4]
    {
        if x < 0 || x as usize >= self.width || y < 0 || y as usize >= self.height
        {
            return [0.0; 4];
        }
        self.get_pixel_float_wrapped(x, y)
    }
}

fn nop<T>(t : T) -> T
{
    t
}

impl Image
{
    pub (crate) fn blank(w : usize, h : usize) -> Self
    {
        let data = ImageData::new_int(w as usize, h as usize);
        let ret = Self { width : w as usize, height : h as usize, data };
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
                let px = input.get_pixel(x as u32, y as u32).0;
                ret.set_pixel(x as isize, y as isize, px);
            }
        }
        ret
    }
    pub (crate) fn blank_with_same_size(&self) -> Self
    {
        Self::blank(self.width, self.height)
    }
    pub (crate) fn blank_white_transparent(w : usize, h : usize) -> Self
    {
        let mut data = ImageData::new_int(w as usize, h as usize);
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
        let ret = Self { width : w as usize, height : h as usize, data };
        ret
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
                let img = T::from_vec(self.width as u32, self.height as u32, flatten(data)).unwrap();
                img
            }
            ImageData::Float(data) =>
            {
                type T = image::ImageBuffer::<image::Rgba<f32>, Vec<f32>>;
                let img = T::from_vec(self.width as u32, self.height as u32, flatten(data)).unwrap();
                image::DynamicImage::from(img).to_rgba8()
            }
        }
    }
    #[inline(never)]
    pub (crate) fn blend_rect_from(&mut self, rect : [[f32; 2]; 2], top : &Image, top_opacity : f32, blend_mode : &String)
    {
        let min_x = 0.max(rect[0][0].floor() as isize) as usize;
        let max_x = (self.width.min(top.width) as isize).min(rect[1][0].ceil() as isize + 1).max(0) as usize;
        let min_y = 0.max(rect[0][1].floor() as isize) as usize;
        let max_y = (self.height.min(top.height) as isize).min(rect[1][1].ceil() as isize + 1).max(0) as usize;
        
        let self_width = self.width;
        let top_width = top.width;
        
        macro_rules! do_loop
        {
            ($bottom:expr, $top:expr, $bottom_read_f:expr, $top_read_f:expr, $bottom_write_f:expr, $mix_f:expr) =>
            {
                {
                    let mut thread_count = 4;
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
                    // FEARLESS CONCURRENCY
                    crossbeam::scope(|s|
                    {
                        for info in infos
                        {
                            s.spawn(move |_|
                            {
                                let bottom = info.0;
                                let offset = info.1;
                                let min_y = 0;
                                let max_y = bottom.len()/self_width;
                                for y in min_y..max_y
                                {
                                    let self_index_y_part = y*self_width;
                                    let top_index_y_part = (y+offset)*top_width;
                                    for x in min_x..max_x
                                    {
                                        let bottom_index = self_index_y_part + x;
                                        let top_index = top_index_y_part + x;
                                        
                                        let bottom_pixel = $bottom_read_f(bottom[bottom_index]);
                                        let top_pixel = $top_read_f($top[top_index]);
                                        let c = $mix_f(top_pixel, bottom_pixel, top_opacity);
                                        bottom[bottom_index] = $bottom_write_f(c);
                                    }
                                }
                            });
                        }
                    }).unwrap();
                }
            }
        }
        
        let blend_float = match blend_mode.as_str()
        {
            "Multiply" => px_func_float::<BlendModeMultiply>,
            "Divide" => px_func_float::<BlendModeDivide>,
            "Screen" => px_func_float::<BlendModeScreen>,
            "Add" => px_func_float::<BlendModeAdd>,
            "Glow Add" => px_func_float::<BlendModeAddGlow>,
            "Subtract" => px_func_float::<BlendModeSubtract>,
            "Difference" => px_func_float::<BlendModeDifference>,
            "Signed Diff" => px_func_float::<BlendModeSignedDifference>,
            "Signed Add" => px_func_float::<BlendModeSignedAdd>,
            "Negation" => px_func_float::<BlendModeNegation>,
            "Lighten" => px_func_float::<BlendModeLighten>,
            "Darken" => px_func_float::<BlendModeDarken>,
            "Linear Burn" => px_func_float::<BlendModeLinearBurn>,
            "Color Burn" => px_func_float::<BlendModeColorBurn>,
            "Color Dodge" => px_func_float::<BlendModeColorDodge>,
            "Glow Dodge" => px_func_float::<BlendModeGlowDodge>,
            "Glow" => px_func_float::<BlendModeGlow>,
            "Reflect" => px_func_float::<BlendModeReflect>,
            "Overlay" => px_func_float::<BlendModeOverlay>,
            "Soft Light" => px_func_float::<BlendModeSoftLight>,
            "Hard Light" => px_func_float::<BlendModeHardLight>,
            "Vivid Light" => px_func_float::<BlendModeVividLight>,
            "Linear Light" => px_func_float::<BlendModeLinearLight>,
            "Pin Light" => px_func_float::<BlendModePinLight>,
            "Hard Mix" => px_func_float::<BlendModeHardMix>,
            "Exclusion" => px_func_float::<BlendModeExclusion>,
            
            "Hue" => px_func_triad_float::<BlendModeHue>,
            "Saturation" => px_func_triad_float::<BlendModeSaturation>,
            "Color" => px_func_triad_float::<BlendModeColor>,
            "Luminosity" => px_func_triad_float::<BlendModeLuminosity>,
            
            "Flat Hue" => px_func_triad_float::<BlendModeFlatHue>,
            "Flat Sat" => px_func_triad_float::<BlendModeFlatSaturation>,
            "Flat Color" => px_func_triad_float::<BlendModeFlatColor>,
            "Value" => px_func_triad_float::<BlendModeValue>,
            
            "Hard Sat" => px_func_triad_float::<BlendModeHardSaturation>,
            "Hard Color" => px_func_triad_float::<BlendModeHardColor>,
            "Lightness" => px_func_triad_float::<BlendModeLightness>,
            
            "Erase" => px_func_full_float::<BlendModeErase>,
            "Reveal" => px_func_full_float::<BlendModeReveal>,
            "Alpha Mask" => px_func_full_float::<BlendModeAlphaMask>,
            "Alpha Reject" => px_func_full_float::<BlendModeAlphaReject>,
            
            "Interpolate" => px_lerp_float,
            
            _ => px_func_float::<BlendModeNormal>, // Normal, or unknown
        };
        
        let blend_int = match blend_mode.as_str()
        {
            "Multiply" => px_func::<BlendModeMultiply>,
            "Divide" => px_func::<BlendModeDivide>,
            "Screen" => px_func::<BlendModeScreen>,
            "Add" => px_func::<BlendModeAdd>,
            "Glow Add" => px_func::<BlendModeAddGlow>,
            "Subtract" => px_func::<BlendModeSubtract>,
            "Difference" => px_func::<BlendModeDifference>,
            "Signed Diff" => px_func::<BlendModeSignedDifference>,
            "Signed Add" => px_func::<BlendModeSignedAdd>,
            "Negation" => px_func::<BlendModeNegation>,
            "Lighten" => px_func::<BlendModeLighten>,
            "Darken" => px_func::<BlendModeDarken>,
            "Linear Burn" => px_func::<BlendModeLinearBurn>,
            "Color Burn" => px_func::<BlendModeColorBurn>,
            "Color Dodge" => px_func::<BlendModeColorDodge>,
            "Glow Dodge" => px_func::<BlendModeGlowDodge>,
            "Glow" => px_func::<BlendModeGlow>,
            "Reflect" => px_func::<BlendModeReflect>,
            "Overlay" => px_func::<BlendModeOverlay>,
            "Soft Light" => px_func::<BlendModeSoftLight>,
            "Hard Light" => px_func::<BlendModeHardLight>,
            "Vivid Light" => px_func::<BlendModeVividLight>,
            "Linear Light" => px_func::<BlendModeLinearLight>,
            "Pin Light" => px_func::<BlendModePinLight>,
            "Hard Mix" => px_func::<BlendModeHardMix>,
            "Exclusion" => px_func::<BlendModeExclusion>,
            
            "Hue" => px_func_triad::<BlendModeHue>,
            "Saturation" => px_func_triad::<BlendModeSaturation>,
            "Color" => px_func_triad::<BlendModeColor>,
            "Luminosity" => px_func_triad::<BlendModeLuminosity>,
            
            "Flat Hue" => px_func_triad::<BlendModeFlatHue>,
            "Flat Sat" => px_func_triad::<BlendModeFlatSaturation>,
            "Flat Color" => px_func_triad::<BlendModeFlatColor>,
            "Value" => px_func_triad::<BlendModeValue>,
            
            "Hard Sat" => px_func_triad::<BlendModeHardSaturation>,
            "Hard Color" => px_func_triad::<BlendModeHardColor>,
            "Lightness" => px_func_triad::<BlendModeLightness>,
            
            "Erase" => px_func_full::<BlendModeErase>,
            "Reveal" => px_func_full::<BlendModeReveal>,
            "Alpha Mask" => px_func_full::<BlendModeAlphaMask>,
            "Alpha Reject" => px_func_full::<BlendModeAlphaReject>,
            
            "Interpolate" => px_lerp,
            
            _ => px_func::<BlendModeNormal>, // Normal, or unknown
        };
        
        match (&mut self.data, &top.data)
        {
            (ImageData::Float(bottom), ImageData::Float(top)) =>
                do_loop!(bottom, top, nop, nop, nop, blend_float),
            (ImageData::Float(bottom), ImageData::Int(top)) =>
                do_loop!(bottom, top, nop, px_to_float, nop, blend_float),
            (ImageData::Int(bottom), ImageData::Float(top)) =>
                do_loop!(bottom, top, px_to_float, nop, px_to_int, blend_float),
            (ImageData::Int(bottom), ImageData::Int(top)) =>
                do_loop!(bottom, top, nop, nop, nop, blend_int),
        }
    }
    pub (crate) fn blend_from(&mut self, top : &Image, top_opacity : f32, blend_mode : &String)
    {
        self.blend_rect_from([[0.0, 0.0], [self.width as f32, self.height as f32]], top, top_opacity, blend_mode)
    }
    
    pub (crate) fn resized(&mut self, new_w : usize, new_h : usize) -> Image
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
    pub (crate) fn clear_rect_with_color_float(&mut self, rect : [[f32; 2]; 2], color : [f32; 4])
    {
        for y in rect[0][1].floor().max(0.0) as isize..=(rect[1][1].ceil() as isize).min(self.height as isize)
        {
            for x in rect[0][0].floor().max(0.0) as isize..=(rect[1][0].ceil() as isize).min(self.width as isize)
            {
                self.set_pixel_float_wrapped(x, y, color);
            }
        }
    }
    pub (crate) fn clear_with_color_float(&mut self, color : [f32; 4])
    {
        for y in 0..self.height as isize
        {
            for x in 0..self.width as isize
            {
                self.set_pixel_float_wrapped(x, y, color);
            }
        }
    }
    pub (crate) fn clear(&mut self)
    {
        self.clear_with_color_float([0.0, 0.0, 0.0, 0.0]);
    }
}
