use std::any::Any;
use uuid::Uuid;
use crate::warimage::*;
use crate::transform::*;
use crate::wpsd_raw::MaskInfo;
use std::collections::HashMap;


#[derive(Clone, Debug, Default, Decode, Encode)]
pub (crate) enum FxData
{
    VF(Vec<f64>),
    VVF(Vec<Vec<f64>>),
    F(f64),
    S(String),
    #[default] Xxx,
}
impl From<Vec<f64>> for FxData
{
    fn from(value : Vec<f64>) -> Self
    {
        FxData::VF(value)
    }
}
impl From<Vec<Vec<f64>>> for FxData
{
    fn from(value : Vec<Vec<f64>>) -> Self
    {
        FxData::VVF(value)
    }
}
impl From<f64> for FxData
{
    fn from(value : f64) -> Self
    {
        FxData::F(value)
    }
}
impl From<bool> for FxData
{
    fn from(value : bool) -> Self
    {
        FxData::F(value.into())
    }
}
impl From<String> for FxData
{
    fn from(value : String) -> Self
    {
        FxData::S(value)
    }
}
impl FxData
{
    pub (crate) fn vvf(&self) -> &Vec<Vec<f64>> { match self { FxData::VVF(x) => return x, _ => panic!(), } }
    pub (crate) fn vf(&self) -> &Vec<f64> { match self { FxData::VF(x) => return x, _ => panic!(), } }
    pub (crate) fn f(&self) -> f64 { match self { FxData::F(x) => return *x, _ => panic!(), } }
    pub (crate) fn s(&self) -> String { match self { FxData::S(x) => return x.clone(), _ => panic!(), } }
}

use bincode::{Decode, Encode};
#[derive(Clone, Debug, Default, Decode, Encode)]
pub (crate) struct LayerInfo
{
    pub (crate) name : String,
    pub (crate) blend_mode : String,
    
    pub (crate) opacity : f32,
    pub (crate) fill_opacity : f32,
    pub (crate) visible : bool,
    
    pub (crate) funny_flag : bool,
    pub (crate) clipped : bool,
    pub (crate) locked : bool,
    pub (crate) alpha_locked : bool,
    
    pub (crate) effects : HashMap<String, HashMap<String, Vec<FxData>>>,
}

