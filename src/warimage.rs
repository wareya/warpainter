use eframe::egui;

#[inline]
pub (crate) fn px_lerp_float(a : [f32; 4], b : [f32; 4], amount : f32) -> [f32; 4]
{
    let mut r = [0.0; 4];
    for i in 0..4
    {
        r[i] = a[i] * (1.0 - amount) + b[i] * amount;
    }
    r
}
#[inline]
pub (crate) fn px_lerp(a : [u8; 4], b : [u8; 4], amount : f32) -> [u8; 4]
{
    px_to_int(px_lerp_float(px_to_float(a), px_to_float(b), amount))
}

#[inline]
pub (crate) fn px_mix_float(mut a : [f32; 4], b : [f32; 4], amount : f32) -> [f32; 4]
{
    a[3] *= amount;
    let mut r = [0.0; 4];
    for i in 0..3
    {
        r[i] = a[i] * a[3] + b[i] * b[3] * (1.0 - a[3]);
    }
    r[3] = a[3] + b[3]*(1.0 - a[3]);
    if r[3] > 0.0
    {
        r[0] /= r[3];
        r[1] /= r[3];
        r[2] /= r[3];
    }
    r
}
#[inline]
pub (crate) fn px_mix(a : [u8; 4], b : [u8; 4], amount : f32) -> [u8; 4]
{
    px_to_int(px_mix_float(px_to_float(a), px_to_float(b), amount))
}

#[inline]
pub (crate) fn to_float(x : u8) -> f32
{
    (x as f32)/255.0
}
#[inline]
pub (crate) fn to_int(x : f32) -> u8
{
    (x*255.0).round().clamp(0.0, 255.0) as u8
}

#[inline]
pub (crate) fn px_to_float(x : [u8; 4]) -> [f32; 4]
{
    [
        to_float(x[0]),
        to_float(x[1]),
        to_float(x[2]),
        to_float(x[3]),
    ]
}
#[inline]
pub (crate) fn px_to_int(x : [f32; 4]) -> [u8; 4]
{
    [
        to_int(x[0]),
        to_int(x[1]),
        to_int(x[2]),
        to_int(x[3]),
    ]
}

#[inline]
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
#[inline]
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

#[inline]
pub (crate) fn get_pixel<T : Copy>(data : &[T], x : isize, y : isize, width : isize) -> [T; 4]
{
    let index = (y*width + x) as usize * 4;
    [data[index], data[index+1], data[index+2], data[index+3]]
}

#[derive(Debug, Clone)]
pub (crate) enum ImageData
{
    Float(Vec<f32>),
    Int(Vec<u8>),
}

impl ImageData
{
    fn premultiply(mut self) -> Self
    {
        match &mut self
        {
            Self::Int(ref mut data) =>
            {
                for i in (0..data.len()).step_by(4)
                {
                    data[i+0] = to_int(to_float(data[i+0]) * to_float(data[i+3]));
                    data[i+1] = to_int(to_float(data[i+1]) * to_float(data[i+3]));
                    data[i+2] = to_int(to_float(data[i+2]) * to_float(data[i+3]));
                }
            }
            Self::Float(ref mut data) =>
            {
                for i in (0..data.len()).step_by(4)
                {
                    data[i+0] = data[i+0] * data[i+3];
                    data[i+1] = data[i+1] * data[i+3];
                    data[i+2] = data[i+2] * data[i+3];
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
                for i in (0..data.len()).step_by(4)
                {
                    if data[i+3] == 0
                    {
                        continue;
                    }
                    data[i+0] = to_int(to_float(data[i+0]) / to_float(data[i+3]));
                    data[i+1] = to_int(to_float(data[i+1]) / to_float(data[i+3]));
                    data[i+2] = to_int(to_float(data[i+2]) / to_float(data[i+3]));
                }
            }
            Self::Float(ref mut data) =>
            {
                for i in (0..data.len()).step_by(4)
                {
                    if data[i+3] == 0.0
                    {
                        continue;
                    }
                    data[i+0] = data[i+0] / data[i+3];
                    data[i+1] = data[i+1] / data[i+3];
                    data[i+2] = data[i+2] / data[i+3];
                }
            }
        }
        self
    }
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
    fn into_int(self) -> Vec<u8>
    {
        match self
        {
            Self::Int(data) => data,
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
    #[inline]
    pub (crate) fn bytes(&self) -> &[u8]
    {
        use byte_slice_cast::*;
        let bytes = match &self.data
        {
            ImageData::Int(data) => data,
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
        let index = y*self.width*4 + x*4;
        match &self.data
        {
            ImageData::Int(data) =>
            {
                [data[index], data[index+1], data[index+2], data[index+3]]
            }
            ImageData::Float(data) =>
            {
                px_to_int([data[index], data[index+1], data[index+2], data[index+3]])
            }
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
        let index = y*self.width*4 + x*4;
        match &self.data
        {
            ImageData::Int(data) =>
            {
                px_to_float([data[index], data[index+1], data[index+2], data[index+3]])
            }
            ImageData::Float(data) =>
            {
                [data[index], data[index+1], data[index+2], data[index+3]]
            }
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
                for px in data.chunks_exact_mut(4)
                {
                    for (i, c) in px.iter_mut().enumerate()
                    {
                        *c = [255, 255, 255, 0][i];
                    }
                }
            }
            ImageData::Float(ref mut data) =>
            {
                for px in data.chunks_exact_mut(4)
                {
                    for (i, c) in px.iter_mut().enumerate()
                    {
                        *c = [1.0, 1.0, 1.0, 0.0][i];
                    }
                }
            }
        }
        let ret = Self { width : w as usize, height : h as usize, data };
        ret
    }
    // FIXME scrap this. `from_rgba_unmultiplied` is slow and not realtime-friendly.
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
                let img = T::from_vec(self.width as u32, self.height as u32, data.clone()).unwrap();
                img
            }
            ImageData::Float(data) =>
            {
                type T = image::ImageBuffer::<image::Rgba<f32>, Vec<f32>>;
                let img = T::from_vec(self.width as u32, self.height as u32, data.clone()).unwrap();
                image::DynamicImage::from(img).to_rgba8()
            }
        }
    }
    pub (crate) fn blend_rect_from(&mut self, rect : [[f32; 2]; 2], top : &Image, top_opacity : f32)
    {
        for y in rect[0][1].floor().max(0.0) as isize..(rect[1][1].ceil() as isize + 1).min(self.height as isize)
        {
            for x in rect[0][0].floor().max(0.0) as isize..(rect[1][0].ceil() as isize + 1).min(self.width as isize)
            {
                let bottom_pixel = self.get_pixel_float_wrapped(x, y);
                let top_pixel = top.get_pixel_float(x, y);
                let c = px_mix_float(top_pixel, bottom_pixel, top_opacity);
                self.set_pixel_float_wrapped(x, y, c);
            }
        }
    }
    pub (crate) fn blend_from(&mut self, top : &Image, top_opacity : f32)
    {
        for y in 0..self.height as isize
        {
            for x in 0..self.width as isize
            {
                let bottom_pixel = self.get_pixel_float_wrapped(x, y);
                let top_pixel = top.get_pixel_float(x, y);
                let c = px_mix_float(top_pixel, bottom_pixel, top_opacity);
                self.set_pixel_float_wrapped(x, y, c);
            }
        }
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
        for y in rect[0][1].floor().max(0.0) as isize..(rect[1][1].ceil() as isize).min(self.height as isize)
        {
            for x in rect[0][0].floor().max(0.0) as isize..(rect[1][0].ceil() as isize).min(self.width as isize)
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
