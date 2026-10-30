use bevy::prelude::*;

#[derive(Component)]
pub struct SettingsMenuRoot;

#[derive(Component)]
pub struct BackToMenuButton;

#[derive(Component)]
pub struct VsyncToggleButton;

#[derive(Component)]
pub struct FpsToggleButton;

#[derive(Component)]
pub struct DevModeToggleButton;

#[derive(Component)]
pub struct FpsLimitButton;

#[derive(Component)]
pub struct ToggleButtonText;

// Volume slider components
#[derive(Component)]
pub struct EffectsVolumeSlider;

#[derive(Component)]
pub struct MusicVolumeSlider;

#[derive(Component)]
pub struct VolumeSliderHandle;

#[derive(Component)]
pub struct VolumeSliderBar;

#[derive(Component)]
pub struct VolumeSliderLabel;

#[derive(Component)]
pub struct SettingsControl;
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Graphics,
    GameModes,
}
#[derive(Resource, Default)]
pub struct SettingsNavigation(pub SettingsPage);
#[derive(Component)]
pub struct SettingsPageRoot(pub SettingsPage);
#[derive(Component)]
pub struct SettingsTab(pub SettingsPage);
