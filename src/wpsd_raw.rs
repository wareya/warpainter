use std::io::Cursor;
use std::io::Read;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub enum DescItem
{
    long(i32),
    doub(f64),
    bool(bool),
    TEXT(String),
    Err(String),
    Objc(Box<Descriptor>),
    #[default] Xxx
}

impl DescItem
{
    pub fn long(&self) -> i32 { match self { DescItem::long(x) => return *x, _ => panic!(), } }
    pub fn doub(&self) -> f64 { match self { DescItem::doub(x) => return *x, _ => panic!(), } }
    pub fn bool(&self) -> bool { match self { DescItem::bool(x) => return *x, _ => panic!(), } }
}

type Descriptor = (String, Vec<(String, DescItem)>);

#[derive(Clone, Debug, Default)]
pub struct MaskInfo {
    pub x : i32,
    pub y : i32,
    pub w : u32,
    pub h : u32,
    pub default_color : u8,
    pub relative : bool,
    pub disabled : bool,
    pub invert : bool,
}

#[derive(Clone, Debug, Default)]
pub struct LayerInfo {
    pub name : String,
    pub opacity : f32,
    pub fill_opacity : f32,
    pub blend_mode : String,
    pub x : i32,
    pub y : i32,
    pub w : u32,
    pub h : u32,
    pub image_channel_count : u16,
    pub image_data_rgba : Vec<u8>,
    pub image_data_k : Vec<u8>,
    pub image_data_has_g : bool,
    pub image_data_has_b : bool,
    pub image_data_has_a : bool,
    pub mask_channel_count : u16,
    pub mask_info : MaskInfo,
    //pub global_mask_opacity : u16,
    //pub global_mask_kind : u16,
    pub image_data_mask : Vec<u8>,
    pub group_opener : bool,
    pub group_closer : bool,
    pub is_clipped : bool,
    pub is_alpha_locked : bool,
    pub is_visible : bool,
    pub adjustment_type : String,
    pub adjustment_info : Vec<f32>,
    pub adjustment_desc : Option<Descriptor>,
}

fn read_u8(cursor: &mut Cursor<&[u8]>) -> u8
{
    let mut buf = [0; 1];
    cursor.read_exact(&mut buf).expect("Failed to read u8");
    buf[0]
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> u16
{
    let mut buf = [0; 2];
    cursor.read_exact(&mut buf).expect("Failed to read u16");
    u16::from_be_bytes(buf)
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> u32
{
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).expect("Failed to read u32");
    u32::from_be_bytes(buf)
}

fn read_i32(cursor: &mut Cursor<&[u8]>) -> i32
{
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).expect("Failed to read i32");
    i32::from_be_bytes(buf)
}

fn read_f32(cursor: &mut Cursor<&[u8]>) -> f32
{
    let mut buf = [0; 4];
    cursor.read_exact(&mut buf).expect("Failed to read f32");
    f32::from_be_bytes(buf)
}

fn read_f64(cursor: &mut Cursor<&[u8]>) -> f64
{
    let mut buf = [0; 8];
    cursor.read_exact(&mut buf).expect("Failed to read f64");
    f64::from_be_bytes(buf)
}

