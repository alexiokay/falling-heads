use bevy::prelude::*;
use bevy::animation::AnimationPlayer;
use crate::components::enemy::{Dragon, DragonAnimationClip};

/// Dragon animation system - plays the "fly" animation on dragons when they spawn
///
/// NOTE: Animation playback is temporarily disabled due to Bevy 0.17 API complexity.
/// The dragon will still fly and move, just without wing flapping animation.
/// TODO: Implement proper animation graph setup for Bevy 0.17
pub fn dragon_animation_system(
    mut commands: Commands,
    dragon_query: Query<(Entity, &DragonAnimationClip), (With<Dragon>, Without<AnimationPlaying>)>,
    children: Query<&Children>,
    animation_players: Query<&AnimationPlayer>,
) {
    for (dragon_entity, _animation_clip) in dragon_query.iter() {
        info!("🐉 Dragon spawned! Looking for AnimationPlayer in hierarchy...");

        // Search for AnimationPlayer in descendants (GLB scenes have it on child entities)
        let player_entity = find_descendant_with_player(dragon_entity, &children, &animation_players);

        if let Some(player_ent) = player_entity {
            info!("🐉 Found AnimationPlayer at entity {:?}", player_ent);

            // TODO: Implement animation playback with proper AnimationGraph setup
            // For now, just mark as processed
            commands.entity(dragon_entity).insert(AnimationPlaying);

            info!("🐉 Dragon ready (animation TODO)");
        } else {
            warn!("🐉 No AnimationPlayer found in dragon hierarchy");
            // Still mark as processed so we don't keep searching
            commands.entity(dragon_entity).insert(AnimationPlaying);
        }
    }
}

/// Marker component to indicate dragon animation system has processed this dragon
#[derive(Component)]
pub struct AnimationPlaying;

/// Recursively find entity with AnimationPlayer component in descendants
fn find_descendant_with_player(
    entity: Entity,
    children_query: &Query<&Children>,
    players: &Query<&AnimationPlayer>,
) -> Option<Entity> {
    if players.contains(entity) {
        return Some(entity);
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Some(found) = find_descendant_with_player(child, children_query, players) {
                return Some(found);
            }
        }
    }

    None
}
