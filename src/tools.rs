
use crate::warimage::*;
use crate::transform::*;
use crate::canvas::CanvasInputState;
use crate::gizmos::*;
use crate::pixelmath::*;

pub (crate) trait Tool
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState);
    fn edits_inplace(&self) -> bool; // whether the layer gets a full layer copy or a blank layer that gets composited on top
    fn is_brushlike(&self) -> bool; // ctrl is color picker, otherwise tool-contolled
    fn get_gizmo(&self, app : &crate::Warpainter, focused : bool) -> Option<Box<dyn Gizmo>>;
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
            app.begin_edit(self.edits_inplace());
        }
        if new_input.held[0]
        {
            let prev_coord = self.prev_input.canvas_mouse_coord;
            let coord = new_input.canvas_mouse_coord;
            
            app.debug(format!("{:?}", coord));
            let color = app.main_color_rgb;
            if let Some(Some(base)) = app.layers.find_layer_unlocked(app.current_layer).map(|x| x.data.as_ref())
            {
                if let Some(image) = (&mut app.editing_image).as_mut()
                {
                    if !self.prev_input.held[0] || prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                    {
                        let ref_color = base.get_pixel_float(coord[0] as isize, coord[1] as isize);
                        let get_dist = |a, r| length(&vec_sub(&r, &a));
                        
                        // clear draw buffer with a color that the tool isn't using
                        let mut dum_color = [0.0, 0.0, 0.0, 0.0];
                        if get_dist(dum_color, color) < 0.5
                        {
                            dum_color = [1.0, 1.0, 1.0, 0.0];
                        }
                        image.clear_with_color_float(dum_color);
                        
                        let mut frontier = vec!();
                        let mut process_coord = |coord : [f32; 2], frontier : &mut Vec<_>|
                        {
                            image.set_pixel_float(coord[0] as isize, coord[1] as isize, color);
                            for add in [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]]
                            {
                                let coord = vec_add(&coord, &add);
                                if coord[0] < 0.0 || coord[0] >= app.canvas_width as f32
                                || coord[1] < 0.0 || coord[1] >= app.canvas_height as f32
                                {
                                    continue;
                                }
                                let cond1 = get_dist(image.get_pixel_float(coord[0] as isize, coord[1] as isize), dum_color) < 0.001;
                                let cond2 = get_dist(base .get_pixel_float(coord[0] as isize, coord[1] as isize), ref_color) < self.threshold;
                                if cond1 && cond2
                                {
                                    frontier.push(coord);
                                }
                            }
                        };
                        
                        process_coord(coord, &mut frontier);
                        while let Some(coord) = frontier.pop()
                        {
                            process_coord(coord, &mut frontier);
                        }
                    }
                }
            }
        }
        else if self.prev_input.held[0]
        {
            app.commit_edit();
        }
        if new_input.held[1] && !self.prev_input.held[1]
        {
            app.cancel_edit();
        }
        
        self.prev_input = new_input.clone();
    }
    fn edits_inplace(&self) -> bool
    {
        false
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, _app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        None
    }
}

pub (crate) struct Pencil
{
    size : f32,
    brush_shape : Image,
    direction_shapes : Vec<Vec<((isize, isize), [f32; 4])>>,
    prev_input : CanvasInputState,
    is_eraser : bool,
}

impl Pencil
{
    pub (crate) fn new() -> Self
    {
        let size = 4.0;
        let brush_shape = Pencil::generate_brush(size);
        let direction_shapes = Pencil::directionalize_brush(&brush_shape);
        Pencil { size, brush_shape, direction_shapes, prev_input : CanvasInputState::default(), is_eraser : false }
    }
    pub (crate) fn to_eraser(mut self) -> Self
    {
        self.is_eraser = true;
        self
    }
    pub (crate) fn update_brush(&mut self)
    {
        self.brush_shape = Pencil::generate_brush(self.size);
        self.direction_shapes = Pencil::directionalize_brush(&self.brush_shape);
    }
    fn generate_brush(size : f32) -> Image
    {
        let img_size = size.ceil() as usize;
        let mut shape = Image::blank(img_size, img_size);
        for uy in 0..img_size as isize
        {
            let y = uy as f32 - (img_size as f32)*0.5 + 0.5;
            for ux in 0..img_size as isize
            {
                let x = ux as f32 - (img_size as f32)*0.5 + 0.5;
                if y*y + x*x < size*size/4.0
                {
                    shape.set_pixel(ux, uy, [255, 255, 255, 255]);
                }
            }
        }
        shape
    }
    fn directionalize_brush(brush_shape : &Image) -> Vec<Vec<((isize, isize), [f32; 4])>>
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
}

