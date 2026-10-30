//! Startup loading screen. Until textures, materials and the chunks around the
//! hero exist (and the GPU has had a moment to build its shaders) the window
//! used to show only the grey clear colour; this covers it with a title and
//! progress, then fades out.
use crate::components::chunk::{LoadedChunks, world_to_chunk_coords};
use crate::components::hero::Hero;
use crate::resources::material_cache::MaterialCache;
use bevy::prelude::*;

/// Chunk radius the world generator loads around the hero (7x7 chunks).
const RADIUS: i32 = 3;
/// Extra time after the world is ready, so shaders finish compiling unseen.
const SETTLE_SECONDS: f32 = 0.8;
const FADE_SECONDS: f32 = 0.35;
/// Never block the game longer than this, whatever happens.
const MAX_SECONDS: f32 = 20.0;

pub struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        // Test and probe runs take their own screenshots; keep them unchanged.
        if !crate::resources::settings::persistence_enabled() {
            return;
        }
        app.add_systems(Startup, spawn_loading_screen)
            .add_systems(Update, update_loading_screen);
    }
}

/// Present while the loading screen is up: game sounds and music stay silent.
#[derive(Resource)]
pub struct LoadingActive;

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct LoadingText;

#[derive(Default)]
struct Progress {
    ready_at: Option<f32>,
}

fn spawn_loading_screen(mut commands: Commands) {
    commands.insert_resource(LoadingActive);
    commands
        .spawn((
            LoadingScreen,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.045, 0.06)),
            GlobalZIndex(1000),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("MINEHEADS"),
                TextFont {
                    font_size: FontSize::Px(52.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.85, 0.45)),
            ));
            p.spawn((
                LoadingText,
                Text::new("Loading textures..."),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.78, 0.8)),
            ));
        });
}

/// Share of the chunks around the hero that are loaded (0..1).
fn world_progress(chunks: &LoadedChunks, hero: Vec3) -> f32 {
    let (cx, cy) = world_to_chunk_coords(hero.x, hero.y, 64.0);
    let mut loaded = 0;
    for dx in -RADIUS..=RADIUS {
        for dy in -RADIUS..=RADIUS {
            if chunks.is_loaded(cx + dx, cy + dy) {
                loaded += 1;
            }
        }
    }
    loaded as f32 / ((2 * RADIUS + 1) * (2 * RADIUS + 1)) as f32
}

#[allow(clippy::too_many_arguments)] // Bevy system parameters
fn update_loading_screen(
    mut commands: Commands,
    time: Res<Time<Real>>,
    materials: Res<MaterialCache>,
    chunks: Option<Res<LoadedChunks>>,
    hero: Query<&Transform, With<Hero>>,
    mut screens: Query<(Entity, &mut BackgroundColor), With<LoadingScreen>>,
    mut texts: Query<(&mut Text, &mut TextColor), With<LoadingText>>,
    mut progress: Local<Progress>,
) {
    let Ok((screen, mut background)) = screens.single_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    let world = match (chunks.as_deref(), hero.single()) {
        (Some(chunks), Ok(hero)) if !materials.materials.is_empty() => {
            world_progress(chunks, hero.translation)
        }
        _ => 0.0,
    };
    let message = if materials.materials.is_empty() {
        "Loading textures...".to_string()
    } else if world < 1.0 {
        format!("Building the world... {:.0}%", world * 100.0)
    } else {
        "Starting...".to_string()
    };
    if world >= 1.0 && progress.ready_at.is_none() {
        progress.ready_at = Some(now);
        info!("Loading screen: world ready after {now:.1}s");
    }
    let fade = progress.ready_at.map_or(0.0, |at| {
        ((now - at - SETTLE_SECONDS) / FADE_SECONDS).clamp(0.0, 1.0)
    });
    if fade >= 1.0 || now > MAX_SECONDS {
        commands.entity(screen).despawn();
        commands.remove_resource::<LoadingActive>();
        return;
    }
    background.0.set_alpha(1.0 - fade);
    for (mut text, mut color) in &mut texts {
        if text.0 != message {
            text.0 = message.clone();
        }
        color.0.set_alpha(1.0 - fade);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_counts_the_chunks_around_the_hero() {
        let mut chunks = LoadedChunks::default();
        assert_eq!(world_progress(&chunks, Vec3::ZERO), 0.0);
        for x in -RADIUS..=RADIUS {
            for y in -RADIUS..=RADIUS {
                chunks.add(x, y);
            }
        }
        assert_eq!(world_progress(&chunks, Vec3::ZERO), 1.0);
        // Far away from the loaded area nothing counts.
        assert_eq!(world_progress(&chunks, Vec3::new(1.0e6, 0.0, 0.0)), 0.0);
    }
}
