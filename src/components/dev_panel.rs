use bevy::prelude::*;

#[derive(Component)]
pub struct DevPanelRoot;

#[derive(Component)]
pub struct WeatherControlButton {
    pub weather_type: crate::components::weather::WeatherType,
}

#[derive(Component)]
pub struct ParameterLabel;
