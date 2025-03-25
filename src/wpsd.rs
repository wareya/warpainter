use std::collections::HashMap;

use crate::*;

pub (crate) fn get_blend_mode(mode : &str) -> String
{
    match mode
    {
        "pass" => "Normal",
        "norm" => "Normal",
        "diss" => "Dither",
        "dark" => "Darken",
        "mul " => "Multiply",
        "idiv" => "Color Burn",
        "lbrn" => "Linear Burn",
        "dkCl" => "Darken",
        "lite" => "Lighten",
        "scrn" => "Screen",
        "div " => "Color Dodge",
        "lddg" => "Glow Dodge",
        "lgCl" => "Lighten",
        "over" => "Overlay",
        "sLit" => "SoftLight",
        "hLit" => "Hard Light",
        "vLit" => "Vivid Light",
        "lLit" => "Linear Light",
        "pLit" => "Pin Light",
        "hMix" => "Hard Mix",
        "diff" => "Difference",
        "smud" => "Exclusion",
        "fsub" => "Subtract",
        "fdiv" => "Divide",
        "hue " => "Hue",
        "sat " => "Saturation",
        "colr" => "Color",
        "lum " => "Luminosity",
        _ => "Normal",
    }.to_string()
}

use crate::wpsd_raw::*;

pub (crate) fn wpsd_open(app : &mut Warpainter, bytes : &[u8])
{
    let psd_data = parse_psd_metadata(&bytes);
    let psd_layers = parse_layer_records(&bytes);
    
    app.layers = Layer::new_group("___root___");
    app.canvas_width = psd_data.width as usize;
    app.canvas_height = psd_data.height as usize;
    
    let mut root = Layer::new_group("PSD File");
    let mut stack = vec!(root);
    
    for (i, layerdata) in psd_layers.into_iter().enumerate()
    {
        let w = layerdata.w as u32;
        let h = layerdata.h as u32;
        if let Some(img) = image::RgbaImage::from_raw(w, h, layerdata.image_data_rgba)
        {
            let img = Image::<4>::from_rgbaimage(&img);
            let mut layer = if layerdata.group_opener { Layer::new_group("New Layer") } else { Layer::new_layer_from_image("New Layer", img) };
            layer.name = layerdata.name.to_string();
            layer.offset[0] = layerdata.x as f32;
            layer.offset[1] = layerdata.y as f32;
            layer.clipped = layerdata.is_clipped;
            layer.visible = layerdata.is_visible;
            layer.opacity = layerdata.opacity;
            //println!("!!!!{:?}", layer.offset);
            layer.blend_mode = get_blend_mode(&layerdata.blend_mode);
            //println!("layer {}: {} (of {:?})", i, layer.name, layer.parent_id());
            println!("layer {}: {}", i, layer.name);
            
            if layerdata.group_closer
            {
                stack.push(layer);
            }
            else if layerdata.group_opener
            {
                let mut temp = stack.pop().unwrap();
                std::mem::swap(&mut temp.children, &mut layer.children);
                stack.last_mut().unwrap().children.insert(0, layer);
            }
            else
            {
                stack.last_mut().unwrap().children.insert(0, layer);
            }
        }
    }
    assert!(stack.len() == 1);
    app.layers.children = vec!(stack.pop().unwrap());
    //for (i, group) in psd.groups() {
    //    let name = group.name();
    //    println!("group {}: {}", i, name);
    //    for (j, n) in 
    //}
    println!("asdf");
}