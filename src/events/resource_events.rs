use crate::components::resource::ResourceType;
use bevy::prelude::*;

#[derive(Message)]
pub struct ResourceCollected {
    pub resource_type: ResourceType,
    pub amount: u32,
}
