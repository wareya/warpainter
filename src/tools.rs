use crate::warimage::*;
use crate::transform::*;
use crate::canvas::CanvasInputState;
use crate::gizmos::*;
use crate::pixelmath::*;

use crate::egui;
use crate::egui::Ui;
use crate::egui::SliderClamping;

enum ReferenceMode
{
    CurrentLayer,
    CurrentFolder,
    Merged,
}

pub (crate) trait Tool
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState);
    fn notify_tool_changed(&mut self, app : &mut crate::Warpainter);
    fn is_brushlike(&self) -> bool; // ctrl is color picker, otherwise tool-contolled
    fn get_gizmo(&self, app : &crate::Warpainter, focused : bool) -> Option<Box<dyn Gizmo>>;
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>;
    fn settings_panel(&mut self, app : &crate::Warpainter, ui : &mut Ui);
}

pub (crate) struct Fill
{
    threshold : f32,
    prev_input : CanvasInputState,
}

impl Fill
{
    pub (crate) fn new() -> Self
    {
        Fill { threshold : 0.5/255.0, prev_input : CanvasInputState::default() }
    }
}
impl Tool for Fill
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        if new_input.held[0] && !self.prev_input.held[0]
        {
            app.begin_edit(false, false);
            
            //let start = std::time::SystemTime::now();
            
            let mut prev_coord = self.prev_input.canvas_mouse_coord;
            let mut coord = new_input.canvas_mouse_coord;
            
            let color = app.main_color_rgb;
            
            let layer = app.layers.find_layer_unlocked(app.current_layer);
            if let Some(Some(base)) = layer.map(|x| x.data.as_ref())
            {
                let layer = layer.unwrap();
                let offset = layer.offset;
                coord = vec_add(&coord, &offset);
                prev_coord = vec_add(&prev_coord, &offset);
                
                if let Some(image) = app.editing_image.as_mut()
                {
                    if !self.prev_input.held[0] || prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                    {
                        let mut rect = [coord, coord];
                        
                        let coord = [coord[0] as isize, coord[1] as isize];
                        let ref_color = base.get_pixel_float(coord[0], coord[1]);
                        
                        fn compare_dist(a : [f32; 4], b : [f32; 4], r : f32) -> bool
                        {
                            let mut d : f32 = 0.0;
                            for i in 0..4
                            {
                                //d += (b[i]-a[i]).abs();
                                d = d.max((b[i]-a[i]).abs());
                            }
                            d <= r
                        }
                        
                        let mut visited = vec!(false; base.width*base.height);
                        let mut frontier = vec!();
                        let mut max_f_size = 0;
                        
                        if coord[0] >= 0 && coord[0] < base.width as isize
                        && coord[1] >= 0 && coord[1] < base.height as isize
                        {
                            frontier.push(coord);
                        }
                        
                        let mut streak_up = false;
                        let mut streak_down = false;
                        let mut last_coord = coord;
                        while let Some(coord) = frontier.pop()
                        {
                            rect = rect_enclose_rect(rect, [[coord[0] as f32, coord[1] as f32], [coord[0] as f32, coord[1] as f32]]);
                            max_f_size = max_f_size.max(frontier.len());
                            
                            if last_coord[1] != coord[1]
                            {
                                streak_up = false;
                                streak_down = false;
                            }
                            last_coord = coord;
                            
                            let x = coord[0];
                            let y = coord[1];
                            visited[y as usize*base.width + x as usize] = true;
                            image.set_pixel_float_wrapped(x, y, color);
                            for add in [[0, -1], [0, 1], [1, 0], [-1, 0]]
                            //for add in [[1, 0], [0, 1], [-1, 0], [0, -1]]
                            {
                                let coord = vec_add(&coord, &add);
                                let x = coord[0];
                                let y = coord[1];
                                if x < 0 || x >= base.width as isize
                                || y < 0 || y >= base.height as isize
                                {
                                    continue;
                                }
                                let cond1 = !visited[y as usize*image.width + x as usize];
                                let cond2 = compare_dist(base.get_pixel_float_wrapped(coord[0], coord[1]), ref_color, self.threshold);
                                
                                // organizing this this way is more comprehensible
                                #[allow(clippy::collapsible_else_if)]
                                if cond1 && cond2
                                {
                                    if add[1] == 0
                                    {
                                        frontier.push(coord);
                                    }
                                    if add[1] == 1 && !streak_up
                                    {
                                        frontier.push(coord);
                                        streak_up = true;
                                    }
                                    if add[1] == -1 && !streak_down
                                    {
                                        frontier.push(coord);
                                        streak_down = true;
                                    }
                                }
                                else
                                {
                                    if add[1] == 1
                                    {
                                        streak_up = false;
                                    }
                                    else if add[1] == -1
                                    {
                                        streak_down = false;
                                    }
                                }
                            }
                        }
                        
                        app.mark_current_layer_dirty(grow_box(rect, [1.0, 1.0]));
                        
                        println!("max frontier size... {}", max_f_size);
                    }
                }
                
                /*
                let elapsed = start.elapsed();
                let elapsed = match elapsed { Ok(x) => x.as_secs_f64(), Err(x) => x.duration().as_secs_f64() };
                if elapsed > 0.01
                {
                    println!("time to flood fill: {}", elapsed);
                }
                */
            }
            app.commit_edit();
        }
        
        self.prev_input = new_input.clone();
    }
    fn notify_tool_changed(&mut self, _app : &mut crate::Warpainter)
    {
        
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, _app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        None
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>
    {
        Some((app.icons.get("tool fill").as_ref().unwrap(), [2.0, 18.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        ui.label("Threshold");
        let mut threshold = self.threshold * 255.0;
        ui.add(egui::Slider::new(&mut threshold, 0.0..=255.0).clamping(SliderClamping::Always));
        self.threshold = threshold/255.0;
    }
}

fn draw_line_no_start_float(image : &mut Image<4>, mut from : [f32; 2], mut to : [f32; 2], color : [f32; 4])
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
        let mut coord = [coord[0] as f64, coord[1] as f64];
        
        // fix unbalanced 6-by-3 (etc) lines
        let vi = if diff[0].abs() < diff[1].abs() { 0 } else { 1 };
        if (amount - 0.5) * 1.0f32.copysign(diff[vi]) + 0.5 > 0.5
        {
            coord[vi] -= 1.0 / (2.1 * max as f64);
        }
        
        let x = coord[0].round() as isize;
        let y = coord[1].round() as isize;
        image.set_pixel_float(x, y, color);
    }
}
fn draw_line_no_start(image : &mut Image<4>, from : [f32; 2], to : [f32; 2], color : [u8; 4])
{
    draw_line_no_start_float(image, from, to, px_to_float(color))
}

type BrushData = Vec<Vec<((isize, isize), [f32; 4])>>;
#[allow(clippy::too_many_arguments)]
fn draw_brush_line_no_start_float(image : &mut Image<4>, mut from : [f32; 2], mut to : [f32; 2], color : [f32; 4], brush : &BrushData, offset : [isize; 2], erase : bool, alpha_lock : bool)
{
    fn dir_index(x_d : isize, y_d : isize) -> usize
    {
        match (x_d, y_d)
        {
            ( 1,  0) => 0,
            ( 1,  1) => 1,
            ( 0,  1) => 2,
            (-1,  1) => 3,
            (-1,  0) => 4,
            (-1, -1) => 5,
            ( 0, -1) => 6,
            ( 1, -1) => 7,
            _ => 1000,
        }
    }
    from[0] = from[0].floor();
    from[1] = from[1].floor();
    to[0] = to[0].floor();
    to[1] = to[1].floor();
    let diff = vec_sub(&from, &to);
    let max = diff[0].abs().max(diff[1].abs());
    let mut prev_x = from[0].round() as isize;
    let mut prev_y = from[1].round() as isize;
    for i in 1..=max as usize
    {
        let amount = i as f32 / max;
        let coord = vec_lerp(&from, &to, amount);
        let mut coord = [coord[0] as f64, coord[1] as f64];
        
        // fix unbalanced 6-by-3 (etc) lines
        let vi = if diff[0].abs() < diff[1].abs() { 0 } else { 1 };
        if (amount - 0.5) * 1.0f32.copysign(diff[vi]) + 0.5 > 0.5
        {
            coord[vi] -= 1.0 / (2.1 * max as f64);
        }
        
        let x = coord[0].round() as isize;
        let y = coord[1].round() as isize;
        let dir = dir_index(x - prev_x, y - prev_y);
        prev_x = x;
        prev_y = y;
        if dir == 1000
        {
            continue;
        }
        let brush_shape = &brush[dir];
        for ((ux, uy), c) in brush_shape
        {
            let under_c = image.get_pixel_float(x + ux - offset[0], y + uy - offset[1]);
            let mut c = *c;
            if !erase
            {
                if c[3] > 0.0
                {
                    c[0] *= color[0];
                    c[1] *= color[1];
                    c[2] *= color[2];
                    c[3] *= color[3];
                    if alpha_lock
                    {
                        c[3] = c[3].min(under_c[3]);
                    }
                    image.set_pixel_float(x + ux - offset[0], y + uy - offset[1], c);
                }
            }
            else
            {
                let mut c = image.get_pixel_float(x + ux - offset[0], y + uy - offset[1]);
                c[3] = 0.0;
                image.set_pixel_float(x + ux - offset[0], y + uy - offset[1], c);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_brush_line_no_start(image : &mut Image<4>, from : [f32; 2], to : [f32; 2], color : [u8; 4], brush : &BrushData, offset : [isize; 2], erase : bool, alpha_lock : bool)
{
    draw_brush_line_no_start_float(image, from, to, px_to_float(color), brush, offset, erase, alpha_lock)
}
fn draw_brush_at_float(image : &mut Image<4>, at : [f32; 2], color : [f32; 4], brush_shape : &Image<4>, erase : bool, alpha_lock : bool)
{
    let x = at[0].floor() as isize;
    let y = at[1].floor() as isize;
    for uy in 0..brush_shape.height as isize
    {
        for ux in 0..brush_shape.width as isize
        {
            let under_c = image.get_pixel_float(x + ux - (brush_shape.width/2) as isize, y + uy - (brush_shape.height/2) as isize);
            let mut c = brush_shape.get_pixel_float(ux, uy);
            if c[3] > 0.0
            {
                if !erase
                {
                    c[0] *= color[0];
                    c[1] *= color[1];
                    c[2] *= color[2];
                    c[3] *= color[3];
                    if alpha_lock
                    {
                        c[3] = c[3].min(under_c[3]);
                    }
                    image.set_pixel_float(x + ux - (brush_shape.width/2) as isize, y + uy - (brush_shape.height/2) as isize, c);
                }
                else
                {
                    let mut c = image.get_pixel_float(x + ux - (brush_shape.width/2) as isize, y + uy - (brush_shape.height/2) as isize);
                    c[3] = 0.0;
                    image.set_pixel_float(x + ux - (brush_shape.width/2) as isize, y + uy - (brush_shape.height/2) as isize, c);
                }
            }
        }
    }
}
fn draw_brush_at(image : &mut Image<4>, at : [f32; 2], color : [u8; 4], brush_shape : &Image<4>, erase : bool, alpha_lock : bool)
{
    draw_brush_at_float(image, at, px_to_float(color), brush_shape, erase, alpha_lock)
}
fn blend_brush_at_float(image : &mut Image<4>, center : [f32; 2], color : [f32; 4], brush_shape : &Image<4>, erase : bool, alpha_lock : bool, mode : String, brush_scale : f32)
{
    let func = find_blend_func_float(&mode);
    
    let w = brush_shape.width as f32 * brush_scale;
    let h = brush_shape.height as f32 * brush_scale;
    
    let realx = center[0] - (w - 1.0) * 0.5;
    let realy = center[1] - (h - 1.0) * 0.5;
    
    let x = realx.floor() as isize;
    let y = realy.floor() as isize;
    
    println!("{} {} {} {}", x, y, realx, realy);
    
    for uy in 0..=h.ceil() as isize+1
    {
        let iy = y + uy;
        let dy = (iy as f32 - realy) / brush_scale;
        for ux in 0..=w.ceil() as isize+1
        {
            let ix = x + ux;
            let dx = (ix as f32 - realx) / brush_scale;
            let mut c = brush_shape.get_pixel_float_lerped(dx as f32, dy as f32);
            //let mut c = brush_shape.get_pixel_float_lerped(0.5, 0.5);
            if c[3] > 0.0
            {
                let mut under_c = image.get_pixel_float(ix, iy);
                if !erase
                {
                    c[0] *= color[0];
                    c[1] *= color[1];
                    c[2] *= color[2];
                    under_c = func(c, under_c, color[3], 1.0, false);
                    if alpha_lock
                    {
                        under_c[3] = under_c[3].min(c[3]);
                    }
                }
                else
                {
                    under_c[3] = lerp(under_c[3], 0.0, c[3] * color[3]);
                }
                image.set_pixel_float(ix, iy, under_c);
            }
        }
    }
}
fn blend_brush_at(image : &mut Image<4>, at : [f32; 2], color : [u8; 4], brush_shape : &Image<4>, erase : bool, alpha_lock : bool, mode : String, brush_scale : f32)
{
    blend_brush_at_float(image, at, px_to_float(color), brush_shape, erase, alpha_lock, mode, brush_scale)
}

fn grow_box(mut rect : [[f32; 2]; 2], grow_size : [f32; 2]) -> [[f32; 2]; 2]
{
    rect = rect_normalize(rect);
    rect[0][0] -= grow_size[0];
    rect[0][1] -= grow_size[1];
    rect[1][0] += grow_size[0];
    rect[1][1] += grow_size[1];
    rect
}

fn generate_brush(size : f32, no_aa : bool) -> Image<4>
{
    let img_size = size.ceil() as usize;
    let mut shape = Image::blank(img_size, img_size);
    for uy in 0..img_size as isize
    {
        let y = uy as f32 - (img_size as f32)*0.5 + 0.5;
        for ux in 0..img_size as isize
        {
            let x = ux as f32 - (img_size as f32)*0.5 + 0.5;
            if no_aa
            {
                if y*y + x*x < size*size/4.0
                {
                    shape.set_pixel(ux, uy, [255, 255, 255, 255]);
                }
            }
            else
            {
                let mut f = (y*y + x*x).sqrt();
                f -= size * 0.5 - 0.5;
                f = 1.0 - f.clamp(0.0, 1.0);
                shape.set_pixel(ux, uy, [255, 255, 255, (f * 255.99) as u8]);
            }
        }
    }
    shape
}
fn directionalize_brush(brush_shape : &Image<4>) -> BrushData
{
    let mut ret = Vec::new();
    let dirs = [
        [ 1,  0],
        [ 1,  1],
        [ 0,  1],
        [-1,  1],
        [-1,  0],
        [-1, -1],
        [ 0, -1],
        [ 1, -1],
    ];
    for dir in dirs
    {
        let mut new_brush = Vec::new();
        for uy in 0..brush_shape.height as isize
        {
            for ux in 0..brush_shape.width as isize
            {
                let current = brush_shape.get_pixel_float(ux, uy);
                let next = brush_shape.get_pixel_float(ux + dir[0], uy + dir[1]);
                if current[3] > 0.0 && next[3] == 0.0
                {
                    new_brush.push(((ux, uy), current));
                }
            }
        }
        // needed for brushes that have natural diagonal "gaps", but we don't generate any yet
        // also, we still need to change this to search over the vec instead of using get_pixel
        /*
        if dir[0].abs() == dir[1].abs()
        {
            for uy in 0..new_brush.height as isize
            {
                for ux in 0..new_brush.width as isize
                {
                    let next_x = brush_shape.get_pixel(ux + dir[0], uy);
                    let next_y = brush_shape.get_pixel(ux, uy + dir[1]);
                    if next_x[3] > 0 && next_y[3] > 0
                    {
                        new_brush.set_pixel(ux, uy, next_y);
                    }
                }
            }
        }*/
        ret.push(new_brush);
    }
    
    ret
}

#[allow(clippy::type_complexity)]
pub (crate) struct Pencil
{
    size : f32,
    brush_shape : Image<4>,
    outline_data : Vec<Vec<[f32; 2]>>,
    direction_shapes : Vec<Vec<((isize, isize), [f32; 4])>>,
    prev_input : CanvasInputState,
    cursor_memory : [f32; 2],
    cursor_log : Vec<[f32; 2]>,
    smooth_mode : bool,
    replace : bool,
    spline : u32,
    is_eraser : bool,
    spacing : bool,
    last_blot : [f32; 2],
}

impl Pencil
{
    pub (crate) fn new() -> Self
    {
        let size = 1.0;
        let brush_shape = generate_brush(size, true);
        let outline_data = brush_shape.analyze_outline();
        let direction_shapes = directionalize_brush(&brush_shape);
        Pencil {
            size,
            brush_shape,
            outline_data,
            direction_shapes,
            prev_input : CanvasInputState::default(),
            cursor_memory : [0.0, 0.0],
            cursor_log : vec!([0.0, 0.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]),
            smooth_mode : false,
            replace : true,
            spline : 1,
            is_eraser : false,
            spacing : false,
            last_blot : [0.0, 0.0],
        }
    }
    pub (crate) fn into_eraser(mut self) -> Self
    {
        self.is_eraser = true;
        self
    }
    pub (crate) fn update_brush(&mut self)
    {
        self.brush_shape = generate_brush(self.size, self.replace);
        self.outline_data = self.brush_shape.analyze_outline();
        self.direction_shapes = directionalize_brush(&self.brush_shape);
    }
}

impl Tool for Pencil
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        let mut new_input = new_input.clone();
        let a = if self.replace && self.brush_shape.width  & 1 == 0 { 0.5 } else { 0.0 };
        let b = if self.replace && self.brush_shape.height & 1 == 0 { 0.5 } else { 0.0 };
        new_input.canvas_mouse_coord[0] += a;
        new_input.canvas_mouse_coord[1] += b;
        
        let mut _old = self.prev_input.clone();
        let mut _new = new_input.clone();
        _old.time = 0.0;
        _old.delta = 0.0;
        _new.time = 0.0;
        _new.delta = 0.0;
        if _old == _new
        {
            //println!("duplicated.");
            return;
        }
        
        // press
        if new_input.held[0] && !self.prev_input.held[0]
        {
            app.begin_edit(self.replace || self.is_eraser || app.eraser_mode, false);
            self.cursor_memory = new_input.canvas_mouse_coord;
            if self.replace
            {
                self.cursor_memory = vec_floor(&self.cursor_memory);
            }
            
            for n in self.cursor_log.iter_mut()
            {
                *n = new_input.canvas_mouse_coord;
            }
            
            app.debug(format!("input event starting at {:?}", new_input.canvas_mouse_coord));
        }
        
        self.cursor_log.push(new_input.canvas_mouse_coord);
        self.cursor_log.remove(0);
        
        let in_0 = self.cursor_log[0];
        let in_1 = self.cursor_log[1];
        let in_2 = self.cursor_log[2];
        let in_3 = self.cursor_log[3];
        
        let vel_1 = vec_sub(&in_1, &in_0);
        let vel_2 = vec_sub(&in_2, &in_1);
        let vel_3 = vec_sub(&in_3, &in_2);
        let accel_2 = vec_sub(&vel_2, &vel_1);
        let accel_3 = vec_sub(&vel_3, &vel_2);
        
        let vel3_not = vec_add(&vel_2, &accel_2);
        let vel4_not = vec_add(&vel_3, &accel_3);
        
        let in_3_not = vec_add(&in_2, &vel3_not);
        let in_4_not = vec_add(&in_3, &vel4_not);
        
        let tanc_2 = vec_mul_scalar(&vec_sub(&in_3, &in_1), 0.5);
        
        let tanc_3not = vec_mul_scalar(&vec_sub(&in_4_not, &in_2), 0.5);
        let tanc_2not = vec_mul_scalar(&vec_sub(&in_3_not, &in_1), 0.5);
        
        let _tan_1 = vec_sub(&in_2, &in_1);
        let _tan_2 = vec_sub(&in_3, &in_2);
        
        let mut prev_in = in_2;
        
        for t in 1..=self.spline
        {
            let t = t as f32 * (1.0 / self.spline as f32);
            
            /*
            // real but high latency
            let h00 = 2.0*t*t*t - 3.0*t*t + 1.0;
            let h01 = -2.0*t*t*t + 3.0*t*t;
            let h10 = t*t*t - 2.0*t*t + t;
            let h11 = t*t*t - t*t;
            
            let new_in = vec_add(&vec_mul_scalar(&in_1, h00), &vec_mul_scalar(&in_2, h01));
            let new_in = vec_add(&new_in, &vec_mul_scalar(&tanc_1, h10));
            let new_in = vec_add(&new_in, &vec_mul_scalar(&tanc_2, h11));
            */
            
            /*
            let h00 = 1.0 - t*t;
            let h01 = t*t;
            let h10 = t*(1.0-t)*t;
            let h11 = -h10;
            
            let new_in = vec_add(&vec_mul_scalar(&in_2, h00), &vec_mul_scalar(&in_3, h01));
            let new_in = vec_add(&new_in, &vec_mul_scalar(&tan_1, h10));
            let new_in = vec_add(&new_in, &vec_mul_scalar(&tan_2, h11));
            */
            
            let h00 = 2.0*t*t*t - 3.0*t*t + 1.0;
            let h01 = -2.0*t*t*t + 3.0*t*t;
            let h10 = t*t*t - 2.0*t*t + t;
            let h11 = t*t*t - t*t;
            
            let mut tan_out = tanc_3not;
            //let tan_in = vec_lerp(&tanc_2not, &tanc_2, 1.0-(1.0-t)*(1.0-t));
            let mut tan_in = vec_lerp(&tanc_2not, &tanc_2, t);
            //let tan_in = vec_lerp(&tanc_2not, &tanc_2, (t*4.0).clamp(0.0, 1.0));
            //let tan_in = vec_lerp(&tanc_2not, &tanc_2, t.sqrt());
            //let tan_in = tanc_2;
            
            let fac = (vec_len(&vel_3)*t/(vec_len(&vel_2)*0.05+0.1)).min(vec_len(&vel_2)*(1.0-t)/(vec_len(&vel_3)*0.05+0.1)).min(1.0);
            tan_in[0] *= fac;
            tan_in[1] *= fac;
            tan_out[0] *= fac;
            tan_out[1] *= fac;
            
            let new_in = vec_add(&vec_mul_scalar(&in_2, h00), &vec_mul_scalar(&in_3, h01));
            let new_in = vec_add(&new_in, &vec_mul_scalar(&tan_in, h10));
            let new_in = vec_add(&new_in, &vec_mul_scalar(&tan_out, h11));
            
            // press or hold or release
            if new_input.held[0] || self.prev_input.held[0]
            {
                let do_smooth = self.smooth_mode;
                let prev_coord = if self.smooth_mode { self.cursor_memory } else { if self.replace { vec_floor(&prev_in) } else { vec_sub(&prev_in, &[0.5, 0.5]) } };
                let mut coord = if self.replace { vec_floor(&new_in) } else { vec_sub(&new_in, &[0.5, 0.5]) };
                
                // broken lint
                #[allow(clippy::suspicious_else_formatting)]
                if do_smooth
                {
                    let coord_d = vec_sub(&coord, &prev_coord);
                    if coord_d[0].abs() > 1.0 || coord_d[1].abs() > 1.0
                    {
                        // exact diagonal movement
                        if coord_d[0].abs() == coord_d[1].abs()
                        {
                            coord = vec_sub(&coord, &[coord_d[0].clamp(-1.0, 1.0), coord_d[1].clamp(-1.0, 1.0)]);
                        }
                        // more horizontal
                        else if coord_d[0].abs() > coord_d[1].abs()
                        {
                            coord = vec_sub(&coord, &[coord_d[0].clamp(-1.0, 1.0), 0.0]);
                        }
                        // more vertical
                        else
                        {
                            coord = vec_sub(&coord, &[0.0, coord_d[1].clamp(-1.0, 1.0)]);
                        }
                    }
                    // not enough motion to move
                    else
                    {
                        coord = prev_coord;
                    }
                }
                
                let coord2 = vec_sub(&coord, &app.get_editing_offset());
                let prev_coord2 = vec_sub(&prev_coord, &app.get_editing_offset());
                
                let color = app.main_color_rgb;
                let eraser = app.eraser_mode || self.is_eraser;
                let alpha_locked = app.current_layer_is_alpha_locked();
                if let Some(image) = app.get_editing_image()
                {
                    let size_vec = [self.brush_shape.width as f32 + 1.0, self.brush_shape.height as f32 + 1.0];
                    let offset_vec = [(self.brush_shape.width/2) as isize, (self.brush_shape.height/2) as isize];
                    if !self.prev_input.held[0]
                    {
                        if !self.replace
                        {
                            blend_brush_at_float(image, coord2, color, &self.brush_shape, eraser, alpha_locked, "Normal".to_string(), new_input.pressure);
                        }
                        else
                        {
                            draw_brush_at_float(image, coord2, color, &self.brush_shape, eraser, alpha_locked);
                        }
                        app.mark_current_layer_dirty(grow_box([coord, coord], size_vec));
                        self.last_blot = coord2;
                    }
                    else if prev_coord[0] != coord[0] || prev_coord[1] != coord[1]
                    {
                        if !self.replace
                        {
                            let mut d = vec_sub(&coord2, &self.last_blot);
                            let inc = (self.size * 0.25 * new_input.pressure).max(0.25);
                            while vec_len(&d) >= inc
                            {
                                let next = vec_add(&self.last_blot, &vec_mul_scalar(&vec_normalize(&d), inc));
                                blend_brush_at_float(image, next, color, &self.brush_shape, eraser, alpha_locked, "Normal".to_string(), new_input.pressure);
                                self.last_blot = next;
                                d = vec_sub(&coord2, &self.last_blot);
                            }
                            app.mark_current_layer_dirty(grow_box([prev_coord, coord], size_vec));
                        }
                        else
                        {
                            draw_brush_line_no_start_float(image, prev_coord2, coord2, color, &self.direction_shapes, offset_vec, eraser, alpha_locked);
                            app.mark_current_layer_dirty(grow_box([prev_coord, coord], size_vec));
                        }
                    }
                }
                
                self.cursor_memory = coord;
            }
            else
            {
                self.cursor_memory = vec_floor(&new_input.canvas_mouse_coord);
            }
            
            prev_in = new_in;
        }
        // release
        if !new_input.held[0] && self.prev_input.held[0]
        {
            app.commit_edit();
        }
        if new_input.held[1] && !self.prev_input.held[1]
        {
            app.cancel_edit();
        }
        
        self.prev_input = new_input;
    }
    fn notify_tool_changed(&mut self, _app : &mut crate::Warpainter)
    {
        
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = self.cursor_memory;
        pos[0] -= app.canvas_width as f32 / 2.0;
        pos[1] -= app.canvas_height as f32 / 2.0;
        let mut loops = self.outline_data.clone();
        for points in loops.iter_mut()
        {
            for point in points.iter_mut()
            {
                *point = vec_add(point, &[pos[0], pos[1]]);
                *point = vec_sub(point, &[(self.brush_shape.width as f32/2.0).floor(), (self.brush_shape.height as f32/2.0).floor()]);
            }
        }
        let gizmo = OutlineGizmo { loops, filled : false };
        Some(Box::new(gizmo))
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>
    {
        Some((app.icons.get("tool pencil").as_ref().unwrap(), [2.0, 19.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        ui.label("Size");
        let old_size = self.size;
        ui.add(egui::Slider::new(&mut self.size, 1.0..=64.0).step_by(1.0).logarithmic(true).clamping(SliderClamping::Always));
        if self.size != old_size
        {
            self.update_brush();
        }
        
        ui.checkbox(&mut self.smooth_mode, "Smooth Diagonals");
        
        let old_replace = self.replace;
        ui.checkbox(&mut self.replace, "No Blending");
        if self.replace != old_replace
        {
            self.update_brush();
        }
        
        ui.label("Spline Smoothing");
        ui.label("(for low FPS devices)");
        let mut spline = self.spline as f32;
        ui.add(egui::Slider::new(&mut spline, 1.0..=16.0).step_by(1.0).clamping(SliderClamping::Always));
        self.spline = spline as u32;
    }
}

#[allow(clippy::type_complexity)]
pub (crate) struct Line
{
    size : f32,
    brush_shape : Image<4>,
    outline_data : Vec<Vec<[f32; 2]>>,
    direction_shapes : Vec<Vec<((isize, isize), [f32; 4])>>,
    cursor_memory : [f32; 2],
    prev_input : CanvasInputState,
    is_eraser : bool,
}

impl Line
{
    pub (crate) fn new() -> Self
    {
        let size = 1.0;
        let brush_shape = generate_brush(size, true);
        let outline_data = brush_shape.analyze_outline();
        let direction_shapes = directionalize_brush(&brush_shape);
        Line {
            size,
            brush_shape,
            outline_data,
            direction_shapes,
            cursor_memory : [0.0, 0.0],
            prev_input : CanvasInputState::default(),
            is_eraser : false,
        }
    }
    pub (crate) fn into_eraser(mut self) -> Self
    {
        self.is_eraser = true;
        self
    }
    pub (crate) fn update_brush(&mut self)
    {
        self.brush_shape = generate_brush(self.size, true);
        self.outline_data = self.brush_shape.analyze_outline();
        self.direction_shapes = directionalize_brush(&self.brush_shape);
    }
}

impl Tool for Line
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        let mut new_input = new_input.clone();
        let a = if self.brush_shape.width  & 1 == 0 { 0.5 } else { 0.0 };
        let b = if self.brush_shape.height & 1 == 0 { 0.5 } else { 0.0 };
        new_input.canvas_mouse_coord[0] += a;
        new_input.canvas_mouse_coord[1] += b;
        
        if new_input.held[0] && !self.prev_input.held[0]
        {
            app.begin_edit(true, false);
            self.cursor_memory = vec_floor(&new_input.canvas_mouse_coord);
        }
        // press or hold or release
        if new_input.held[0] || self.prev_input.held[0]
        {
            let prev_coord = vec_floor(&self.prev_input.canvas_mouse_coord);
            let coord = vec_floor(&new_input.canvas_mouse_coord);
            
            if prev_coord != coord || (new_input.held[0] && !self.prev_input.held[0])
            {
                app.cancel_edit();
                app.begin_edit(true, false);
                
                let color = app.main_color_rgb;
                let eraser = app.eraser_mode || self.is_eraser;
                let alpha_locked = app.current_layer_is_alpha_locked();
                if let Some(image) = app.get_editing_image()
                {
                    let size_vec = [self.brush_shape.width as f32, self.brush_shape.height as f32];
                    let offset_vec = [(self.brush_shape.width/2) as isize, (self.brush_shape.height/2) as isize];
                    
                    draw_brush_at_float(image, self.cursor_memory, color, &self.brush_shape, eraser, alpha_locked);
                    if prev_coord != coord
                    {
                        draw_brush_line_no_start_float(image, self.cursor_memory, coord, color, &self.direction_shapes, offset_vec, eraser, alpha_locked);
                    }
                    app.mark_current_layer_dirty(grow_box([coord, coord], size_vec));
                }
            }
        }
        else
        {
            self.cursor_memory = vec_floor(&new_input.canvas_mouse_coord);
        }
        // release
        if !new_input.held[0] && self.prev_input.held[0]
        {
            app.commit_edit();
        }
        if new_input.held[1] && !self.prev_input.held[1]
        {
            app.cancel_edit();
        }
        
        self.prev_input = new_input;
    }
    fn notify_tool_changed(&mut self, _app : &mut crate::Warpainter)
    {
        
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = vec_floor(&self.prev_input.canvas_mouse_coord);
        pos[0] -= app.canvas_width as f32 / 2.0;
        pos[1] -= app.canvas_height as f32 / 2.0;
        let mut loops = self.outline_data.clone();
        for points in loops.iter_mut()
        {
            for point in points.iter_mut()
            {
                *point = vec_add(point, &[pos[0], pos[1]]);
                *point = vec_sub(point, &[(self.brush_shape.width as f32/2.0).floor(), (self.brush_shape.height as f32/2.0).floor()]);
            }
        }
        let gizmo = OutlineGizmo { loops, filled : false };
        Some(Box::new(gizmo))
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>
    {
        Some((app.icons.get("crosshair").as_ref().unwrap(), [6.0, 6.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        ui.label("Size");
        let old_size = self.size;
        ui.add(egui::Slider::new(&mut self.size, 1.0..=64.0).step_by(1.0).logarithmic(true).clamping(SliderClamping::Always));
        if self.size != old_size
        {
            self.update_brush();
        }
    }
}

pub (crate) struct Selection
{
    start_point : Option<[f32; 2]>,
    current_point : Option<[f32; 2]>,
    outline_data : Vec<Vec<[f32; 2]>>,
    prev_input : CanvasInputState,
}

impl Selection
{
    pub (crate) fn new() -> Self
    {
        Selection {
            start_point : None,
            current_point : None,
            outline_data : Vec::new(),
            prev_input : CanvasInputState::default(),
        }
    }
    fn get_loops(mut rect : [[f32; 2]; 2], app : &crate::Warpainter) -> Vec<Vec<[f32; 2]>>
    {
        fn peak_wave(mut x : f32) -> f32
        {
            x += core::f32::consts::PI * 2.0;
            x = x.rem_euclid(core::f32::consts::PI * 0.5) + 0.25 * core::f32::consts::PI;
            x.sin() * 2.0f32.sqrt()
        }
        
        for point in rect.iter_mut()
        {
            point[0] += 0.5;
            point[1] += 0.5;
            *point = &app.xform * &*point;
        }
        
        rect = rect_normalize(rect);
        
        let r = app.xform.get_rotation();
        let f = peak_wave(r/180.0*core::f32::consts::PI) * app.xform.get_scale() * 0.5;
        
        rect[0] = vec_sub(&rect[0], &[f, f]);
        rect[1] = vec_add(&rect[1], &[f, f]);
        
        let mut loops = vec!(vec!(
            rect[0],
            [rect[1][0], rect[0][1]],
            rect[1],
            [rect[0][0], rect[1][1]],
            rect[0],
        ));
        
        for points in loops.iter_mut()
        {
            for point in points.iter_mut()
            {
                *point = &app.xform.inverse() * &*point;
            }
        }
        
        loops
    }
}
impl Tool for Selection
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        // press
        if new_input.held[0] && !self.prev_input.held[0]
        {
            app.clear_selection();
            self.start_point = Some(vec_floor(&new_input.canvas_mouse_coord));
        }
        // press or hold or release
        if new_input.held[0] || self.prev_input.held[0]
        {
            let point = vec_floor(&new_input.canvas_mouse_coord);
            if Some(point) != self.start_point || self.current_point.is_some()
            {
                self.current_point = Some(point);
            }
        }
        // release
        if !new_input.held[0] && self.prev_input.held[0]
        {
            if let (Some(a), Some(b)) = (self.start_point, self.current_point)
            {
                let rect = [a, b];
                let loops = Self::get_loops(rect, app);
                
                app.commit_selection(loops);
            }
            
            self.start_point = None;
            self.current_point = None;
        }
        self.prev_input = new_input.clone();
    }
    fn notify_tool_changed(&mut self, _app : &mut crate::Warpainter)
    {
        
    }
    fn is_brushlike(&self) -> bool
    {
        false
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        if let (Some(a), Some(b)) = (self.start_point, self.current_point)
        {
            let mut rect = [a, b];
            rect = rect_translate(rect, [app.canvas_width as f32 / -2.0, app.canvas_height as f32 / -2.0]);
            
            let loops = Self::get_loops(rect, app);
            
            let gizmo = OutlineGizmo { loops, filled : false };
            Some(Box::new(gizmo))
        }
        else
        {
            None
        }
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>
    {
        Some((app.icons.get("tool select cursor").as_ref().unwrap(), [6.0, 14.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, _ui : &mut Ui)
    {
    }
}
pub (crate) struct MoveTool
{
    base_image : Option<Image<4>>,
    move_image : Option<Image<4>>,
    offset : [f32; 2],
    prev_input : CanvasInputState,
}

impl MoveTool
{
    pub (crate) fn new() -> Self
    {
        MoveTool {
            base_image : None,
            move_image : None,
            offset : [0.0, 0.0],
            prev_input : CanvasInputState::default(),
        }
    }
}
impl Tool for MoveTool
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        if new_input.held[0] && !self.prev_input.held[0]
        {
            self.prev_input.canvas_mouse_coord = new_input.canvas_mouse_coord;
            app.begin_state_edit();
        }
        // press or hold
        if new_input.held[0]
        {
            let prev_point = vec_floor(&self.prev_input.canvas_mouse_coord);
            let point = vec_floor(&new_input.canvas_mouse_coord);
            
            // structured like this for future expansion
            #[allow(clippy::collapsible_if)]
            if point != prev_point
            {
                let diff = vec_sub(&point, &prev_point);
                
                if app.selection_mask.is_none()
                {
                    if let Some(base) = app.layers.find_layer_unlocked_mut(app.current_layer)
                    {
                        base.dirtify_all();
                        base.offset[0] += diff[0];
                        base.offset[1] += diff[1];
                        app.full_rerender(); // FIXME
                    }
                }
                else 
                {
                    if !app.is_editing()
                    {
                        app.begin_edit(true, true);
                        if let Some(edit_image) = &app.editing_image
                        {
                            let get_alpha : Box<dyn Fn(usize, usize) -> f32 + Sync + Send> = if let Some(mask) = &app.selection_mask
                            {
                                Box::new(|x, y| mask.get_pixel_float(x as isize, y as isize)[0])
                            }
                            else
                            {
                                Box::new(|_x, _y| 1.0)
                            };
                            
                            let mut base_image = edit_image.clone();
                            let mut move_image = edit_image.clone();
                            
                            move_image.loop_rect_threaded(
                                [[0.0, 0.0], [move_image.width as f32, move_image.height as f32]],
                                &|x, y, mut color : [f32; 4]|
                                {
                                    color[3] *= get_alpha(x, y);
                                    color
                                }
                            );
                            
                            base_image.loop_rect_threaded(
                                [[0.0, 0.0], [base_image.width as f32, base_image.height as f32]],
                                &|x, y, mut color : [f32; 4]|
                                {
                                    color[3] *= 1.0 - get_alpha(x, y);
                                    color
                                }
                            );
                            
                            self.base_image = Some(base_image);
                            self.move_image = Some(move_image);
                        }
                    }
                    let canvas_size = [app.canvas_width as f32, app.canvas_height as f32];
                    
                    let mut min = canvas_size;
                    let mut max = [0.0f32, 0.0f32];
                    
                    for points in app.selection_poly.iter_mut()
                    {
                        for point in points.iter_mut()
                        {
                            min[0] = min[0].min(point[0]);
                            min[1] = min[1].min(point[1]);
                            max[0] = max[0].max(point[0]);
                            max[1] = max[1].max(point[1]);
                            *point = vec_add(point, &diff);
                            min[0] = min[0].min(point[0]);
                            min[1] = min[1].min(point[1]);
                            max[0] = max[0].max(point[0]);
                            max[1] = max[1].max(point[1]);
                        }
                    }
                    
                    self.offset = vec_add(&self.offset, &diff);
                    if let (Some(base_image), Some(move_image), Some(editing_image))
                        = (&mut self.base_image.as_mut(), &mut self.move_image.as_mut(), app.get_editing_image())
                    {
                        let offset = [self.offset[0] as isize, self.offset[1] as isize];
                        *editing_image = base_image.clone();
                        editing_image.blend_rect_from([[0.0, 0.0], canvas_size], move_image, None, None, 1.0, 1.0, false, offset, "Weld");
                    }
                    
                    app.mark_current_layer_dirty(grow_box([min, max], [1.0, 1.0]));
                }
            }
        }
        if !new_input.held[0] && self.prev_input.held[0]
        {
            app.commit_edit();
        }
        if new_input.held[1] && !self.prev_input.held[1]
        {
            app.cancel_edit();
        }
        
        // TODO: mid-tool-use undo/redo state when releasing drag
        
        self.prev_input = new_input.clone();
    }
    fn notify_tool_changed(&mut self, app : &mut crate::Warpainter)
    {
        app.commit_edit();
        self.base_image = None;
        self.move_image = None;
        self.offset = [0.0, 0.0];
    }
    fn is_brushlike(&self) -> bool
    {
        false
    }
    fn get_gizmo(&self, _app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        None
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>
    {
        Some((app.icons.get("tool select cursor").as_ref().unwrap(), [6.0, 14.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, _ui : &mut Ui)
    {
    }
}

pub (crate) struct Eyedropper
{
    size : f32,
    sample_source : ReferenceMode,
    prev_input : CanvasInputState,
    pick_alpha : bool,
}

impl Eyedropper
{
    pub (crate) fn new() -> Self
    {
        let size = 1.0;
        Eyedropper { size, sample_source : ReferenceMode::CurrentLayer, pick_alpha : true, prev_input : CanvasInputState::default() }
    }
}

impl Tool for Eyedropper
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        if new_input.held[0]
        {
            let coord = new_input.canvas_mouse_coord;
            let coord = vec_sub(&coord, &app.get_current_offset());
            // FIXME: use size, sample source
            let image = app.get_current_layer_image();
            if let Some(image) = image
            {
                let mut color = image.get_pixel_float(coord[0] as isize, coord[1] as isize);
                if !self.pick_alpha
                {
                    color[3] = app.main_color_rgb[3];
                }
                app.set_main_color_rgb(color);
            }
        }
        
        self.prev_input = new_input.clone();
    }
    fn notify_tool_changed(&mut self, _app : &mut crate::Warpainter)
    {
        
    }
    fn is_brushlike(&self) -> bool
    {
        false
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = self.prev_input.canvas_mouse_coord;
        pos[0] = pos[0].floor() - app.canvas_width as f32 / 2.0;
        pos[1] = pos[1].floor() - app.canvas_height as f32 / 2.0;
        let gizmo = SquareGizmo { x : pos[0] + 0.5, y : pos[1] + 0.5, r : 0.5 };
        Some(Box::new(gizmo))
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a(egui::TextureHandle, Image<4>), [f32; 2])>
    {
        Some((app.icons.get("tool eyedropper").as_ref().unwrap(), [2.0, 20.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        //ui.label("Size");
        //ui.add(egui::Slider::new(&mut self.size, 1.0..=8.0).step_by(1.0).clamping());
        ui.checkbox(&mut self.pick_alpha, "Pick Alpha");
    }
}