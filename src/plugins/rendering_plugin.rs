use bevy::prelude::*;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        info!("RenderingPlugin loaded - Bevy handles frustum culling automatically");
        // Note: Bevy's built-in rendering already does frustum culling
        // No additional systems needed
    }
}