impl LayerInfo
{
    fn new(name : String) -> Self
    {
        Self {
            name,
            blend_mode : "Normal".to_string(),
            
            opacity : 1.0,
            fill_opacity : 1.0,
            visible : true,
            
            funny_flag : false,
            clipped : false,
            locked : false,
            alpha_locked : false,
            
            effects : HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub (crate) enum Adjustment
{
    Invert,
    Posterize(f32),
    Threshold(f32),
    BrightContrast([f32; 5]),
    HueSatLum([f32; 3]),
    Levels(Vec<[f32; 5]>),
    Curves(Vec<Vec<[f32; 2]>>),
    BlackWhite(([f32; 6], bool, [f32; 3])),
    #[default] Xxx,
}

pub (crate) trait CloneAny : Any
{ 
    fn any(&self) -> &dyn Any;
    fn mut_any(&mut self) -> &mut dyn Any;
    fn clone_box(&self) -> Box<dyn CloneAny>;
}
impl<T : Any + Clone> CloneAny for T
{
    fn any(&self) -> &dyn Any
    {
        self
    }
    fn mut_any(&mut self) -> &mut dyn Any
    {
        self
    }
    fn clone_box(&self) -> Box<dyn CloneAny>
    {
        Box::new(self.clone())
    }
}
impl dyn CloneAny
{
    pub fn to_ref<T : Any>(&self) -> Option<&T>
    {
        self.any().downcast_ref::<T>()
    }
    pub fn to_mut<T : Any>(&mut self) -> Option<&mut T>
    {
        self.mut_any().downcast_mut::<T>()
    }
}
impl Clone for Box<dyn CloneAny>
{
    fn clone(&self) -> Self
    {
        (**self).clone_box().into()
    }
}
impl std::fmt::Debug for Box<dyn CloneAny>
{
    fn fmt(&self, f : &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "<Box CloneAny>")
    }
}

#[derive(Clone, Debug, Default)]
pub (crate) struct Layer
{
    pub (crate) uuid : u128,
    
    pub (crate) data : Option<Image<4>>,
    pub (crate) children : Vec<Layer>,
    
    pub (crate) mask : Option<Image<1>>,
    pub (crate) mask_info : Option<MaskInfo>,
    
    pub (crate) flattened_data : Option<Image<4>>,
    pub (crate) flattened_dirty_rect : Option<[[f32; 2]; 2]>,
    pub (crate) edited_dirty_rect : Option<[[f32; 2]; 2]>,
    
    pub (crate) offset : [f32; 2],
    
    pub (crate) name : String,
    pub (crate) blend_mode : String,
    pub (crate) custom_blend_mode : String,
    
    pub (crate) opacity : f32,
    pub (crate) fill_opacity : f32,
    pub (crate) visible : bool,
    
    pub (crate) funny_flag : bool,
    pub (crate) clipped : bool,
    pub (crate) locked : bool,
    pub (crate) alpha_locked : bool,
    
    pub (crate) old_info_for_undo : LayerInfo,
    
    pub (crate) adjustment : Option<Adjustment>,
    
    pub (crate) effects : HashMap<String, HashMap<String, Vec<FxData>>>,
    
    pub (crate) thumbnail : Option<Box<dyn CloneAny>>,
}

impl Layer
{
    pub (crate) fn get_info(&self) -> LayerInfo
    {
        LayerInfo {
            name : self.name.clone(),
            blend_mode : self.blend_mode.clone(),
            opacity : self.opacity,
            fill_opacity : self.fill_opacity,
            visible : self.visible,
            funny_flag : self.funny_flag,
            clipped : self.clipped,
            locked : self.locked,
            alpha_locked : self.alpha_locked,
            
            effects : self.effects.clone(),
        }
    }
    pub (crate) fn set_info(&mut self, info : &LayerInfo)
    {
        self.name = info.name.clone();
        self.blend_mode = info.blend_mode.clone();
        self.opacity = info.opacity;
        self.fill_opacity = info.fill_opacity;
        self.visible = info.visible;
        self.funny_flag = info.funny_flag;
        self.clipped = info.clipped;
        self.locked = info.locked;
        self.alpha_locked = info.alpha_locked;
        
        self.effects = info.effects.clone();
        
        self.commit_info();
    }
    pub (crate) fn commit_info(&mut self)
    {
        self.old_info_for_undo = self.get_info();
    }
    pub(crate) fn new_layer_from_image<T : ToString>(name : T, image : Image<4>) -> Self
    {
        Layer {
            name : name.to_string(),
            blend_mode : "Normal".to_string(),
            custom_blend_mode : "".to_string(),
            
            data : Some(image),
            mask : None,
            mask_info : None,
            adjustment : None,
            children : vec!(),
            
            flattened_data : None,
            flattened_dirty_rect : None,
            edited_dirty_rect : None,
            
            uuid : Uuid::new_v4().as_u128(),
            
            offset : [0.0, 0.0],
            
            opacity : 1.0,
            fill_opacity : 1.0,
            visible : true,
            
            funny_flag : false,
            clipped : false,
            locked : false,
            alpha_locked : false,
            
            effects : HashMap::new(),
            
            thumbnail : None,
            
            old_info_for_undo : LayerInfo::new(name.to_string()),
        }
    }
    pub(crate) fn new_layer<T : ToString>(name : T, w : usize, h : usize) -> Self
    {
        Self::new_layer_from_image(name, Image::blank(w, h))
    }
    pub(crate) fn new_group<T : ToString>(name : T) -> Self
    {
        Layer {
            name : name.to_string(),
            blend_mode : "Normal".to_string(),
            custom_blend_mode : "".to_string(),
            
            data : None,
            mask : None,
            mask_info : None,
            adjustment : None,
            children : vec!(),
            
            flattened_data : None,
            flattened_dirty_rect : None,
            edited_dirty_rect : None,
            
            uuid : Uuid::new_v4().as_u128(),
            
            offset : [0.0, 0.0],
            
            opacity : 1.0,
            fill_opacity : 1.0,
            visible : true,
            
            funny_flag : false,
            clipped : false,
            locked : false,
            alpha_locked : false,
            
            effects : HashMap::new(),
            
            thumbnail : None,
            
            old_info_for_undo : LayerInfo::new(name.to_string()),
        }
    }
    pub(crate) fn is_drawable(&self) -> bool
    {
        self.data.is_some()
    }
    pub(crate) fn is_group(&self) -> bool
    {
        self.data.is_none()
    }
    pub(crate) fn find_layer(&self, uuid : u128) -> Option<&Layer>
    {
        if self.uuid == uuid
        {
            Some(self)
        }
        else
        {
            for child in self.children.iter()
            {
                let r = child.find_layer(uuid);
                if r.is_some()
                {
                    return r;
                }
            }
            None
        }
    }
    pub(crate) fn find_layer_unlocked(&self, uuid : u128) -> Option<&Layer>
    {
        if let Some(layer) = self.find_layer(uuid)
        {
            if !layer.locked
            {
                return Some(layer);
            }
        }
        None
    }
    pub(crate) fn find_layer_unlocked_mut(&mut self, uuid : u128) -> Option<&mut Layer>
    {
        if let Some(layer) = self.find_layer_mut(uuid)
        {
            if !layer.locked
            {
                return Some(layer);
            }
        }
        None
    }
    pub(crate) fn find_layer_mut(&mut self, uuid : u128) -> Option<&mut Layer>
    {
        if self.uuid == uuid
        {
            Some(self)
        }
        else
        {
            for child in self.children.iter_mut()
            {
                let r = child.find_layer_mut(uuid);
                if r.is_some()
                {
                    return r;
                }
            }
            None
        }
    }
    pub(crate) fn find_layer_parent(&self, uuid : u128) -> Option<&Layer>
    {
        if self.uuid == uuid
        {
            None
        }
        else
        {
            for child in self.children.iter()
            {
                let is_some = child.find_layer(uuid).is_some();
                if is_some
                {
                    return Some(self);
                }
            }
            None
        }
    }
    pub(crate) fn find_layer_parent_mut(&mut self, uuid : u128) -> Option<&mut Layer>
    {
        if self.uuid == uuid
        {
            None
        }
        else
        {
            for child in self.children.iter_mut()
            {
                let is_some = child.find_layer_mut(uuid).is_some();
                if is_some
                {
                    return Some(self);
                }
            }
            None
        }
    }
    pub(crate) fn get_flatten_dirty_rect(&self) -> Option<[[f32; 2]; 2]>
    {
        if self.flattened_dirty_rect.is_some()// && Some(self.uuid) == override_uuid
        {
            return self.flattened_dirty_rect;
        }
        let mut reference = None;
        for child in self.children.iter()
        {
            if let Some(inner) = child.get_flatten_dirty_rect()
            {
                if reference.is_some()
                {
                    reference = Some(rect_enclose_rect(reference.unwrap(), inner));
                }
                else
                {
                    reference = Some(inner);
                }
            }
        }
        reference
    }
    pub(crate) fn dirtify_rect(&mut self, inner : [[f32; 2]; 2])
    {
        if self.flattened_dirty_rect.is_some()
        {
            self.flattened_dirty_rect = Some(rect_enclose_rect(self.flattened_dirty_rect.unwrap(), inner));
        }
        else
        {
            self.flattened_dirty_rect = Some(rect_normalize(inner));
        }
        let mut biggen = 0.0;
        for _fx in &self.effects
        {
            biggen += 3.0; // FIXME
        }
        if biggen != 0.0
        {
            *self.flattened_dirty_rect.as_mut().unwrap() = rect_grow(self.flattened_dirty_rect.unwrap(), 3.0);
        }
        self.edited_dirty_rect = Some(rect_enclose_rect(self.edited_dirty_rect.unwrap_or(self.flattened_dirty_rect.unwrap()), self.flattened_dirty_rect.unwrap()));
    }
    pub(crate) fn dirtify_edited(&mut self)
    {
        if self.edited_dirty_rect.is_some()
        {
            self.dirtify_rect(self.edited_dirty_rect.unwrap());
        }
        self.edited_dirty_rect = None;
    }
    pub(crate) fn dirtify_full_rect(&mut self)
    {
        self.flattened_dirty_rect = match &self.data
        {
            Some(image) =>
            {
                Some([[0.0, 0.0], [image.width as f32, image.height as f32]])
            }
            _ => Some([[0.0, 0.0], [1000000.0, 1000000.0]]) // FIXME store child sizes
        };
        let mut biggen = 0.0;
        for _fx in &self.effects
        {
            biggen += 3.0; // FIXME
        }
        if biggen != 0.0
        {
            *self.flattened_dirty_rect.as_mut().unwrap() = rect_grow(self.flattened_dirty_rect.unwrap(), 3.0);
        }
    }
    pub(crate) fn dirtify_point(&mut self, point : [f32; 2])
    {
        self.dirtify_rect([point, point]);
    }
    pub(crate) fn dirtify_all(&mut self)
    {
        let mut reference = None;
        // FIXME cache somehow??? or is it not worth it
        self.visit_layers(0, &mut |layer, _|
        {
            if let Some(image) = &layer.data
            {
                let rect = [layer.offset, vec_add(&layer.offset, &[image.width as f32, image.height as f32])];
                if reference.is_some()
                {
                    reference = Some(rect_enclose_rect(reference.unwrap(), rect));
                }
                else
                {
                    reference = Some(rect);
                }
            }
            Some(())
        });
        if self.adjustment.is_some()
        {
            reference = Some([[0.0, 0.0], [1000000.0, 1000000.0]]);
        }
        if let Some(x) = reference
        {
            self.dirtify_rect(x);
        }
    }
    pub(crate) fn would_override(&mut self, override_uuid : Option<u128>, override_data : Option<&Image<4>>) -> bool
    {
        Some(self.uuid) == override_uuid && override_data.is_some()
    }
    pub(crate) fn flatten<'a, 'b>(&'a mut self, canvas_width : usize, canvas_height : usize, override_uuid : Option<u128>, override_data : Option<&'b Image<4>>) -> &'b Image<4> where 'a: 'b
    {
        #[allow(clippy::unnecessary_unwrap)] // broken lint
        if self.would_override(override_uuid, override_data)
        {
            // FIXME use different dirty rects for override and non-override
            // and detect switching between override and non-override mode
            // and use both rects (enclosure) when indeed switching
            self.flattened_dirty_rect = None;
            override_data.unwrap()
        }
        else
        {
            self.flatten_as_root(canvas_width, canvas_height, override_uuid, override_data)
        }
    }
    pub(crate) fn flatten_as_root<'a>(&'a mut self, canvas_width : usize, canvas_height : usize, override_uuid : Option<u128>, override_data : Option<&Image<4>>) -> &'a Image<4>
    {
        if self.adjustment.is_some()
        {
            if self.flattened_data.is_none()
            {
                self.flattened_data = Some(Image::blank(1, 1));
            }
            return self.flattened_data.as_ref().unwrap();
        }
        
        let dirty_rect = self.get_flatten_dirty_rect();
        if dirty_rect.is_none() && self.flattened_data.is_some()
        //if self.flattened_data.is_none() && self.flattened_data.is_some()
        {
            return self.flattened_data.as_ref().unwrap();
        }
        else if let Some(image) = &self.data
        {
            self.flattened_dirty_rect = None;
            return image;
        }
        else
        {
            //println!("group is dirty, reflattening ({:?})", dirty_rect);
            let mut new_dirty_rect;
            
            #[allow(clippy::unnecessary_unwrap)] // broken lint
            if self.flattened_data.is_none() || dirty_rect.is_none()
            {
                //println!("asdf");
                new_dirty_rect = [[0.0, 0.0], [canvas_width as f32, canvas_height as f32]];
                self.flattened_data = Some(Image::blank(canvas_width, canvas_height));
            }
            else
            {
                //new_dirty_rect = [[0.0, 0.0], [canvas_width as f32, canvas_height as f32]];
                new_dirty_rect = dirty_rect.unwrap();
                //println!("clearing rect {:?} (layer {})...", new_dirty_rect, self.name);
                self.flattened_data.as_mut().unwrap().clear_rect_with_color_float(new_dirty_rect, [0.0, 0.0, 0.0, 0.0]);
            }
            // We keep track of what's "first" (bottommost) in a given group to give it a special blend mode against the empty flattening target layer.
            // This makes it so that "reveal" etc blend modes work more intuitively instead of having to choose
            // between erased transparent data being lost or fully transparent higher layers overwriting fully transparent lower layers.
            let mut first = true;
            let mut _stash_is_first = false;
            let mut stash = None;
            let mut stash_offs = [0, 0];
            let mut stash_clean = None;
            let mut stash_mask = None;
            let mut stash_mask_info = None;
            let mut stash_opacity = 0.0;
            let mut stash_fill_opacity = 0.0;
            let mut stash_funny_flag = false;
            let mut stash_blend_mode = "".to_string();
            
            for i in (0..self.children.len()).rev()
            {
                let (a, b) = self.children.split_at_mut(i);
                let child = b.first_mut().unwrap();
                if !child.visible
                {
                    child.flatten(canvas_width, canvas_height, override_uuid, override_data);
                    continue;
                }
                let alen = a.len();
                let mut above = a.last_mut();
                let mut n = 0;
                while above.is_some() && !above.as_ref().unwrap().visible && n + 1 < alen
                {
                    n += 1;
                    above = a.get_mut(alen - 1 - n);
                }
                let mut mode = child.blend_mode.clone();
                if mode == "Custom"
                {
                    mode += &("\n".to_string() + &child.custom_blend_mode);
                }
                let opacity = child.opacity;
                let fill_opacity = child.fill_opacity;
                let child_clipped = child.clipped;
                let child_funny_flag = child.funny_flag;
                let mut child_fx = child.effects.clone().into_iter().collect::<Vec<_>>();
                child_fx.sort_by_key(|a| 
                    match a.0.as_str()
                    {
                        "dropshadow" => 0,
                        "gradfill" => 1,
                        "colorfill" => 2,
                        "stroke" => 3,
                        _ => 0,
                    }
                );
                
                //println!("???{:?}", self.offset);
                let mut above_offset = [0, 0];
                if child.data.is_some()
                {
                    above_offset = [child.offset[0] as isize, child.offset[1] as isize];
                }
                
                //let source_data = child.flatten(canvas_width, canvas_height, override_uuid, override_data);
                child.flatten(canvas_width, canvas_height, override_uuid, override_data);
                let source_data = if child.would_override(override_uuid, override_data)
                {
                    override_data.unwrap()
                }
                else if child.flattened_data.is_some()
                {
                    child.flattened_data.as_ref().unwrap()
                }
                else
                {
                    child.data.as_ref().unwrap()
                };
                
                #[allow(clippy::unnecessary_unwrap)] // broken lint
                if above.is_some() && above.as_ref().unwrap().clipped && !child_clipped && !child.adjustment.is_some()
                {
                    // child is a clip target, get into clip target mode
                    // for color
                    stash = Some(source_data.clone());
                    stash_mask = child.mask.clone();
                    stash_mask_info = child.mask_info.clone();
                    stash_offs = above_offset;
                    // remove alpha
                    stash.as_mut().unwrap().clear_rect_alpha_float(new_dirty_rect, 1.0);
                    // for alpha, we restore the color bit's alpha with this later
                    stash_clean = Some(source_data.clone());
                    _stash_is_first = first;
                    stash_opacity = opacity;
                    stash_fill_opacity = fill_opacity;
                    stash_funny_flag = child_funny_flag;
                    stash_blend_mode = mode.clone();
                    
                    let mut rect = new_dirty_rect;
                    rect[0][0] -= above_offset[0] as f32;
                    rect[0][1] -= above_offset[1] as f32;
                    rect[1][0] -= above_offset[0] as f32;
                    rect[1][1] -= above_offset[1] as f32;
                    
                    // blend top into it
                    let above = above.unwrap();
                    above_offset[0] = above.offset[0] as isize - stash_offs[0];
                    above_offset[1] = above.offset[1] as isize - stash_offs[1];
                    let above_opacity = above.opacity;
                    let above_funny_flag = above.funny_flag;
                    let above_fill_opacity = above.fill_opacity;
                    let above_mode = &above.blend_mode.clone();
                    //let above_data = above.flatten(canvas_width, canvas_height, override_uuid, override_data);
                    
                    if let Some(adjustment) = &above.adjustment
                    {
                        stash.as_mut().unwrap().apply_adjustment(rect, &adjustment, above.mask.as_ref(), above.mask_info.as_ref(), above_opacity, above_fill_opacity, above_funny_flag, above_offset, above_mode);
                    }
                    else
                    {
                        above.flatten(canvas_width, canvas_height, override_uuid, override_data);
                        let above_data = if above.would_override(override_uuid, override_data)
                        {
                            override_data.unwrap()
                        }
                        else if above.flattened_data.is_some()
                        {
                            above.flattened_data.as_ref().unwrap()
                        }
                        else
                        {
                            above.data.as_ref().unwrap()
                        };
                        
                        stash.as_mut().unwrap().blend_rect_from(rect, above_data, above.mask.as_ref(), above.mask_info.as_ref(), above_opacity, above_fill_opacity, above_funny_flag, above_offset, above_mode);
                    }
                }
                else if stash.is_some() && (above.is_none() || !above.as_ref().unwrap().clipped)
                {
                    // done with the clipping mask sequence, blend into rest of group
                    let mut rect = new_dirty_rect;
                    rect[0][0] -= stash_offs[0] as f32;
                    rect[0][1] -= stash_offs[1] as f32;
                    rect[1][0] -= stash_offs[0] as f32;
                    rect[1][1] -= stash_offs[1] as f32;
                    
                    // restore original alpha
                    stash.as_mut().unwrap().blend_rect_from(rect, stash_clean.as_ref().unwrap(), None, None, stash_opacity, stash_fill_opacity, stash_funny_flag, [0, 0], "Clip Alpha");
                    //let s2 = stash.as_mut().unwrap().clone();
                    //stash.as_mut().unwrap().apply_fx_dummy_outline(rect, Some(s2).as_ref(), None, None, stash_opacity, stash_fill_opacity, stash_funny_flag, [0, 0], "Normal");
                    
                    above_offset = stash_offs;
                    
                    self.flattened_data.as_mut().unwrap().blend_rect_from(new_dirty_rect, stash.as_ref().unwrap(), stash_mask.as_ref(), stash_mask_info.as_ref(), stash_opacity, stash_fill_opacity, stash_funny_flag, above_offset, &stash_blend_mode);
                    
                    stash = None;
                    stash_clean = None;
                    stash_mask = None;
                    stash_mask_info = None;
                }
                else if let (Some(above), Some(ref mut stash)) = (above, stash.as_mut()) // above.is_some() is redundant with the above if branch, but left in for clarity
                {
                    // continuing a clip mask blend
                    let above_opacity = above.opacity;
                    let above_fill_opacity = above.fill_opacity;
                    let above_funny_flag = above.funny_flag;
                    above_offset[0] = above.offset[0] as isize - stash_offs[0];
                    above_offset[1] = above.offset[1] as isize - stash_offs[1];
                    let above_mode = &above.blend_mode.clone();
                    
                    let mut rect = new_dirty_rect;
                    rect[0][0] -= stash_offs[0] as f32;
                    rect[0][1] -= stash_offs[1] as f32;
                    rect[1][0] -= stash_offs[0] as f32;
                    rect[1][1] -= stash_offs[1] as f32;
                    
                    if let Some(adjustment) = &above.adjustment
                    {
                        stash.apply_adjustment(rect, &adjustment, above.mask.as_ref(), above.mask_info.as_ref(), above_opacity, above_fill_opacity, above_funny_flag, above_offset, above_mode);
                    }
                    else
                    {
                        above.flatten(canvas_width, canvas_height, override_uuid, override_data);
                        let above_data = if above.would_override(override_uuid, override_data)
                        {
                            override_data.unwrap()
                        }
                        else if above.flattened_data.is_some()
                        {
                            above.flattened_data.as_ref().unwrap()
                        }
                        else
                        {
                            above.data.as_ref().unwrap()
                        };
                        stash.blend_rect_from(rect, above_data, above.mask.as_ref(), above.mask_info.as_ref(), above_opacity, above_fill_opacity, above_funny_flag, above_offset, above_mode);
                    }
                }
                else
                {
                    if let Some(adjustment) = &child.adjustment
                    {
                        self.flattened_data.as_mut().unwrap().apply_adjustment(new_dirty_rect, &adjustment, child.mask.as_ref(), child.mask_info.as_ref(), opacity, fill_opacity, child.funny_flag, above_offset, &mode);
                    }
                    else
                    {
                        //println!("{}", child_fx.len());
                        println!("{:?}", child_fx);
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
                            let mut fill = self.flattened_data.clone().unwrap();
                            let mut dropshadow = None;
                            // CLONE
                            let mut fill_mask = fill.alike();
                            // CLONE
                            let mut full_mask = fill_mask.clone();
                            
                            let mut rect = new_dirty_rect;
                            
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
                                    new_dirty_rect = rect_grow(new_dirty_rect, r);
                                }
                                rect = rect_grow(rect, r);
                                
                                let rect_shifted = rect_translate(rect, [-above_offset[0] as f32, -above_offset[1] as f32]);
                                
                                // CLONE
                                let mut data = source_data.alike_grown(r_int as usize);
                                data.apply_fx(rect_shifted, &fx, Some(source_data), child.mask.as_ref(), child.mask_info.as_ref(), 1.0, 1.0, child.funny_flag, [r_int, r_int], "Normal");
                                fill.blend_rect_from(rect, &data, None, None, 1.0, fx_opacity, true, offset2, &fx_mode);
                                
                                // FIXME: use separate alpha and mask
                                full_mask.blend_rect_from(rect, &data, None, None, 1.0, 1.0, false, offset2, "Weld");
                                //fill.blend_rect_from(rect, &overlay, None, None, fx_opacity, 1.0, false, [0, 0], &weld_func);
                                
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
                                //if child.flattened_dirty_rect.is_some() || child.edited_dirty_rect.is_some() || self.flattened_data.is_none() || dirty_rect.is_none()
                                if child.flattened_dirty_rect.is_some() || child.edited_dirty_rect.is_some()
                                {
                                    new_dirty_rect = rect_grow(new_dirty_rect, r);
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
                                //let mut overlay = self.flattened_data.clone().unwrap();
                                //let mut overlay = if !fx_is_fill(&fx) { self.flattened_data.clone().unwrap() } else { fill.clone() };
                                let mut overlay = if !fx_is_fill(&fx) { if let Some(ds) = &dropshadow { ds.clone() } else { self.flattened_data.clone().unwrap() } } else { fill.clone() };
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
                                            d2.blend_rect_from(rect, &full_mask, None, None, 1.0, 1.0, false, [0, 0], "Clip Alpha");
                                            fill.blend_rect_from(rect, &d2, None, None, 1.0, 1.0, false, [0, 0], "Weld");
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
                                        d2.blend_rect_from(rect, &overlay, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
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
                                    ds.blend_rect_from(rect, &full_mask, None, None, 1.0, 1.0, false, [0, 0], "Erase");
                                    ds.blend_rect_from(rect, &full_mask, None, None, 1.0, 1.0, false, [0, 0], "Clip Alpha");
                                    fill.blend_rect_from(rect, &ds, None, None, 1.0, 1.0, false, [0, 0], "Weld");
                                }
                            }
                            
                            self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Alpha Antiblend");
                            self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Blend Weld");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Normal");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Alpha Antiblend");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Blend Weld");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Hard Interpolate");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Normal");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Interpolate");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Hard Interpolate");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Erase");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Sum Weld");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Weld");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Soft Weld");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Composite");
                            
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Normal");
                            
                            //fill.blend_rect_from(rect, &full_mask, None, None, 1.0, 1.0, false, [0, 0], "Merge Alpha");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Copy");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Weld");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Hard Weld");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Hard Interpolate");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Interpolate");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Normal");
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &fill, None, None, opacity, 1.0, false, [0, 0], "Composite");
                            
                            // for debugging
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(rect, &overlay, None, None, opacity, 1.0, false, [0, 0], "Hard Interpolate");
                            //*self.flattened_data.as_mut().unwrap() = overlay;
                            //*self.flattened_data.as_mut().unwrap() = overlay_mask;
                            //*self.flattened_data.as_mut().unwrap() = fill;
                            //*self.flattened_data.as_mut().unwrap() = masked_source;
                            //*self.flattened_data.as_mut().unwrap() = fill_mask;
                            //*self.flattened_data.as_mut().unwrap() = full_mask;
                        }
                        else
                        {
                            //let mut fill = self.flattened_data.clone().unwrap();
                            //fill.blend_rect_from(new_dirty_rect, source_data, None, None, 1.0, fill_opacity, child.funny_flag, above_offset, &mode);
                            //self.flattened_data.as_mut().unwrap().blend_rect_from(new_dirty_rect, &fill,
                            //    child.mask.as_ref(), child.mask_info.as_ref(), opacity, 1.0, false, [0, 0], "Composite");
                            
                            self.flattened_data.as_mut().unwrap().blend_rect_from(new_dirty_rect, source_data, child.mask.as_ref(), child.mask_info.as_ref(), opacity, fill_opacity, child.funny_flag, above_offset, &mode);
                        }
                    }
                }
                first = false;
            }
            self.flattened_dirty_rect = None;
            return self.flattened_data.as_ref().unwrap();
        }
    }
    pub(crate) fn flatten_get_cached(&self) -> Option<&Image<4>>
    {
        self.flattened_data.as_ref()
    }
    pub(crate) fn visit_layers(&self, depth : usize, f : &mut dyn FnMut(&Layer, usize) -> Option<()>) -> Option<()>
    {
        f(self, depth)?;
        for child in self.children.iter()
        {
            child.visit_layers(depth+1, f)?;
        }
        Some(())
    }
    pub(crate) fn visit_layers_mut(&mut self, depth : usize, f : &mut dyn FnMut(&mut Layer, usize) -> Option<()>) -> Option<()>
    {
        f(self, depth)?;
        for child in self.children.iter_mut()
        {
            child.visit_layers_mut(depth+1, f)?;
        }
        Some(())
    }
    pub(crate) fn visit_layer_parent(&self, find_uuid : u128, f : &mut dyn FnMut(&Layer, usize)) -> Option<()>
    {
        for i in 0..self.children.len()
        {
            if self.children[i].uuid == find_uuid
            {
                f(self, i);
                return None;
            }
            else if self.children[i].visit_layer_parent(find_uuid, f).is_none()
            {
                return None;
            }
        }
        Some(())
    }
    pub(crate) fn visit_layer_parent_mut(&mut self, find_uuid : u128, f : &mut dyn FnMut(&mut Layer, usize)) -> Option<()>
    {
        for i in 0..self.children.len()
        {
            if self.children[i].uuid == find_uuid
            {
                f(self, i);
                return None;
            }
            else if self.children[i].visit_layer_parent_mut(find_uuid, f).is_none()
            {
                return None;
            }
        }
        Some(())
    }
    pub (crate) fn count(&self) -> usize
    {
        let mut n = 0;
        self.visit_layers(0, &mut |_layer, _|
        {
            n += 1;
            Some(())
        });
        n
    }
    pub (crate) fn count_drawable(&self) -> usize
    {
        let mut n = 0;
        self.visit_layers(0, &mut |_layer, _|
        {
            if self.data.is_some()
            {
                n += 1;
            }
            Some(())
        });
        n
    }
    // finds the uuid of the layer or group before the given layer in the hierarchy, excluding self
    // parents are considered to be before their children
    pub (crate) fn uuid_of_prev(&self, find_uuid : u128) -> Option<u128>
    {
        let mut prev_uuid = 0;
        let mut found = false;
        self.visit_layers(0, &mut |layer, _|
        {
            if layer.uuid == find_uuid
            {
                found = true;
                return None;
            }
            prev_uuid = layer.uuid;
            Some(())
        });
        if found && prev_uuid != self.uuid
        {
            return Some(prev_uuid);
        }
        None
    }
    // finds the uuid of the layer or group after the given layer in the hierarchy, including children
    // parents are considered to be before their children
    pub (crate) fn uuid_of_next(&self, find_uuid : u128) -> Option<u128>
    {
        let mut prev_uuid = 0;
        let mut next_uuid = 0;
        let mut found = false;
        self.visit_layers(0, &mut |layer, _|
        {
            if prev_uuid == find_uuid
            {
                next_uuid = layer.uuid;
                found = true;
                return None;
            }
            prev_uuid = layer.uuid;
            Some(())
        });
        if found
        {
            return Some(next_uuid);
        }
        None
    }
    // deletes the given layer if it exists
    pub (crate) fn delete_layer(&mut self, find_uuid : u128)
    {
        // FIXME change to use visit_layer_parent_mut
        self.visit_layers_mut(0, &mut |layer, _|
        {
            let old_len = layer.children.len();
            layer.children.retain(|layer| layer.uuid != find_uuid);
            let new_len = layer.children.len();
            if new_len != old_len
            {
                layer.dirtify_full_rect();
                None
            }
            else
            {
                Some(())
            }
        });
    }
    pub (crate) fn move_layer_up(&mut self, find_uuid : u128) -> Option<Layer>
    {
        for i in 0..self.children.len()
        {
            if self.children[i].uuid == find_uuid
            {
                self.dirtify_full_rect();
                if i == 0
                {
                    if self.uuid != 0
                    {
                        return Some(self.children.remove(i));
                    }
                    else
                    {
                        break;
                    }
                }
                else
                {
                    let layer = self.children.remove(i);
                    if self.children[i-1].data.is_some()
                    {
                        // target is a layer, insert next to it
                        self.children.insert(i-1, layer);
                    }
                    else
                    {
                        // target is a group, insert into it
                        self.children[i-1].children.push(layer);
                    }
                    self.children[i-1].dirtify_full_rect();
                    break;
                }
            }
            else if let Some(layer) = self.children[i].move_layer_up(find_uuid)
            {
                self.children[i].dirtify_full_rect();
                self.dirtify_full_rect();
                
                self.children.insert(i, layer);
                break;
            }
        }
        None
    }
    pub (crate) fn move_layer_down(&mut self, find_uuid : u128) -> Option<Layer>
    {
        for i in 0..self.children.len()
        {
            if self.children[i].uuid == find_uuid
            {
                self.dirtify_full_rect();
                if i+1 >= self.children.len()
                {
                    if self.uuid != 0
                    {
                        return Some(self.children.remove(i));
                    }
                    else
                    {
                        break;
                    }
                }
                else
                {
                    let layer = self.children.remove(i);
                    if self.children[i].data.is_some()
                    {
                        // target is a layer, insert next to it
                        self.children.insert(i+1, layer);
                    }
                    else
                    {
                        // target is a group, insert into it
                        self.children[i].children.insert(0, layer);
                    }
                    self.children[i].dirtify_full_rect();
                    break;
                }
            }
            else if let Some(layer) = self.children[i].move_layer_down(find_uuid)
            {
                self.children[i].dirtify_full_rect();
                self.dirtify_full_rect();
                
                self.children.insert(i+1, layer);
                break;
            }
        }
        None
    }
    pub (crate) fn add_group(&mut self, find_uuid : u128)
    {
        self.visit_layer_parent_mut(find_uuid, &mut |parent, i|
        {
            parent.dirtify_all();
            parent.children.insert(i, Layer::new_group("New Group"));
        });
    }
    pub (crate) fn move_into_new_group(&mut self, find_uuid : u128)
    {
        self.visit_layer_parent_mut(find_uuid, &mut |parent, i|
        {
            parent.dirtify_all();
            let layer = parent.children.remove(i);
            let mut group = Layer::new_group("New Group");
            group.children.insert(0, layer);
            parent.children.insert(i, group);
        });
    }
}