fn draw_line_no_start_float(image : &mut Image, mut from : [f32; 2], mut to : [f32; 2], color : [f32; 4])
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
fn draw_line_no_start(image : &mut Image, from : [f32; 2], to : [f32; 2], color : [u8; 4])
{
    draw_line_no_start_float(image, from, to, px_to_float(color))
}

fn draw_brush_line_no_start_float(image : &mut Image, mut from : [f32; 2], mut to : [f32; 2], color : [f32; 4], brush : &Vec<Vec<((isize, isize), [f32; 4])>>, offset : [isize; 2], erase : bool)
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
            let mut c = *c;
            if !erase
            {
                if c[3] > 0.0
                {
                    c[0] *= color[0];
                    c[1] *= color[1];
                    c[2] *= color[2];
                    c[3] *= color[3];
                    image.set_pixel_float(x + ux - offset[0], y + uy - offset[1], color);
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
fn draw_brush_line_no_start(image : &mut Image, from : [f32; 2], to : [f32; 2], color : [u8; 4], brush : &Vec<Vec<((isize, isize), [f32; 4])>>, offset : [isize; 2], erase : bool)
{
    draw_brush_line_no_start_float(image, from, to, px_to_float(color), brush, offset, erase)
}
fn draw_brush_at_float(image : &mut Image, at : [f32; 2], color : [f32; 4], brush_shape : &Image, erase : bool)
{
    let x = at[0].floor() as isize;
    let y = at[1].floor() as isize;
    for uy in 0..brush_shape.height as isize
    {
        for ux in 0..brush_shape.width as isize
        {
            let mut c = brush_shape.get_pixel_float(ux, uy);
            if c[3] > 0.0
            {
                if !erase
                {
                    c[0] *= color[0];
                    c[1] *= color[1];
                    c[2] *= color[2];
                    c[3] *= color[3];
                    image.set_pixel_float(x + ux - (brush_shape.width/2) as isize, y + uy - (brush_shape.height/2) as isize, color);
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
fn draw_brush_at(image : &mut Image, at : [f32; 2], color : [u8; 4], brush_shape : &Image, erase : bool)
{
    draw_brush_at_float(image, at, px_to_float(color), brush_shape, erase)
}


fn grow_box(mut rect : [[f32; 2]; 2], grow_size : [f32; 2]) -> [[f32; 2]; 2]
{
    use crate::rect_normalize;
    rect = rect_normalize(rect);
    rect[0][0] -= grow_size[0];
    rect[0][1] -= grow_size[1];
    rect[1][0] += grow_size[0];
    rect[1][1] += grow_size[1];
    rect
}

impl Tool for Pencil
{
    fn think(&mut self, app : &mut crate::Warpainter, new_input : &CanvasInputState)
    {
        if new_input.held[0] && !self.prev_input.held[0]
        {
            app.begin_edit(self.edits_inplace());
        }
        if new_input.held[0]
        {
            let prev_coord = self.prev_input.canvas_mouse_coord;
            let coord = new_input.canvas_mouse_coord;
            
            app.debug(format!("{:?}", coord));
            let color = app.main_color_rgb;
            let eraser = app.eraser_mode || self.is_eraser;
            if let Some(image) = app.get_editing_image()
            {
                let size_vec = [self.brush_shape.width as f32, self.brush_shape.height as f32];
                let offset_vec = [(self.brush_shape.width/2) as isize, (self.brush_shape.height/2) as isize];
                if !self.prev_input.held[0]
                {
                    draw_brush_at_float(image, coord, color, &self.brush_shape, eraser);
                    app.mark_current_layer_dirty(grow_box([coord, coord], size_vec));
                }
                else if prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                {
                    draw_brush_line_no_start_float(image, prev_coord, coord, color, &self.direction_shapes, offset_vec, eraser);
                    app.mark_current_layer_dirty(grow_box([prev_coord, coord], size_vec));
                }
            }
        }
        else if self.prev_input.held[0]
        {
            app.commit_edit();
        }
        if new_input.held[1] && !self.prev_input.held[1]
        {
            app.cancel_edit();
        }
        
        self.prev_input = new_input.clone();
    }
    fn edits_inplace(&self) -> bool
    {
        true
    }
    fn is_brushlike(&self) -> bool
    {
        true
    }
    fn get_gizmo(&self, app : &crate::Warpainter, _focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = self.prev_input.canvas_mouse_coord;
        pos[0] = pos[0].floor() - app.canvas_width as f32 / 2.0;
        pos[1] = pos[1].floor() - app.canvas_height as f32 / 2.0;
        let gizmo = BrushGizmo { x : pos[0] + 0.5, y : pos[1] + 0.5, r : 0.5 };
        Some(Box::new(gizmo))
    }
}