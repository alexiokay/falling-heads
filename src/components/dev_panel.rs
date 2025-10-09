use bevy::prelude::*;

#[derive(Component)]
pub struct DevPanelRoot;

#[derive(Component)]
pub struct WeatherControlButton {
    pub weather_type: crate::components::weather::WeatherType,
}

#[derive(Component)]
pub struct PowerButton {
    pub power_type: crate::resources::command_queue::PowerCommandType,
}

#[derive(Component)]
pub struct BiomeTeleportButton {
    pub biome_type: crate::components::block::BiomeType,
}

#[derive(Component)]
pub struct ParameterLabel;
