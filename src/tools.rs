
use crate::warimage::*;
use crate::transform::*;
use crate::canvas::CanvasInputState;
use crate::gizmos::*;
use crate::pixelmath::*;

use crate::egui;
use crate::egui::Ui;

enum ReferenceMode
{
    CurrentLayer,
    CurrentFolder,
    Merged,
}

pub (crate) trait Tool
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState);
    fn is_brushlike(&self) -> bool; // ctrl is color picker, otherwise tool-contolled
    fn get_gizmo(&self, app : &crate::Warpainter, focused : bool) -> Option<Box<dyn Gizmo>>;
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a egui::TextureHandle, [f32; 2])>;
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
            app.begin_edit(false);
            
            let start = std::time::SystemTime::now();
            
            let prev_coord = self.prev_input.canvas_mouse_coord;
            let coord = new_input.canvas_mouse_coord;
            
            let color = app.main_color_rgb;
            if let Some(Some(base)) = app.layers.find_layer_unlocked(app.current_layer).map(|x| x.data.as_ref())
            {
                if let Some(image) = (&mut app.editing_image).as_mut()
                {
                    if !self.prev_input.held[0] || prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                    {
                        let coord = [coord[0] as isize, coord[1] as isize];
                        let ref_color = base.get_pixel_float(coord[0], coord[1]);
                        
                        fn compare_dist(a : [f32; 4], b : [f32; 4], r : f32) -> bool
                        {
                            let mut d = 0.0;
                            for i in 0..4
                            {
                                d += (b[i]-a[i]).abs();
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
                        println!("max frontier size... {}", max_f_size);
                    }
                }
                
                let elapsed = start.elapsed();
                let elapsed = match elapsed { Ok(x) => x.as_secs_f64(), Err(x) => x.duration().as_secs_f64() };
                if elapsed > 0.01
                {
                    println!("time to flood fill: {}", elapsed);
                }
            }
            
            app.commit_edit();
        }
        
        self.prev_input = new_input.clone();
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, _app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        None
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a egui::TextureHandle, [f32; 2])>
    {
        Some((app.icons.get("tool fill").as_ref().unwrap(), [2.0, 18.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        ui.label("Treshold");
        let mut threshold = self.threshold * 255.0;
        ui.add(egui::Slider::new(&mut threshold, 0.0..=100.0).clamp_to_range(true));
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
        let x = coord[0].round() as isize;
        let y = coord[1].round() as isize;
        image.set_pixel_float(x, y, color);
    }
}
fn draw_line_no_start(image : &mut Image<4>, from : [f32; 2], to : [f32; 2], color : [u8; 4])
{
    draw_line_no_start_float(image, from, to, px_to_float(color))
}

fn draw_brush_line_no_start_float(image : &mut Image<4>, mut from : [f32; 2], mut to : [f32; 2], color : [f32; 4], brush : &Vec<Vec<((isize, isize), [f32; 4])>>, offset : [isize; 2], erase : bool, alpha_lock : bool)
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
            _ => panic!(),
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
        let x = coord[0].round() as isize;
        let y = coord[1].round() as isize;
        let dir = dir_index(x - prev_x, y - prev_y);
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
        prev_x = x;
        prev_y = y;
    }
}
fn draw_brush_line_no_start(image : &mut Image<4>, from : [f32; 2], to : [f32; 2], color : [u8; 4], brush : &Vec<Vec<((isize, isize), [f32; 4])>>, offset : [isize; 2], erase : bool, alpha_lock : bool)
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

fn grow_box(mut rect : [[f32; 2]; 2], grow_size : [f32; 2]) -> [[f32; 2]; 2]
{
    rect = rect_normalize(rect);
    rect[0][0] -= grow_size[0];
    rect[0][1] -= grow_size[1];
    rect[1][0] += grow_size[0];
    rect[1][1] += grow_size[1];
    rect
}

fn generate_brush(size : f32) -> Image<4>
{
    let img_size = size.ceil() as usize;
    let mut shape = Image::blank(img_size, img_size);
    for uy in 0..img_size as isize
    {
        let y = uy as f32 - (img_size as f32)*0.5 + 0.5;
        for ux in 0..img_size as isize
        {
            let x = ux as f32 - (img_size as f32)*0.5 + 0.5;
            if y*y + x*x < size*size/4.0 && (x != 0.0 || img_size == 1) // <- for testing outline analysis
            {
                shape.set_pixel(ux, uy, [255, 255, 255, 255]);
            }
        }
    }
    shape
}
fn directionalize_brush(brush_shape : &Image<4>) -> Vec<Vec<((isize, isize), [f32; 4])>>
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
pub (crate) struct Pencil
{
    size : f32,
    brush_shape : Image<4>,
    outline_data : Vec<Vec<[f32; 2]>>,
    direction_shapes : Vec<Vec<((isize, isize), [f32; 4])>>,
    prev_input : CanvasInputState,
    cursor_memory : [f32; 2],
    smooth_mode : bool,
    is_eraser : bool,
}

