use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

#[derive(Clone, Debug, Default)]
pub struct MaskData {
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
    pub mask_data : MaskData,
    pub image_data_masks : Vec<u8>,
    pub group_opener : bool,
    pub group_closer : bool,
    pub is_clipped : bool,
    pub is_alpha_locked : bool,
    pub is_visible : bool,
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
    println!("starting at: {:X}\t", cursor.position());
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
            println!("at: {:X} - {:X}\t", cursor.position(), c2.position());
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
        for _ in 0..h
        {
            let i2 = i;
            //print!("at: {:X} - {:X}\t", cursor.position(), c2.position());
            let len = read_u16(cursor);
            let start = c2.position();
            // FIXME: ignore overflow and pad out underflow?
            while c2.position() - start < len as u64
            {
                let n = read_u8(&mut c2) as i8;
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
                }
            }
            //println!("effective w: {}", i - i2);
            c2.set_position(start + len as u64);
        }
    }
    else
    {
        panic!("unsupported compression format at 0x{:X}", cursor.position() - 2);
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
        let mut image_data_masks : Vec<u8> = vec!();
        
        let mut rgba_count = 0;
        let mut has_g = false;
        let mut has_b = false;
        let mut has_a = false;
        let mut aux_count = 0;
        
        let mut cdat_cursor = cursor.clone();
        
        for _ in 0..image_channel_count
        {
            let _channel_id = read_u16(&mut cursor) as i16;
            let _channel_length = read_u32(&mut cursor) as usize;
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
        
        let mtop = read_i32(&mut cursor);
        let mleft = read_i32(&mut cursor);
        let mbottom = read_i32(&mut cursor);
        let mright = read_i32(&mut cursor);
        let mut mask_data = MaskData::default();
        mask_data.x = mleft;
        mask_data.y = mtop;
        mask_data.w = (mright - mleft) as u32;
        mask_data.h = (mbottom - mtop) as u32;
        mask_data.default_color = read_u8(&mut cursor);
        let mflags = read_u8(&mut cursor);
        mask_data.relative = (mflags & 1) != 0;
        mask_data.disabled = (mflags & 2) != 0;
        mask_data.invert = (mflags & 4) != 0;
        
        cursor.set_position(maskdat_start + maskdat_len);
        
        for _ in 0..image_channel_count
        {
            let channel_id = read_u16(&mut cdat_cursor) as i16;
            has_g |= channel_id == 1;
            has_b |= channel_id == 2;
            has_a |= channel_id == -1;
            let channel_length = read_u32(&mut cdat_cursor) as usize;
            println!("channel... {} {}", channel_id, channel_length);
            if channel_id >= -1 && channel_id <= 2
            {
                rgba_count += 1;
                let pos = if channel_id >= 0 { channel_id } else { 3 } as usize;
                //println!("{} {} {} {}", w, h, pos, channel_length);
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
                aux_count += 1;
                if channel_length > 2
                {
                    append_img_data(&mut idata_c, &mut image_data_masks, channel_length as u64, mask_data.h as u64);
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
            mask_data,
            image_data_masks,
            group_opener : false,
            group_closer : false,
            is_clipped : clipping == 1,
            is_alpha_locked : (flags & 1) != 0,
            is_visible : (flags & 2) == 0,
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
