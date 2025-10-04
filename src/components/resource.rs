use bevy::prelude::*;

#[derive(Component)]
pub struct Resource {
    pub resource_type: ResourceType,
}

/// Component for resources that rotate on the X axis
#[derive(Component)]
pub struct RotatingResource {
    pub rotation_speed: f32, // Degrees per second
}

impl Default for RotatingResource {
    fn default() -> Self {
        Self {
            rotation_speed: 180.0, // 180 degrees per second
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Coal,
    Iron,
    Gold,
    Diamond,
    Emerald,
    Redstone,
}
