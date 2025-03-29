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
        "lddg" => "Add",
        "lddg_glow" => "Glow Add",
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
    
    let root = Layer::new_group("PSD File");
    let mut stack = vec!(root);
    
    for (i, mut layerdata) in psd_layers.into_iter().enumerate()
    {
        let w = layerdata.w as u32;
        let h = layerdata.h as u32;
        let mut mask_img = None;
        if layerdata.mask_channel_count != 0
        {
            mask_img = image::GrayImage::from_raw(layerdata.mask_info.w, layerdata.mask_info.h, layerdata.image_data_mask);
        }
        //println!("{:?}", mask_img);
        if let Some(img) = image::RgbaImage::from_raw(w, h, layerdata.image_data_rgba)
        {
            let mask = mask_img.map(|x| Image::<1>::from_yimage(&x, layerdata.mask_info.invert));
            if layerdata.mask_info.invert { layerdata.mask_info.default_color = 255 - layerdata.mask_info.default_color; }
            layerdata.mask_info.invert = false;
            layerdata.mask_info.x -= layerdata.x as i32;
            layerdata.mask_info.y -= layerdata.y as i32;
            let img = Image::<4>::from_rgbaimage(&img);
            let mut layer = if layerdata.group_opener { Layer::new_group("New Layer") } else { Layer::new_layer_from_image("New Layer", img) };
            layer.mask_info = if mask.is_some() { Some(layerdata.mask_info) } else { None };
            layer.mask = mask;
            //println!("{:?}", layer.mask);
            layer.name = layerdata.name.to_string();
            layer.offset[0] = layerdata.x as f32;
            layer.offset[1] = layerdata.y as f32;
            layer.clipped = layerdata.is_clipped;
            layer.visible = layerdata.is_visible;
            layer.opacity = layerdata.opacity;
            layer.fill_opacity = layerdata.fill_opacity;
            //println!("!!!!{:?}", layer.offset);
            layer.blend_mode = get_blend_mode(&layerdata.blend_mode);
            
            layer.adjustment = match layerdata.adjustment_type.as_str()
            {
                "" => None,
                "nvrt" => Some(Adjustment::Invert),
                "post" => Some(Adjustment::Posterize(layerdata.adjustment_info[0])),
                "thrs" => Some(Adjustment::Threshold(layerdata.adjustment_info[0])),
                "brit" => Some(Adjustment::BrightContrast(<[f32; 5]>::try_from(&layerdata.adjustment_info[0..5]).unwrap())),
                "hue2" => Some(Adjustment::HueSatLum(<[f32; 3]>::try_from(&layerdata.adjustment_info[4..7]).unwrap())),
                "levl" => 
                {
                    let mut data = vec!();
                    let mut i = 0;
                    for _ in 0..6
                    {
                        data.push(<[f32; 5]>::try_from(&layerdata.adjustment_info[i..i+5]).unwrap());
                        i += 5;
                    }
                    Some(Adjustment::Levels(data))
                }
                "curv" =>
                {
                    let mut data = vec!();
                    let mut i = 0;
                    for _ in 0..6
                    {
                        let mut n = layerdata.adjustment_info[i];
                        i += 1;
                        let mut nodes = vec!();
                        for j in 0..n as usize
                        {
                            nodes.push([layerdata.adjustment_info[i], layerdata.adjustment_info[i+1]]);
                            i += 2;
                        }
                        data.push(nodes);
                    }
                    Some(Adjustment::Curves(data))
                }
                "blwh" =>
                {
                    let mut data = [0.0; 6];
                    let mut tintColor = false; // TODO
                    let mut data2 = [0.0; 3]; // TODO
                    
                    let mut n = HashMap::new();
                    for t in &layerdata.adjustment_desc.unwrap().1
                    {
                        n.insert(t.0.clone(), t.1.clone());
                    }
                    
                    data[0] = n.get("Rd  ").unwrap().long() as f32;
                    data[1] = n.get("Yllw").unwrap().long() as f32;
                    data[2] = n.get("Grn ").unwrap().long() as f32;
                    data[3] = n.get("Cyn ").unwrap().long() as f32;
                    data[4] = n.get("Bl  ").unwrap().long() as f32;
                    data[5] = n.get("Mgnt").unwrap().long() as f32;
                    
                    Some(Adjustment::BlackWhite((data, tintColor, data2)))
                }
                //_ => panic!(),
                _ => None,
            };
            
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
    app.current_layer = app.layers.children[0].uuid;
    app.current_tool = 4;
    //for (i, group) in psd.groups() {
    //    let name = group.name();
    //    println!("group {}: {}", i, name);
    //    for (j, n) in 
    //}
    println!("asdf");
}