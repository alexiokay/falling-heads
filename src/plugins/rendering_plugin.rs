use bevy::prelude::*;
pub struct RenderingPlugin;
impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::systems::graphics::GraphicsPlugin);
    }
}
