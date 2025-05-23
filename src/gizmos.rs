
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
            painter.line_segment([start.into(), end.into()], white);
        }
        else
        {
            painter.line_segment([start.into(), end.into()], black);
            
            //// counteract bad linear-color-space AA by drawing twice
            //painter.line_segment([start.into(), end.into()].into(), black);
            // not needed in egui 0.21.0
        }
    }
}

pub (crate) fn draw_doubled(painter : &egui::Painter, point_lists : &[&[[f32; 2]]])
{
    let white = egui::Stroke::new(3.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
    let black = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255));
    
    // white section for outline
    for points in point_lists.iter()
    {
        for i in 0..points.len()-1
        {
            painter.line_segment([points[i].into(), points[i+1].into()], white);
        }
    }
    // black section for inner line
    for points in point_lists.iter()
    {
        for i in 0..points.len()-1
        {
            painter.line_segment([points[i].into(), points[i+1].into()], black);
            
            //// counteract bad linear-color-space AA by drawing twice
            //painter.line_segment([pair[0].into(), pair[1].into()].into(), black);
            // not needed in egui 0.21.0
        }
    }
}


pub (crate) fn draw_doubled_smaller(painter : &egui::Painter, point_lists : &[&[[f32; 2]]])
{
    let white = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255));
    let black = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 255));
    
    // white section for outline
    for points in point_lists.iter()
    {
        for i in 0..points.len()-1
        {
            let v1 = [points[i][0], points[i][1]];
            let v2 = [points[i+1][0], points[i+1][1]];
            let v3 = vec_sub(&v1, &v2);
            let v4 = [v3[1], -v3[0]];
            let v5 = vec_normalize(&v4);
            let p = [vec_add(&points[i], &v5).into(), vec_add(&points[i+1], &v5).into()];
            painter.line_segment(p, white);
        }
    }
    // black section for inner line
    for points in point_lists.iter()
    {
        for i in 0..points.len()-1
        {
            painter.line_segment([points[i].into(), points[i+1].into()], black);
            
            //// counteract bad linear-color-space AA by drawing twice
            //painter.line_segment([pair[0].into(), pair[1].into()].into(), black);
            // not needed in egui 0.21.0
        }
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

pub (crate) struct SquareGizmo
{
    pub (crate) x : f32,
    pub (crate) y : f32,
    pub (crate) r : f32,
}

impl Gizmo for SquareGizmo
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
        
        draw_doubled(painter, &[&[points[0], points[1], points[3], points[2], points[0]]])
    }
}

pub (crate) struct OutlineGizmo
{
    pub (crate) loops : Vec<Vec<[f32; 2]>>, // vec of loops, each loop is a closed list of points
    pub (crate) filled : bool, // whether the outline should be drawn "filled"
}

impl Gizmo for OutlineGizmo
{
    fn draw(&mut self, _ui : &mut egui::Ui, app : &mut crate::Warpainter, response : &mut egui::Response, painter : &egui::Painter)
    {
        let mut xform = app.xform.clone();
        let center = response.rect.center();
        xform.translate([center.x, center.y]);
        
        for mut points in self.loops.clone()
        {
            for point in points.iter_mut()
            {
                *point = &xform * &*point;
            }
            
            points.push(*points.last().unwrap());
            
            draw_doubled(painter, &[&points]);
        }
    }
}