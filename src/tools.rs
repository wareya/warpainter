
use crate::warimage::*;
use crate::transform::*;
use crate::canvas::CanvasInputState;
use crate::gizmos::*;

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
    prev_input : CanvasInputState,
}

impl Pencil
{
    pub (crate) fn new() -> Self
    {
        Pencil { size : 1.0, prev_input : CanvasInputState::default() }
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
        image.set_pixel_float(coord[0].round() as isize, coord[1].round() as isize, color);
    }
}
fn draw_line_no_start(image : &mut Image, from : [f32; 2], to : [f32; 2], color : [u8; 4])
{
    draw_line_no_start_float(image, from, to, px_to_float(color))
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
            if let Some(image) = app.get_editing_image()
            {
                if !self.prev_input.held[0]
                {
                    image.set_pixel_float(coord[0] as isize, coord[1] as isize, color);
                    app.mark_current_layer_dirty([coord, coord]);
                }
                else if prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                {
                    draw_line_no_start_float(image, prev_coord, coord, color);
                    app.mark_current_layer_dirty([prev_coord, coord]);
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