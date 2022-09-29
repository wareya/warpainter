
use crate::warimage::*;

use uuid::Uuid;

pub(crate) struct Layer
{
    pub(crate) name : String,
    pub(crate) blend_mode : String,
    
    pub(crate) data : Option<Image>,
    pub(crate) children : Vec<Layer>,
    
    pub(crate) flattened_data : Option<Image>,
    pub(crate) flattened_dirty : bool,
    
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
            
            flattened_data : None,
            flattened_dirty : true,
            
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
            
            flattened_data : None,
            flattened_dirty : true,
            
            uuid : Uuid::new_v4().as_u128(),
            
            offset : [0.0, 0.0],
            
            opacity : 1.0,
            visible : true,
            locked : false,
            clipped : false,
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
    pub(crate) fn flatten_is_dirty(&self, override_uuid : Option<u128>) -> bool
    {
        if self.flattened_dirty || Some(self.uuid) == override_uuid
        {
            return true;
        }
        else
        {
            for child in self.children.iter()
            {
                if child.flatten_is_dirty(override_uuid)
                {
                    return true;
                }
            }
        }
        false
    }
    pub(crate) fn flatten<'a, 'b>(&'a mut self, canvas_width : usize, canvas_height : usize, override_uuid : Option<u128>, override_data : Option<&'b Image>) -> &'b Image where 'a: 'b
    {
        if Some(self.uuid) == override_uuid && override_data.is_some()
        {
            return override_data.unwrap();
        }
        else
        {
            self.flatten_as_root(canvas_width, canvas_height, override_uuid, override_data)
        }
    }
    pub(crate) fn flatten_as_root<'a>(&'a mut self, canvas_width : usize, canvas_height : usize, override_uuid : Option<u128>, override_data : Option<&Image>) -> &'a Image
    {
        if !self.flatten_is_dirty(override_uuid) && self.flattened_data.is_some()
        {
            return self.flattened_data.as_ref().unwrap();
        }
        else if let Some(image) = &self.data
        {
            self.flattened_dirty = false;
            return image;
        }
        else
        {
            println!("layer group is dirty...reflattening");
            let mut image = Image::blank(canvas_width, canvas_height);
            for child in self.children.iter_mut().rev()
            {
                if child.visible
                {
                    let opacity = child.opacity;
                    image.blend_from(child.flatten(canvas_width, canvas_height, override_uuid, override_data), opacity);
                }
            }
            self.flattened_data = Some(image);
            self.flattened_dirty = false;
            return self.flattened_data.as_ref().unwrap();
        }
    }
    pub(crate) fn visit_layers(&self, depth : usize, f : &mut dyn FnMut(&Layer, usize) -> Option<()>) -> Option<()>
    {
        if f(self, depth).is_none()
        {
            return None;
        }
        for child in self.children.iter()
        {
            if child.visit_layers(depth+1, f).is_none()
            {
                return None;
            }
        }
        Some(())
    }
    pub(crate) fn visit_layers_mut(&mut self, depth : usize, f : &mut dyn FnMut(&mut Layer, usize) -> Option<()>) -> Option<()>
    {
        if f(self, depth).is_none()
        {
            return None;
        }
        for child in self.children.iter_mut()
        {
            if child.visit_layers_mut(depth+1, f).is_none()
            {
                return None;
            }
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
                layer.flattened_dirty = true;
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
                self.flattened_dirty = true;
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
                    break;
                }
            }
            else if let Some(layer) = self.children[i].move_layer_up(find_uuid)
            {
                self.children[i].flattened_dirty = true;
                self.flattened_dirty = true;
                
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
                self.flattened_dirty = true;
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
                    break;
                }
            }
            else if let Some(layer) = self.children[i].move_layer_down(find_uuid)
            {
                self.children[i].flattened_dirty = true;
                self.flattened_dirty = true;
                
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
            parent.flattened_dirty = true;
            parent.children.insert(i, Layer::new_group("New Group"));
        });
    }
    pub (crate) fn into_group(&mut self, find_uuid : u128)
    {
        self.visit_layer_parent_mut(find_uuid, &mut |parent, i|
        {
            parent.flattened_dirty = true;
            let layer = parent.children.remove(i);
            let mut group = Layer::new_group("New Group");
            group.children.insert(0, layer);
            parent.children.insert(i, group);
        });
    }
}
