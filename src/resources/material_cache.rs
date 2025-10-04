use bevy::prelude::*;
use std::collections::HashMap;
use crate::components::block::BlockType;
use crate::components::resource::ResourceType;

/// Cache of shared materials for all block types and resources
#[derive(Resource)]
pub struct MaterialCache {
    pub materials: HashMap<BlockType, Handle<StandardMaterial>>,
    pub resource_materials: HashMap<ResourceType, Handle<StandardMaterial>>,
}

impl MaterialCache {
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            resource_materials: HashMap::new(),
        }
    }

    pub fn get(&self, block_type: BlockType) -> Option<Handle<StandardMaterial>> {
        self.materials.get(&block_type).cloned()
    }

    pub fn get_resource(&self, resource_type: ResourceType) -> Option<Handle<StandardMaterial>> {
        self.resource_materials.get(&resource_type).cloned()
    }
}

impl Default for MaterialCache {
    fn default() -> Self {
        Self::new()
    }
}
