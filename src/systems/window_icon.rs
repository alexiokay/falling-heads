//! Window and taskbar icon (`icon/falling-heads.png` from the theme pack, made by
//! `tools/Make-Icon.ps1`). Bevy has no icon setting, so it is applied to the
//! native window once the image has loaded.
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;

pub struct WindowIconPlugin;

impl Plugin for WindowIconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_icon)
            .add_systems(Update, apply_icon.run_if(resource_exists::<PendingIcon>));
    }
}

#[derive(Resource)]
struct PendingIcon(Handle<Image>);

fn load_icon(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(PendingIcon(server.load("icon/falling-heads.png")));
}

/// Runs on the main thread (winit window calls must).
fn apply_icon(
    mut commands: Commands,
    pending: Res<PendingIcon>,
    images: Res<Assets<Image>>,
    window: Query<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    let Some(image) = images.get(&pending.0) else {
        return;
    };
    let Ok(entity) = window.single() else {
        return;
    };
    let Some(rgba) = image.data.clone() else {
        commands.remove_resource::<PendingIcon>();
        return;
    };
    let size = image.size();
    let applied = WINIT_WINDOWS.with_borrow(|windows| {
        let Some(native) = windows.get_window(entity) else {
            return false;
        };
        match winit::window::Icon::from_rgba(rgba, size.x, size.y) {
            Ok(icon) => {
                native.set_window_icon(Some(icon));
                info!("Window icon set ({}x{})", size.x, size.y);
            }
            Err(e) => warn!("Window icon not applied: {e}"),
        }
        true
    });
    if applied {
        commands.remove_resource::<PendingIcon>();
    }
}
