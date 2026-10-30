use crate::components::block::BiomeType;
use bevy::prelude::*;
use std::collections::HashMap;

/// Component marking a background entity
#[derive(Component)]
pub struct Background {
    pub biome: BiomeType,
    /// Stable world-grid coordinate; independent of ECS iteration order.
    pub cell: IVec2,
}

/// Resource to cache background materials and meshes (prevents leak of 7k+ materials!)
#[derive(Resource, Default)]
pub struct BackgroundCache {
    pub materials: HashMap<BiomeType, Handle<StandardMaterial>>,
    pub mesh: Option<Handle<Mesh>>, // Single 64x64 mesh used by all backgrounds
}
