use bevy::prelude::*;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        info!("RenderingPlugin loaded");
        // TODO: Add rendering systems
    }
}
