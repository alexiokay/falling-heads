use bevy::prelude::*;
use crate::components::enemy::{Health, Enemy};
use crate::components::hero::Hero;
use crate::components::health_ui::HealthBar;

/// Spawn health bars for entities with Health component
pub fn spawn_health_bars(
    mut commands: Commands,
    entities_with_health: Query<Entity, With<Health>>,
    existing_health_bars: Query<&HealthBar>,
    hero_query: Query<Entity, With<Hero>>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    // CRITICAL FIX: Build set of entities that already have health bars to prevent duplicates
    let entities_with_bars: std::collections::HashSet<Entity> = existing_health_bars
        .iter()
        .map(|bar| bar.target_entity)
        .collect();

    for entity in entities_with_health.iter() {
        // Skip if this entity already has a health bar
        if entities_with_bars.contains(&entity) {
            continue;
        }

        // Determine offset based on entity type
        let offset_y = if hero_query.contains(entity) {
            40.0  // Hero offset
        } else if enemy_query.contains(entity) {
            30.0  // Enemy offset
        } else {
            30.0  // Default offset
        };

        // Spawn a health bar entity
        commands.spawn((
            HealthBar {
                target_entity: entity,
                offset_y,
            },
            Text2d::default(),
            Transform::default(),
        ));
    }
}

/// Update health bar positions and text
/// PERFORMANCE: Runs every frame for position, but only updates text when health changes
pub fn update_health_bars(
    mut commands: Commands,
    mut health_bar_query: Query<(Entity, &HealthBar, &mut Transform, &mut Text2d)>,
    entity_query: Query<(&Transform, &Health), Without<HealthBar>>,
    changed_health: Query<&Health, Changed<Health>>,
) {
    for (bar_entity, health_bar, mut bar_transform, mut text) in health_bar_query.iter_mut() {
        // Check if target entity still exists
        if let Ok((target_transform, health)) = entity_query.get(health_bar.target_entity) {
            // Always update position (entities move)
            bar_transform.translation = Vec3::new(
                target_transform.translation.x,
                target_transform.translation.y + health_bar.offset_y,
                100.0,  // High Z to render on top
            );

            // Only update text when health changed (performance optimization)
            if changed_health.get(health_bar.target_entity).is_ok() {
                let hearts_str = "♥".repeat(health.current as usize);
                let empty_hearts_str = "♡".repeat((health.max - health.current) as usize);
                *text = Text2d::new(format!("{}{}", hearts_str, empty_hearts_str));
            }
        } else {
            // Target entity was despawned, remove health bar
            commands.entity(bar_entity).despawn();
        }
    }
}
