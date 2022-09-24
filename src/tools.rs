
use crate::warimage::*;
use crate::transform::*;
use crate::canvas::CanvasInputState;
use crate::gizmos::*;

pub (crate) trait Tool
{
    fn think(&mut self, app : &mut crate::Warpaint, new_input : &CanvasInputState);
    fn edits_inplace(&self) -> bool; // whether the layer gets a full layer copy or a blank layer that gets composited on top
    fn is_brushlike(&self) -> bool; // ctrl is color picker, otherwise tool-contolled
    fn get_gizmo(&self, app : &crate::Warpaint, focused : bool) -> Option<Box<dyn Gizmo>>;
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
    fn think(&mut self, app : &mut crate::Warpaint, new_input : &CanvasInputState)
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
            if let Some(mut image) = app.get_editing_image()
            {
                if !self.prev_input.held[0]
                {
                    image.set_pixel_float(coord[0] as isize, coord[1] as isize, color);
                }
                else if prev_coord[0].floor() != coord[0].floor() || prev_coord[1].floor() != coord[1].floor()
                {
                    draw_line_no_start_float(image, prev_coord, coord, color);
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
    fn get_gizmo(&self, app : &crate::Warpaint, focused : bool) -> Option<Box<dyn Gizmo>>
    {
        let mut pos = self.prev_input.canvas_mouse_coord;
        pos[0] -= app.canvas_width as f32 / 2.0;
        pos[1] -= app.canvas_height as f32 / 2.0;
        let mut gizmo = BrushGizmo { x : pos[0].floor() + 0.5, y : pos[1].floor() + 0.5, r : 0.5 };
        Some(Box::new(gizmo))
    }
}