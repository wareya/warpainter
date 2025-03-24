use std::collections::HashMap;

use psd::{ColorMode, Psd, PsdChannelCompression};

use crate::*;

fn packbits_decompress(input : &[u8]) -> Vec<u8>
{
    let mut output = Vec::new();
    let mut i = 0;
    while i < input.len()
    {
        let byte = input[i];
        i += 1;
        if byte <= 127
        {
            let len = byte as usize + 1;
            if i + len > input.len()
            {
                break;
            }
            output.extend_from_slice(&input[i..i + len]);
            i += len;
        }
        else if byte != 128
        {
            let len = 257usize.wrapping_sub(byte as usize);
            if i >= input.len()
            {
                break;
            }
            output.extend(std::iter::repeat(input[i]).take(len));
            i += 1;
        }
    }
    output
}

pub (crate) fn wpsd_open(app : &mut Warpainter, bytes : &[u8])
{
    let psd = Psd::from_bytes(&bytes).unwrap();
    
    app.layers = Layer::new_group("___root___");
    app.canvas_width = psd.width() as usize;
    app.canvas_height = psd.height() as usize;
    
    let mut group = Layer::new_group("PSD File");
    let group_uuid = group.uuid;
    
    let mut layers = HashMap::<usize, Layer>::new();
    
    for (i, layer) in psd.layers().iter().enumerate() {
        let pxbytes : Vec<u8> = layer.rgba();
        
        use psd::IntoRgba;
        use psd::PsdChannelKind;
        
        let w = layer.width() as u32;
        let h = layer.height() as u32;
        let mut rgba = vec![255; (w*h*4) as usize];
        
        use psd::ChannelBytes;
        let insert = |rgba : &mut [u8], offs : usize, bytes : & ChannelBytes|
        {
            let bytes = match bytes {
                ChannelBytes::RawData(channel_bytes) => channel_bytes.to_vec(),
                ChannelBytes::RleCompressed(channel_bytes) => packbits_decompress(&channel_bytes),
            };
            for i in 0..bytes.len()
            {
                rgba[i as usize*4 + offs] = bytes[i as usize];
            }
        };
        let r = layer.get_channel(PsdChannelKind::Red).unwrap();
        let g = layer.get_channel(PsdChannelKind::Green).unwrap_or(r);
        let b = layer.get_channel(PsdChannelKind::Blue).unwrap_or(r);
        insert(&mut rgba, 0, r);
        insert(&mut rgba, 1, g);
        insert(&mut rgba, 2, b);
        if let Some(a) = layer.get_channel(PsdChannelKind::TransparencyMask)
        {
            insert(&mut rgba, 3, a);
        }
        
        //println!("cough: {} {} {}", rgba.len(), layer.width(), layer.height());
        
        if let Some(img) = image::RgbaImage::from_raw(w, h, rgba)
        {
            let img = Image::<4>::from_rgbaimage(&img);
            let mut image_layer = Layer::new_layer_from_image("New Layer", img);
            image_layer.name = layer.name().to_string();
            image_layer.offset[0] = layer.layer_left() as f32;
            image_layer.offset[1] = layer.layer_top() as f32;
            image_layer.clipped = !layer.is_clipping_mask();
            image_layer.visible = layer.visible();
            //println!("!!!!{:?}", image_layer.offset);
            use psd::*;
            image_layer.blend_mode = match layer.blend_mode()
            {
                BlendMode::PassThrough   => "Normal",
                BlendMode::Normal        => "Normal",
                BlendMode::Dissolve      => "Dither",
                BlendMode::Darken        => "Darken",
                BlendMode::Multiply      => "Multiply",
                BlendMode::ColorBurn     => "Color Burn",
                BlendMode::LinearBurn    => "Linear Burn",
                BlendMode::DarkerColor   => "Darken",
                BlendMode::Lighten       => "Lighten",
                BlendMode::Screen        => "Screen",
                BlendMode::ColorDodge    => "Color Dodge",
                BlendMode::LinearDodge   => "Glow Dodge",
                BlendMode::LighterColor  => "Lighten",
                BlendMode::Overlay       => "Overlay",
                BlendMode::SoftLight     => "SoftLight",
                BlendMode::HardLight     => "Hard Light",
                BlendMode::VividLight    => "Vivid Light",
                BlendMode::LinearLight   => "Linear Light",
                BlendMode::PinLight      => "Pin Light",
                BlendMode::HardMix       => "Hard Mix",
                BlendMode::Difference    => "Difference",
                BlendMode::Exclusion     => "Exclusion",
                BlendMode::Subtract      => "Subtract",
                BlendMode::Divide        => "Divide",
                BlendMode::Hue           => "Hue",
                BlendMode::Saturation    => "Saturation",
                BlendMode::Color         => "Color",
                BlendMode::Luminosity    => "Luminosity",
                _ => "Normal",
            }.to_string();
            println!("layer {}: {} (of {:?})", i, image_layer.name, layer.parent_id());
            //group.children.insert(0, image_layer);
            group.children.push(image_layer);
            //layers.insert(i, image_layer);
        }
    }
    
    app.layers.children = vec!(group);
    //for (i, group) in psd.groups() {
    //    let name = group.name();
    //    println!("group {}: {}", i, name);
    //    for (j, n) in 
    //}
    println!("asdf");
}