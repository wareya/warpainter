use crate::warimage::*;
use uuid::Uuid;

fn rect_enclose_point(mut rect : [[f32; 2]; 2], point : [f32; 2]) -> [[f32; 2]; 2]
{
    rect[0][0] = rect[0][0].min(point[0]);
    rect[0][1] = rect[0][1].min(point[1]);
    rect[1][0] = rect[1][0].max(point[0]);
    rect[1][1] = rect[1][1].max(point[1]);
    rect
}
fn rect_enclose_rect(mut rect : [[f32; 2]; 2], rect_2 : [[f32; 2]; 2]) -> [[f32; 2]; 2]
{
    rect = rect_enclose_point(rect, rect_2[0]);
    rect = rect_enclose_point(rect, rect_2[1]);
    rect
}
fn rect_normalize(rect : [[f32; 2]; 2]) -> [[f32; 2]; 2]
{
    rect_enclose_point([rect[0], rect[0]], rect[1])
}

pub(crate) struct Layer
{
    pub(crate) name : String,
    pub(crate) blend_mode : String,
    
    pub(crate) data : Option<Image>,
    pub(crate) children : Vec<Layer>,
    
    pub(crate) flattened_data : Option<Image>,
    pub(crate) flattened_dirty_rect : Option<[[f32; 2]; 2]>,
    
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
            flattened_dirty_rect : None,
            
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
            flattened_dirty_rect : None,
            
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
    }
    pub(crate) fn dirtify_point(&mut self, point : [f32; 2])
    {
        self.dirtify_rect([point, point]);
    }
    pub(crate) fn dirtify_all(&mut self)
    {
        let mut size = [0.0f32, 0.0f32];
        // FIXME cache somehow??? or is it not worth it
        self.visit_layers(0, &mut |layer, _|
        {
            match &layer.data
            {
                Some(image) =>
                {
                    size[0] = size[0].max(image.width as f32);
                    size[1] = size[1].max(image.height as f32);
                }
                _ => {}
            }
            Some(())
        });
        self.dirtify_rect([[0.0, 0.0], size]);
    }
    pub(crate) fn flatten<'a, 'b>(&'a mut self, canvas_width : usize, canvas_height : usize, override_uuid : Option<u128>, override_data : Option<&'b Image>) -> &'b Image where 'a: 'b
    {
        if Some(self.uuid) == override_uuid && override_data.is_some()
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
    pub(crate) fn flatten_as_root<'a>(&'a mut self, canvas_width : usize, canvas_height : usize, override_uuid : Option<u128>, override_data : Option<&Image>) -> &'a Image
    {
        let dirty_rect = self.get_flatten_dirty_rect();
        if dirty_rect.is_none() && self.flattened_data.is_some()
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
            let new_dirty_rect;
            
            if self.flattened_data.is_none() || dirty_rect.is_none()
            {
                new_dirty_rect = [[0.0, 0.0], [canvas_width as f32, canvas_height as f32]];
                self.flattened_data = Some(Image::blank(canvas_width, canvas_height));
            }
            else
            {
                new_dirty_rect = dirty_rect.unwrap();
                self.flattened_data.as_mut().unwrap().clear_rect_with_color_float(new_dirty_rect, [0.0, 0.0, 0.0, 0.0]);
            }
            for child in self.children.iter_mut().rev()
            {
                if child.visible
                {
                    let mode = child.blend_mode.clone();
                    let opacity = child.opacity;
                    let source_data = child.flatten(canvas_width, canvas_height, override_uuid, override_data);
                    self.flattened_data.as_mut().unwrap().blend_rect_from(new_dirty_rect, source_data, opacity, &mode);
                }
            }
            self.flattened_dirty_rect = None;
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
                layer.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
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
                self.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
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
                    self.children[i-1].flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
                    break;
                }
            }
            else if let Some(layer) = self.children[i].move_layer_up(find_uuid)
            {
                self.children[i].flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
                self.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
                
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
                self.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
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
                    self.children[i].flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
                    break;
                }
            }
            else if let Some(layer) = self.children[i].move_layer_down(find_uuid)
            {
                self.children[i].flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
                self.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
                
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
            parent.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
            parent.children.insert(i, Layer::new_group("New Group"));
        });
    }
    pub (crate) fn into_group(&mut self, find_uuid : u128)
    {
        self.visit_layer_parent_mut(find_uuid, &mut |parent, i|
        {
            parent.flattened_dirty_rect = Some([[0.0, 0.0], [10000.0, 10000.0]]); // fixme store width/height/etc
            let layer = parent.children.remove(i);
            let mut group = Layer::new_group("New Group");
            group.children.insert(0, layer);
            parent.children.insert(i, group);
        });
    }
}
