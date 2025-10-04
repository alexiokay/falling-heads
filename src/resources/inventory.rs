use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct Inventory {
    pub coal: u32,
    pub iron: u32,
    pub gold: u32,
    pub diamond: u32,
    pub emerald: u32,
    pub redstone: u32,
    pub netherite: u32,
}
