
use eframe::egui;

pub (crate) trait Gizmo
{
    fn draw(ui : &mut egui::Ui, input : &mut egui::InputState, response : &mut egui::Response);
}

struct BoxGizmo
{
    x : f32,
    y : f32,
    w : f32,
    h : f32,
}
impl Gizmo for BoxGizmo
{
    fn draw(ui : &mut egui::Ui, input : &mut egui::InputState, response : &mut egui::Response)
    {
        
    }
}