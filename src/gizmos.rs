
use eframe::egui;
use crate::transform::*;

pub (crate) trait Gizmo
{
    fn draw(&mut self, ui : &mut egui::Ui, app : &mut crate::Warpainter, response : &mut egui::Response, painter : &egui::Painter);
}

pub (crate) fn draw_dotted(painter : &egui::Painter, from : [f32; 2], to : [f32; 2], dot_length : f32)
{
    let white = egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
    let black = egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255));
    
    let len = length(&vec_sub(&from, &to));
    if len == 0.0
    {
        return;
    }
    
    for i in 0..(len/dot_length).ceil() as usize
    {
        let i_start = ((i as f32    )/(len/dot_length)).min(1.0);
        let i_end   = ((i as f32+1.0)/(len/dot_length)).min(1.0);
        let mut start = vec_lerp(&from, &to, i_start);
        let mut end   = vec_lerp(&from, &to, i_end);
        start[0] = start[0].floor() + 0.5;
        start[1] = start[1].floor() + 0.5;
        end[0] = end[0].floor() + 0.5;
        end[1] = end[1].floor() + 0.5;
        if i % 2 == 0
        {
            painter.line_segment([start.into(), end.into()].into(), white);
        }
        else
        {
            painter.line_segment([start.into(), end.into()].into(), black);
            
            //// counteract bad linear-color-space AA by drawing twice
            //painter.line_segment([start.into(), end.into()].into(), black);
            // not needed in egui 0.21.0
        }
    }
}

pub (crate) fn draw_doubled(painter : &egui::Painter, points : &[[[f32; 2]; 2]])
{
    let white = egui::Stroke::new(3.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
    let black = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255));
    
    // white section for outline
    for pair in points.iter()
    {
        painter.line_segment([pair[0].into(), pair[1].into()].into(), white);
    }
    // black section for inner line
    for pair in points.iter()
    {
        painter.line_segment([pair[0].into(), pair[1].into()].into(), black);
        
        //// counteract bad linear-color-space AA by drawing twice
        //painter.line_segment([pair[0].into(), pair[1].into()].into(), black);
        // not needed in egui 0.21.0
    }
}

pub (crate) struct BoxGizmo
{
    pub (crate) x : f32,
    pub (crate) y : f32,
    pub (crate) w : f32,
    pub (crate) h : f32,
}

impl Gizmo for BoxGizmo
{
    fn draw(&mut self, _ui : &mut egui::Ui, app : &mut crate::Warpainter, response : &mut egui::Response, painter : &egui::Painter)
    {
        let x = self.x;
        let y = self.y;
        let w = self.w;
        let h = self.h;
        let mut points = [
            [x, y  ], [x+w, y  ],
            [x, y+h], [x+w, y+h],
        ];
        
        let mut xform = app.xform.clone();
        let center = response.rect.center();
        xform.translate([center.x, center.y]);
        for point in points.iter_mut()
        {
            *point = &xform * &*point;
        }
        
        draw_dotted(painter, points[0], points[1], 4.0);
        draw_dotted(painter, points[0], points[2], 4.0);
        draw_dotted(painter, points[1], points[3], 4.0);
        draw_dotted(painter, points[2], points[3], 4.0);
    }
}

pub (crate) struct BrushGizmo
{
    pub (crate) x : f32,
    pub (crate) y : f32,
    pub (crate) r : f32,
}

impl Gizmo for BrushGizmo
{
    fn draw(&mut self, _ui : &mut egui::Ui, app : &mut crate::Warpainter, response : &mut egui::Response, painter : &egui::Painter)
    {
        let x = self.x;
        let y = self.y;
        let r = self.r;
        let mut points = [
            [x-r, y-r], [x+r, y-r],
            [x-r, y+r], [x+r, y+r],
        ];
        
        let mut xform = app.xform.clone();
        let center = response.rect.center();
        xform.translate([center.x, center.y]);
        for point in points.iter_mut()
        {
            *point = &xform * &*point;
        }
        
        draw_doubled(painter, &[
            [points[0], points[1]],
            [points[0], points[2]],
            [points[1], points[3]],
            [points[2], points[3]],
        ]);
    }
}