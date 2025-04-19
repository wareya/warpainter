use std::collections::HashMap;

use crate::warimage::*;
use crate::transform::*;
use crate::layers::*;

pub (crate) fn blend_with_fx(
    flattened_data : &mut Option<Image<4>>,
    new_dirty_rect : &mut [[f32; 2]; 2],
    above_offset : [isize; 2],
    source_data : &Image<4>,
    child : &Layer,
    child_fx : Vec<(String, HashMap<String, Vec<FxData>>)>,
    opacity : f32,
    fill_opacity : f32,
    child_clipped : bool,
    child_funny_flag : bool,
    mode : String,
)
{
    //println!("{}", child_fx.len());
    //println!("{:?}", child_fx);
    let mut real_count = 0;
    for fx in &child_fx
    {
        if *fx.0 == "_enabled".to_string()
        {
            if fx.1["bool"][0].f() == 0.0
            {
                real_count = 0;
                break;
            }
            continue;
        }
        if (fx.1.contains_key("enabled") && fx.1["enabled"][0].f() == 0.0) || *fx.0 == "_scale".to_string()
        {
            continue;
        }
        real_count += 1;
    }
    if real_count > 0
    {
        // CLONE
        let mut fill = flattened_data.clone().unwrap();
        let mut dropshadow = None;
        // CLONE
        let mut fill_mask = fill.alike();
        // CLONE
        let mut full_mask = fill_mask.clone();
        
        let mut rect = *new_dirty_rect;
        
        // dropshadow needs special handling; the basic layer blend mode blends on top of it instead of below
        for fx in child_fx.iter()
        {
            if fx.0 != "dropshadow".to_string()
            {
                continue;
            }
            if fx.1["enabled"][0].f() == 0.0
            {
                continue;
            }
            
            let fx = fx.clone();
            
            let fx_opacity = fx.1["opacity"][0].f() as f32 / 100.0;
            let fx_mode = fx_get_early_blend_mode(&fx);
            let r = fx_get_radius(&fx).round();
            let r_int = r as isize;
            let offset2 = [above_offset[0] - r_int, above_offset[1] - r_int];
            
            if child.flattened_dirty_rect.is_some() || child.edited_dirty_rect.is_some()
            {
                *new_dirty_rect = rect_grow(*new_dirty_rect, r);
            }
            rect = rect_grow(rect, r);
            
            let rect_shifted = rect_translate(rect, [-above_offset[0] as f32, -above_offset[1] as f32]);
            
            // CLONE
            let weld_func = fx_get_weld_func(&fx);
            
            let mut overlay = fill.clone();
            
            let mut data = source_data.alike_grown(r_int as usize);
            data.apply_fx(rect_shifted, &fx, Some(source_data), child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, child.funny_flag, [r_int, r_int], "Normal");
            overlay.blend_rect_from(rect, &data, None, None, 1.0, 1.0, false, offset2, &fx_mode);
            
            // FIXME: use separate alpha and mask
            full_mask.blend_rect_from(rect, &data, None, None, 1.0, 1.0, false, offset2, "Weld");
            fill.blend_rect_from(rect, &overlay, None, None, fx_opacity, 1.0, false, [0, 0], &weld_func);
            
            // CLONE
            dropshadow = Some(fill.clone());
            break;
        }
        
        let rect_shifted = rect_translate(rect, [-above_offset[0] as f32, -above_offset[1] as f32]);
        
        let mut source = source_data.clone();
        source.clear_rect_alpha_float(rect_shifted, 1.0);
        
        fill.blend_rect_from(rect, &source, None, None, 1.0, fill_opacity, child.funny_flag, above_offset, &mode);
        
        fill_mask.blend_rect_from(rect, &source_data, child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, false, above_offset, "Copy");
        full_mask.blend_rect_from(rect, &source_data, child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, false, above_offset, "Normal");
        
        let mut fill_masking_performed = false;
        
        for mut fx in child_fx
        {
            if fx.0 == "_enabled".to_string() || fx.0 == "_scale".to_string() || fx.0 == "dropshadow".to_string()
            {
                continue;
            }
            if fx.1["enabled"][0].f() == 0.0
            {
                continue;
            }
            
            fx_update_metadata(&mut fx, &child, &source_data);
            
            println!("applying effect {}", fx.0);
            let r = fx_get_radius(&fx).round();
            let r_int = r as isize;
            
            //println!("{:?}", fx);
            let fx_opacity = fx.1["opacity"][0].f() as f32 / 100.0;
            //if child.flattened_dirty_rect.is_some() || child.edited_dirty_rect.is_some() || flattened_data.is_none() || dirty_rect.is_none()
            if child.flattened_dirty_rect.is_some() || child.edited_dirty_rect.is_some()
            {
                *new_dirty_rect = rect_grow(*new_dirty_rect, r);
            }
            rect = rect_grow(rect, r);
            
            let rect_shifted = rect_translate(rect, [-above_offset[0] as f32, -above_offset[1] as f32]);
            
            // CLONE
            let mut data = source_data.alike_grown(r_int as usize);
            data.apply_fx(rect_shifted, &fx, Some(source_data), child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, child.funny_flag, [r_int, r_int], "Normal");
            
            let offset2 = [above_offset[0] - r_int, above_offset[1] - r_int];
            
            let fx_mode = fx_get_early_blend_mode(&fx);
            println!("fx mode {} on effect {:?}", fx_mode, fx.0);
            let weld_func = fx_get_weld_func(&fx);
            let mask_func = fx_get_mask_func(&fx);
            
            // CLONE
            //let mut overlay = flattened_data.clone().unwrap();
            //let mut overlay = if !fx_is_fill(&fx) { flattened_data.clone().unwrap() } else { fill.clone() };
            let mut overlay = if !fx_is_fill(&fx) { if let Some(ds) = &dropshadow { ds.clone() } else { flattened_data.clone().unwrap() } } else { fill.clone() };
            // CLONE
            let mut overlay_mask = overlay.alike();
            overlay_mask.blend_rect_from(rect, &data, child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, false, offset2, "Copy");
            
            if !fx_is_fill(&fx)
            {
                data.clear_rect_alpha_float(rect_shifted, 1.0);
            }
            //data.clear_rect_alpha_float(rect_shifted, 1.0);
            overlay.blend_rect_from(rect, &data, child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, true, offset2, &fx_mode);
            
            if !fx_is_fill(&fx)
            {
                if !fill_masking_performed
                {
                    fill_masking_performed = true;
                    fill.blend_rect_from(rect, &fill_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
                    if let Some(ds) = &dropshadow
                    {
                        let mut d2 = ds.clone();
                        d2.blend_rect_from(rect, &fill, None, None, 1.0, 1.0, false, [0, 0], "Erase");
                        d2.blend_rect_from(rect, &full_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
                        fill.blend_rect_from(rect, &d2, None, None, 1.0, 1.0, false, [0, 0], "Normal");
                    }
                }
            }
            else
            {
                //overlay.blend_rect_from(rect, &overlay_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
                fill.blend_rect_from(rect, &overlay, None, None, 1.0, 1.0, false, [0, 0], "Interpolate");
                continue;
            }
            
            if !fx_is_fill(&fx)
            {
                overlay.blend_rect_from(rect, &overlay_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
            }
            
            if !fx_is_fill(&fx)
            {
                let mut fill2 = fill.clone();
                full_mask.blend_rect_from(rect, &overlay, None, None, 1.0, 1.0, false, [0, 0], "Erase");
                full_mask.blend_rect_from(rect, &overlay, None, None, fx_opacity, 1.0, false, [0, 0], &mask_func);
                fill.blend_rect_from(rect, &overlay, None, None, 1.0, 1.0, false, [0, 0], "Erase");
                if let Some(ds) = &dropshadow
                {
                    let mut d2 = ds.clone();
                    d2.blend_rect_from(rect, &overlay, None, None, 1.0, 1.0, false, [0, 0], "Clip Alpha");
                    fill.blend_rect_from(rect, &d2, None, None, 1.0, 1.0, false, [0, 0], "Weld");
                }
                fill2.blend_rect_from(rect, &overlay, None, None, 1.0, 1.0, false, [0, 0], &weld_func);
                fill.blend_rect_from(rect, &fill2, None, None, fx_opacity, 1.0, false, [0, 0], "Interpolate");
            }
            else
            {
                full_mask.blend_rect_from(rect, &overlay, None, None, fx_opacity, 1.0, false, [0, 0], &mask_func);
                fill.blend_rect_from(rect, &overlay, None, None, fx_opacity, 1.0, false, [0, 0], &weld_func);
            }
        }
        
        if !fill_masking_performed
        {
            fill.blend_rect_from(rect, &fill_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
            if let Some(mut ds) = dropshadow
            {
                ds.blend_rect_from(rect, &fill, None, None, 1.0, 1.0, false, [0, 0], "Erase");
                ds.blend_rect_from(rect, &full_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
                fill.blend_rect_from(rect, &ds, None, None, 1.0, 1.0, false, [0, 0], "Weld");
            }
        }
        
        flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Alpha Antiblend");
        flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Blend Weld");
    }
    else
    {
        flattened_data.as_mut().unwrap().blend_rect_from(*new_dirty_rect, source_data, child.mask.as_ref(), child.mask_info.as_ref(), opacity, fill_opacity, child.funny_flag, above_offset, &mode);
    }
}