impl Pencil
{
    pub (crate) fn new() -> Self
    {
        let size = 1.0;
        let brush_shape = generate_brush(size);
        let outline_data = brush_shape.analyze_outline();
        let direction_shapes = directionalize_brush(&brush_shape);
        Pencil {
            size,
            brush_shape,
            outline_data,
            direction_shapes,
            prev_input : CanvasInputState::default(),
            cursor_memory : [0.0, 0.0],
            smooth_mode : false,
            is_eraser : false,
        }
    }
    pub (crate) fn to_eraser(mut self) -> Self
    {
        self.is_eraser = true;
        self
    }
    pub (crate) fn update_brush(&mut self)
    {
        self.brush_shape = generate_brush(self.size);
        self.outline_data = self.brush_shape.analyze_outline();
        self.direction_shapes = directionalize_brush(&self.brush_shape);
    }
}

impl Tool for Pencil
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
            app.begin_edit(true);
            self.cursor_memory = vec_floor(&new_input.canvas_mouse_coord);
        }
        // press or hold or release
        if new_input.held[0] || self.prev_input.held[0]
        {
            let do_smooth = new_input.held[0] && self.smooth_mode;
            let prev_coord = if self.smooth_mode { self.cursor_memory } else { vec_floor(&self.prev_input.canvas_mouse_coord) };
            let mut coord = vec_floor(&new_input.canvas_mouse_coord);
            
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
            
            let color = app.main_color_rgb;
            let eraser = app.eraser_mode || self.is_eraser;
            let alpha_locked = app.current_layer_is_alpha_locked();
            if let Some(image) = app.get_editing_image()
            {
                let size_vec = [self.brush_shape.width as f32, self.brush_shape.height as f32];
                let offset_vec = [(self.brush_shape.width/2) as isize, (self.brush_shape.height/2) as isize];
                if !self.prev_input.held[0]
                {
                    draw_brush_at_float(image, coord, color, &self.brush_shape, eraser, alpha_locked);
                    app.mark_current_layer_dirty(grow_box([coord, coord], size_vec));
                }
                else if prev_coord[0] != coord[0] || prev_coord[1] != coord[1]
                {
                    draw_brush_line_no_start_float(image, prev_coord, coord, color, &self.direction_shapes, offset_vec, eraser, alpha_locked);
                    app.mark_current_layer_dirty(grow_box([prev_coord, coord], size_vec));
                }
            }
            
            self.cursor_memory = coord;
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
        
        self.prev_input = new_input.clone();
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = self.cursor_memory;
        pos[0] = pos[0] - app.canvas_width as f32 / 2.0;
        pos[1] = pos[1] - app.canvas_height as f32 / 2.0;
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
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a egui::TextureHandle, [f32; 2])>
    {
        Some((app.icons.get("tool pencil").as_ref().unwrap(), [2.0, 19.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        ui.label("Size");
        let old_size = self.size;
        ui.add(egui::Slider::new(&mut self.size, 1.0..=64.0).step_by(1.0).logarithmic(true).clamp_to_range(true));
        if self.size != old_size
        {
            self.update_brush();
        }
        ui.checkbox(&mut self.smooth_mode, "Smooth Diagonals");
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
}
impl Tool for Selection
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        // press
        if new_input.held[0] && !self.prev_input.held[0]
        {
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
            let mut loops = Vec::new();
            if let (Some(a), Some(b)) = (self.start_point, self.current_point)
            {
                let mut rect = rect_normalize([a, b]);
                rect[1][0] += 1.0;
                rect[1][1] += 1.0;
                rect = rect_translate(rect, [app.canvas_width as f32 / -2.0, app.canvas_height as f32 / -2.0]);
                loops = vec!(vec!(
                    rect[0],
                    [rect[1][0], rect[0][1]],
                    rect[1],
                    [rect[0][0], rect[1][1]],
                    rect[0],
                ));
                
                //app.commit_selection(loops);
            }
            
            self.start_point = None;
            self.current_point = None;
        }
        self.prev_input = new_input.clone();
    }
    fn is_brushlike(&self) -> bool
    {
        false
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        if let (Some(a), Some(b)) = (self.start_point, self.current_point)
        {
            let mut rect = rect_normalize([a, b]);
            rect[1][0] += 1.0;
            rect[1][1] += 1.0;
            rect = rect_translate(rect, [app.canvas_width as f32 / -2.0, app.canvas_height as f32 / -2.0]);
            let loops = vec!(vec!(
                rect[0],
                [rect[1][0], rect[0][1]],
                rect[1],
                [rect[0][0], rect[1][1]],
                rect[0],
            ));
            let gizmo = OutlineGizmo { loops, filled : false };
            Some(Box::new(gizmo))
        }
        else
        {
            None
        }
    }
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a egui::TextureHandle, [f32; 2])>
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
    fn get_cursor<'a>(&self, app : &'a crate::Warpainter) -> Option<(&'a egui::TextureHandle, [f32; 2])>
    {
        Some((app.icons.get("tool eyedropper").as_ref().unwrap(), [2.0, 20.0]))
    }
    fn settings_panel(&mut self, _app : &crate::Warpainter, ui : &mut Ui)
    {
        //ui.label("Size");
        //ui.add(egui::Slider::new(&mut self.size, 1.0..=8.0).step_by(1.0).clamp_to_range(true));
        ui.checkbox(&mut self.pick_alpha, "Pick Alpha");
    }
}