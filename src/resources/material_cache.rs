use crate::components::block::BlockType;
use crate::components::resource::ResourceType;
use bevy::prelude::*;
use std::collections::HashMap;

/// Cache of shared materials for all block types and resources
#[derive(Resource)]
pub struct MaterialCache {
    pub materials: HashMap<BlockType, Handle<StandardMaterial>>,
    pub resource_materials: HashMap<ResourceType, Handle<StandardMaterial>>,
    pub tree_materials:
        HashMap<(crate::systems::forest_trees::TreeSpecies, bool), Handle<StandardMaterial>>,
    pub forest_floor: Option<Handle<StandardMaterial>>,
}

impl MaterialCache {
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            resource_materials: HashMap::new(),
            tree_materials: HashMap::new(),
            forest_floor: None,
        }
    }

    pub fn get(&self, block_type: BlockType) -> Option<Handle<StandardMaterial>> {
        self.materials.get(&block_type).cloned()
    }
}

impl Default for MaterialCache {
    fn default() -> Self {
        Self::new()
    }
}
