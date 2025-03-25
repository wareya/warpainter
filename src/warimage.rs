use eframe::egui;

use crate::LayerPaint;
use crate::UndoEvent;
use crate::pixelmath::*;
use crate::transform::*;

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
#[derive(Clone, Debug, Decode, Encode)]
pub (crate) enum ImageData<const N : usize>
{
    Float(Vec<[f32; N]>),
    Int(Vec<[u8; N]>),
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
#[derive(Clone, Debug, Default, Decode, Encode)]
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
    
    #[inline(never)]
    pub (crate) fn blend_rect_from(&mut self, mut rect : [[f32; 2]; 2], top : &Image<4>, mask : Option<&Image<1>>, top_opacity : f32, top_offset : [isize; 2], blend_mode : &str)
    {
        //rect[0][0] += top_offset[0] as f32;
        //rect[1][0] += top_offset[0] as f32;
        //rect[0][1] += top_offset[1] as f32;
        //rect[1][1] += top_offset[1] as f32;
        
        // top opacity is ignored if a mask is used
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
        
        let get_opacity : Box<dyn Fn(usize, usize) -> f32 + Send + Sync> = if let Some(mask) = mask
        {
            Box::new(|x : usize, y : usize| mask.get_pixel_float_wrapped(x as isize, y as isize)[0])
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
                                
                                let c = blend_f(top_pixel, bottom_pixel, opacity);
                                let c = post_f(c, top_pixel, bottom_pixel, opacity, [x, y + offset]);
                                
                                bottom[bottom_index] = $bottom_write_f(c);
                            }
                        }
                    } };
                    
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
    pub (crate) fn blend_from(&mut self, top : &Image<4>, mask : Option<&Image<1>>, top_opacity : f32, top_offset : [isize; 2], blend_mode : &str)
    {
        self.blend_rect_from([[0.0, 0.0], [self.width as f32, self.height as f32]], top, mask, top_opacity, top_offset, blend_mode)
    }
    
    pub (crate) fn analyze_edit(old_data : &Image<4>, new_data : &Image<4>, uuid : u128, rect : Option<[[f32; 2]; 2]>) -> UndoEvent
    {
        let mut min_x = new_data.width;
        let mut max_x = 0;
        let mut min_y = new_data.height;
        let mut max_y = 0;
        if let Some(rect) = rect
        {
            min_x = rect[0][0].floor() as usize;
            min_y = rect[0][1].floor() as usize;
            max_x = rect[1][0].ceil() as usize;
            max_y = rect[1][1].ceil() as usize;
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
        
        println!("{} {} {} {} {:?}", min_x, max_x, min_y, max_y, rect);
        
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

