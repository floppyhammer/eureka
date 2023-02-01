use std::collections::HashMap;

pub enum ResourceType {
    Texture,
    Font,
    Mesh,
    Material,
}

pub trait AsResource {
    fn get_type(&self) -> ResourceType;
}

pub struct ResourceId(u64);

pub struct ResourceManager {
    resources: HashMap<ResourceId, Box<dyn AsResource>>,
}