pub fn parse_psd_metadata(data : &[u8]) -> PsdMetadata
{
    let mut cursor = Cursor::new(&data[..]);

    let mut signature = [0; 4];
    cursor.read_exact(&mut signature).expect("Failed to read PSD signature");
    if signature != [0x38, 0x42, 0x50, 0x53]
    {
        panic!("Invalid PSD signature");
    }

    let version = read_u16(&mut cursor);
    if version != 1
    {
        panic!("Unsupported PSD version");
    }

    cursor.set_position(cursor.position() + 6);

    let channel_count = read_u16(&mut cursor);
    let height = read_u32(&mut cursor);
    let width = read_u32(&mut cursor);
    let depth = read_u16(&mut cursor);
    let color_mode = read_u16(&mut cursor);

    PsdMetadata
    {
        width,
        height,
        channel_count,
        depth,
        color_mode,
    }
}
pub fn append_img_data(cursor : &mut Cursor<&[u8]>, output : &mut Vec<u8>, size : u64, h : u64)
{
    //println!("starting at: {:X}\t", cursor.position());
    let mode = read_u16(cursor);
    if mode == 0
    {
        cursor.take(size).read_to_end(output).unwrap();
    }
    else if mode == 1
    {
        let mut c2 = cursor.clone();
        c2.set_position(c2.position() + h * 2);
        for _ in 0..h
        {
            //println!("at: {:X} - {:X}\t", cursor.position(), c2.position());
            let len = read_u16(cursor);
            let start = c2.position();
            // FIXME: ignore overflow and pad out underflow?
            while c2.position() < start as u64 + len as u64
            {
                let n = read_u8(&mut c2) as i8;
                if n >= 0
                {
                    (&mut c2).take(n as u64 + 1).read_to_end(output).unwrap();
                }
                else if n != -128
                {
                    output.extend(std::iter::repeat(read_u8(&mut c2)).take((1 - n as i64) as usize));
                }
            }
        }
        cursor.set_position(c2.position());
    }
    else
    {
        panic!("unsupported compression format");
    }
}
pub fn copy_img_data(cursor : &mut Cursor<&[u8]>, output : &mut [u8], stride : usize, size : u64, h : u64)
{
    //println!("pos... 0x{:X}", cursor.position());
    let pos = cursor.position();
    let mode = read_u16(cursor);
    //println!("size... 0x{:X}", size as usize - 2);
    if mode == 0
    {
        for i in 0..size as usize - 2
        {
            output[i*stride] = read_u8(cursor);
        }
    }
    else if mode == 1
    {
        let mut c2 = cursor.clone();
        c2.set_position(c2.position() + h * 2);
        let mut i = 0;
        let mut j = 2;
        for _ in 0..h
        {
            let i2 = i;
            //print!("at: {:X} - {:X}\t", cursor.position(), c2.position());
            let len = read_u16(cursor);
            j += 2;
            let start = c2.position();
            // FIXME: ignore overflow and pad out underflow?
            while c2.position() - start < len as u64
            {
                let n = read_u8(&mut c2) as i8;
                j += 1;
                if n >= 0
                {
                    for _ in 0..n as u64 + 1
                    {
                        let c = read_u8(&mut c2);
                        if i*stride < output.len()
                        {
                            output[i*stride] = c;
                        }
                        i += 1;
                        j += 1;
                    }
                }
                else if n != -128
                {
                    let c = read_u8(&mut c2);
                    for _ in 0..1 - n as i64
                    {
                        if i*stride < output.len()
                        {
                            output[i*stride] = c;
                        }
                        i += 1;
                    }
                    j += 1;
                }
            }
            //println!("effective w: {}", i - i2);
            c2.set_position(start + len as u64);
        }
        assert!(j == size, "{} {}", j, size);
    }
    else
    {
        panic!("unsupported compression format {} at 0x{:X}", mode, pos);
    }
    cursor.set_position(pos + size);
}
pub fn parse_layer_records(data : &[u8]) -> Vec<LayerInfo>
{
    let metadata = parse_psd_metadata(data);
    assert!(metadata.depth == 8);
    assert!(metadata.color_mode == 3);
    
    let mut cursor = Cursor::new(&data[..]);
    cursor.set_position(26);

    let color_mode_length = read_u32(&mut cursor) as u64;
    cursor.set_position(cursor.position() + color_mode_length);

    let image_resources_length = read_u32(&mut cursor) as u64;
    cursor.set_position(cursor.position() + image_resources_length);

    let layer_mask_info_length = read_u32(&mut cursor) as u64;
    let layer_mask_info_end = cursor.position() + layer_mask_info_length;

    let layer_info_length = read_u32(&mut cursor) as u64;
    let layer_info_end = cursor.position() + layer_info_length;
    
    let layer_count = read_u16(&mut cursor) as i16;
    let layer_count = layer_count.abs(); // If negative, transparency info exists
    
    println!("starting at {:X}", cursor.position());
    
    let mut idata_c = Cursor::new(&data[..]);
    idata_c.set_position(cursor.position());
    
    for _ in 0..layer_count
    {
        read_i32(&mut idata_c);
        read_i32(&mut idata_c);
        read_i32(&mut idata_c);
        read_i32(&mut idata_c);
        let image_channel_count = read_u16(&mut idata_c) as u64;
        idata_c.set_position(idata_c.position() + 6*image_channel_count + 4 + 4 + 4);
        let idat_len = read_u32(&mut idata_c) as u64;
        idata_c.set_position(idata_c.position() + idat_len);
    }

    let mut layers = Vec::new();

    for i in 0..layer_count
    {
        let top = read_i32(&mut cursor);
        let left = read_i32(&mut cursor);
        let bottom = read_i32(&mut cursor);
        let right = read_i32(&mut cursor);

        let x = left;
        let y = top;
        let w = (right - left) as u32;
        let h = (bottom - top) as u32;
        
        let image_channel_count = read_u16(&mut cursor);
        //println!("chan count {}", image_channel_count);
        
        let channel_info_start = cursor.position();
        
        cursor.set_position(channel_info_start);
        let mut image_data_rgba : Vec<u8> = vec![255u8; (w * h * 4) as usize];
        let mut image_data_k : Vec<u8> = vec!();
        let mut image_data_mask : Vec<u8> = vec!();
        
        let mut rgba_count = 0;
        let mut has_g = false;
        let mut has_b = false;
        let mut has_a = false;
        let mut aux_count = 0;
        
        let mut cdat_cursor = cursor.clone();
        
        let mut has_neg2 = false;
        let mut has_neg3 = false;
        for _ in 0..image_channel_count
        {
            let channel_id = read_u16(&mut cursor) as i16;
            let _channel_length = read_u32(&mut cursor) as usize;
            has_neg2 = has_neg2 || channel_id == -2;
            has_neg3 = has_neg3 || channel_id == -3;
        }
        
        let mut blend_mode_signature = [0; 4];
        cursor.read_exact(&mut blend_mode_signature).expect("Failed to read blend mode signature");
        assert!(blend_mode_signature == [0x38, 0x42, 0x49, 0x4D]);

        let mut blend_mode_key = [0; 4];
        cursor.read_exact(&mut blend_mode_key).expect("Failed to read blend mode key");
        let blend_mode = String::from_utf8_lossy(&blend_mode_key).to_string();

        let opacity = read_u8(&mut cursor) as f32 / 255.0;
        println!("opacity: {}", opacity * 100.0);
        let clipping = read_u8(&mut cursor);
        let flags = read_u8(&mut cursor);
        let _filler = read_u8(&mut cursor);

        let exdat_len = read_u32(&mut cursor) as u64;
        let exdat_start = cursor.position();
        
        let maskdat_len = read_u32(&mut cursor) as u64;
        let maskdat_start = cursor.position();
        
        // FIXME: support maskdat_len == 0 case
        let mtop = read_i32(&mut cursor);
        let mleft = read_i32(&mut cursor);
        let mbottom = read_i32(&mut cursor);
        let mright = read_i32(&mut cursor);
        let mut mask_info = MaskInfo::default();
        mask_info.x = mleft;
        mask_info.y = mtop;
        mask_info.w = (mright - mleft) as u32;
        mask_info.h = (mbottom - mtop) as u32;
        mask_info.default_color = read_u8(&mut cursor);
        let mflags = read_u8(&mut cursor);
        mask_info.relative = (mflags & 1) != 0;
        mask_info.disabled = (mflags & 2) != 0;
        mask_info.invert = (mflags & 4) != 0;
        
        cursor.set_position(maskdat_start + maskdat_len);
        
        for n in 0..image_channel_count
        {
            let channel_id = read_u16(&mut cdat_cursor) as i16;
            has_g |= channel_id == 1;
            has_b |= channel_id == 2;
            has_a |= channel_id == -1;
            let channel_length = read_u32(&mut cdat_cursor) as usize;
            println!("channel... {} {} at 0x{:X}", channel_id, channel_length, idata_c.position());
            if channel_id >= -1 && channel_id <= 2
            {
                rgba_count += 1;
                let pos = if channel_id >= 0 { channel_id } else { 3 } as usize;
                println!("{} {} {} {}", w, h, pos, channel_length);
                if channel_length > 2
                {
                    copy_img_data(&mut idata_c, &mut image_data_rgba[pos..], 4, channel_length as u64, h as u64);
                }
                else
                {
                    idata_c.set_position(idata_c.position() + 2);
                }
            }
            else if channel_id == 3 // CMYK's K
            {
                if channel_length > 2
                {
                    append_img_data(&mut idata_c, &mut image_data_k, channel_length as u64, h as u64);
                }
                else
                {
                    idata_c.set_position(idata_c.position() + 2);
                }
            }
            else
            {
                println!("mask... {} {} {}", mask_info.w, mask_info.h, channel_length);
                aux_count += 1;
                if aux_count > 1
                {
                    idata_c.set_position(idata_c.position() + channel_length as u64);
                }
                else if channel_length > 2
                {
                    println!("adding mask data...");
                    append_img_data(&mut idata_c, &mut image_data_mask, channel_length as u64, mask_info.h as u64);
                }
                else
                {
                    idata_c.set_position(idata_c.position() + 2);
                }
            }
        }
        
        let blendat_len = read_u32(&mut cursor) as u64;
        cursor.set_position(cursor.position() + blendat_len);
        
        let mut name_len = read_u8(&mut cursor);
        while (name_len + 1) % 4 != 0
        {
            name_len += 1;
        }
        let mut name = vec![0; name_len as usize];
        cursor.read_exact(&mut name[..]).expect("Failed to read ASCII name");
        let name = String::from_utf8_lossy(&name).to_string();

        let mut layer = LayerInfo {
            name,
            opacity,
            fill_opacity : 1.0,
            blend_mode,
            x,
            y,
            w,
            h,
            image_channel_count,
            image_data_rgba,
            image_data_k,
            image_data_has_g : has_g,
            image_data_has_b : has_b,
            image_data_has_a : has_a,
            mask_channel_count : aux_count,
            mask_info,
            image_data_mask,
            group_opener : false,
            group_closer : false,
            is_clipped : clipping == 1,
            is_alpha_locked : (flags & 1) != 0,
            is_visible : (flags & 2) == 0,
            adjustment_type : "".to_string(),
            adjustment_info : vec!(),
            adjustment_desc : None,
        };
        
        //println!("--- {:X}", cursor.position());
        
        while cursor.position() < exdat_start + exdat_len
        {
            let mut sig = [0; 4];
            cursor.read_exact(&mut sig).expect("Failed to read metadata signature");
            assert!(sig == [0x38, 0x42, 0x49, 0x4D]);
            
            let mut name = [0; 4];
            cursor.read_exact(&mut name).expect("Failed to read metadata name");
            let name = String::from_utf8_lossy(&name).to_string();
            
            let len = read_u32(&mut cursor) as u64;
            //println!("?? {}", len);
            let start = cursor.position();
            
            println!("reading metadata.... {}", name.as_str());
            
            fn read_descriptor(c : &mut Cursor<&[u8]>) -> Descriptor
            {
                // skip name. usually/often blank
                let n = read_u32(c) as u64;
                c.set_position(c.position() + n * 2);
                
                let mut idlen = read_u32(c);
                if idlen == 0 { idlen = 4; }
                let mut id = vec![0; idlen as usize];
                c.read_exact(&mut id).unwrap();
                let id = String::from_utf8_lossy(&id).to_string();
                
                let mut data = vec!();
                
                let itemcount = read_u32(c);
                
                for _ in 0..itemcount
                {
                    let mut namelen = read_u32(c);
                    if namelen == 0 { namelen = 4; }
                    let mut name = vec![0; namelen as usize];
                    c.read_exact(&mut name).unwrap();
                    let name = String::from_utf8_lossy(&name).to_string();
                    
                    let mut id = vec![0; 4];
                    c.read_exact(&mut id).unwrap();
                    let id = String::from_utf8_lossy(&id).to_string();
                    
                    match id.as_str()
                    {
                        "long" => data.push((name, DescItem::long(read_i32(c)))),
                        "doub" => data.push((name, DescItem::doub(read_f64(c)))),
                        "Objc" => data.push((name, DescItem::Objc(Box::new(read_descriptor(c))))),
                        "bool" => data.push((name, DescItem::bool(read_u8(c) != 0))),
                        "TEXT" =>
                        {
                            let len = read_u32(c) as u64;
                            let mut text = vec![0; len as usize * 2];
                            for i in 0..len
                            {
                                text[i as usize] = read_u16(c);
                            }
                            let text = String::from_utf16_lossy(&text).to_string();
                            data.push((name, DescItem::TEXT(text)));
                        }
                        _ =>
                        {
                            println!("!!! errant descriptor subobject type... {}", id);
                            data.push((name, DescItem::Err(id)));
                            break;
                        }
                    }
                }
                
                //
                
                (id, data)
            };
            
            match name.as_str()
            {
                "lsct" =>
                {
                    let kind = read_u32(&mut cursor) as u64;
                    layer.group_opener = kind == 1 || kind == 2;
                    layer.group_closer = kind == 3;
                    if kind == 1 || kind == 2
                    {
                        println!("group opener!");
                    }
                    if kind == 3
                    {
                        println!("group closer!");
                    }
                }
                "luni" =>
                {
                    let len = read_u32(&mut cursor) as u64;
                    let mut name = vec![0; len as usize * 2];
                    for i in 0..len
                    {
                        name[i as usize] = read_u16(&mut cursor);
                    }
                    layer.name = String::from_utf16_lossy(&name).to_string();
                }
                "tsly" =>
                {
                    let thing = read_u8(&mut cursor);
                    if thing == 0 && layer.blend_mode == "lddg"
                    {
                        layer.blend_mode = "lddg_glow".to_string();
                    }
                }
                "iOpa" =>
                {
                    layer.fill_opacity = read_u8(&mut cursor) as f32 / 255.0;
                }
                // adjustment layers
                "post" =>
                {
                    let mut data = vec!();
                    data.push(read_u16(&mut cursor) as f32); // number of levels
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = data;
                }
                "nvrt" =>
                {
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = vec!();
                }
                "brit" =>
                {
                    let mut data = vec!();
                    data.push(read_u16(&mut cursor) as f32); // brightness
                    data.push(read_u16(&mut cursor) as f32); // contrast
                    data.push(read_u16(&mut cursor) as f32); // "Mean value for brightness and contrast"
                    data.push(read_u8(&mut cursor) as f32); // "Lab color only"
                    data.push(1.0); // legacy mode
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = data;
                }
                "thrs" =>
                {
                    let mut data = vec!();
                    data.push(read_u16(&mut cursor) as f32);
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = data;
                }
                "hue2" =>
                {
                    let mut data = vec!();
                    
                    //assert!(read_u16(&mut cursor) == 2);
                    read_u16(&mut cursor); // version
                    data.push(read_u8(&mut cursor) as f32); // if 1, is absolute/colorization (rather than relative)
                    read_u8(&mut cursor);
                    
                    // "colorization"
                    data.push(read_u16(&mut cursor) as i16 as f32); // hue
                    data.push(read_u16(&mut cursor) as i16 as f32); // sat
                    data.push(read_u16(&mut cursor) as i16 as f32); // lightness (-1 to +1)
                    
                    // "master"
                    data.push(read_u16(&mut cursor) as i16 as f32); // hue
                    data.push(read_u16(&mut cursor) as i16 as f32); // sat
                    data.push(read_u16(&mut cursor) as i16 as f32); // lightness (-1 to +1)
                    
                    // todo: read hextant values?
                    
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = data;
                }
                "levl" =>
                {
                    let mut data = vec!();
                    
                    assert!(read_u16(&mut cursor) == 2);
                    for i in 0..28
                    {
                        data.push(read_u16(&mut cursor) as f32 / 255.0); // in floor
                        data.push(read_u16(&mut cursor) as f32 / 255.0); // in ceil
                        data.push(read_u16(&mut cursor) as f32 / 255.0); // out floor
                        data.push(read_u16(&mut cursor) as f32 / 255.0); // out ceil
                        data.push(read_u16(&mut cursor) as f32 / 100.0); // gamma
                    }
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = data;
                }
                "curv" =>
                {
                    let mut data = vec!();
                    
                    read_u8(&mut cursor);
                    assert!(read_u16(&mut cursor) == 1);
                    let enabled = read_u32(&mut cursor);
                    
                    for i in 0..32
                    {
                        if (enabled & (1u32 << i)) != 0
                        {
                            let n = read_u16(&mut cursor);
                            data.push(n as f32); // number of points
                            for _ in 0..n
                            {
                                let y = read_u16(&mut cursor) as f32 / 255.0;
                                data.push(read_u16(&mut cursor) as f32 / 255.0); // x
                                data.push(y); // y
                            }
                        }
                        else
                        {
                            data.push(0.0); // number of points
                        }
                    }
                    layer.adjustment_type = name.clone();
                    layer.adjustment_info = data;
                }
                "blwh" =>
                {
                    assert!(read_u32(&mut cursor) == 16);
                    layer.adjustment_type = name.clone();
                    layer.adjustment_desc = Some(read_descriptor(&mut cursor));
                }
                "CgEd" =>
                {
                    assert!(read_u32(&mut cursor) == 16);
                    //layer.adjustment_type = name.clone();
                    //layer.adjustment_type = "brit".to_string();
                    let temp = read_descriptor(&mut cursor).1;
                    println!("{:?}", temp);
                    let mut n = HashMap::new();
                    for t in temp
                    {
                        n.insert(t.0, t.1);
                    }
                    println!("{:?}", n);
                    //("null", [("Vrsn", long(1)), ("Brgh", long(9)), ("Cntr", long(30)), ("means", long(127)), ("Lab ", bool(false)), ("useLegacy", bool(true)), ("Auto", bool(true))])
                    let mut data = vec!();
                    data.push(n.get("Brgh").unwrap().long() as f32);
                    data.push(n.get("Cntr").unwrap().long() as f32);
                    data.push(n.get("means").unwrap().long() as f32);
                    data.push(n.get("Lab ").unwrap().bool() as u8 as f32);
                    data.push(n.get("useLegacy").unwrap().bool() as u8 as f32);
                    println!("??????????? {:?}", data);
                    layer.adjustment_info = data;
                }
                _ => {}
            }
            cursor.set_position(start + len);
        }
        //println!("{:X} {:X}", cursor.position(), exdat_start + exdat_len);
        assert!(cursor.position() == exdat_start + exdat_len);
        
        println!("added layer with name {}", layer.name);
        layers.push(layer);
    }
    
    layers
}

#[derive(Debug, PartialEq)]
pub struct PsdMetadata {
    pub width: u32,
    pub height: u32,
    pub color_mode: u16,
    pub depth: u16,
    pub channel_count: u16,
}
