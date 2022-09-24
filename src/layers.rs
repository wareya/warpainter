
use crate::warimage::*;

use uuid::Uuid;

pub(crate) struct Layer
{
    pub(crate) name : String,
    pub(crate) blend_mode : String,
    
    pub(crate) data : Option<Image>,
    pub(crate) children : Vec<Layer>,
    
    pub(crate) uuid : u128,
    
    pub(crate) offset : [f32; 2],
    
    pub(crate) opacity : f32,
    pub(crate) visible : bool,
    pub(crate) locked : bool,
    pub(crate) clipped : bool,
}

impl Layer
{
    pub(crate) fn new_layer_from_image<T : ToString>(name : T, image : Image) -> Self
    {
        Layer {
            name : name.to_string(),
            blend_mode : "Normal".to_string(),
            
            data : Some(image),
            children : vec!(),
            
            uuid : Uuid::new_v4().as_u128(),
            
            offset : [0.0, 0.0],
            
            opacity : 1.0,
            visible : true,
            locked : false,
            clipped : false,
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
            
            data : None,
            children : vec!(),
            
            uuid : Uuid::new_v4().as_u128(),
            
            offset : [0.0, 0.0],
            
            opacity : 1.0,
            visible : true,
            locked : false,
            clipped : false,
        }
    }
    pub(crate) fn is_layer(&self) -> bool
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
    pub(crate) fn flatten(&self, canvas_width : usize, canvas_height : usize, override_uuid : u128, override_data : Option<&Image>) -> Image
    {
        if self.uuid == override_uuid
        {
            if let Some(data) = override_data
            {
                return data.clone();
            }
        }
        if let Some(image) = &self.data
        {
            image.clone()
        }
        else
        {
            let mut image = Image::blank(canvas_width, canvas_height);
            for child in self.children.iter().rev()
            {
                image.blend_from(&child.flatten(canvas_width, canvas_height, override_uuid, override_data));
            }
            image
        }
    }
    pub(crate) fn visit_layers(&self, depth : usize, f : &mut dyn FnMut(&Layer))
    {
        f(self);
        for child in self.children.iter()
        {
            child.visit_layers(depth+1, f);
        }
    }
}
