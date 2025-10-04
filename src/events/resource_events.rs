use bevy::prelude::*;
use crate::components::resource::ResourceType;

#[derive(Message)]
pub struct ResourceCollected {
    pub resource_type: ResourceType,
    pub amount: u32,
}
