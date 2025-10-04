use bevy::prelude::*;
use crate::systems::physics::*;
use crate::resources::damage_cooldown::DamageCooldown;
use crate::states::GameState;

pub struct PhysicsPlugin;

/// System execution order for physics
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    Input,      // AI decisions, player input
    Forces,     // Apply gravity and forces
    Movement,   // Update positions based on velocity
    Collision,  // Detect collisions
    Response,   // Resolve collisions and apply effects
}

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<DamageCooldown>()
            // Configure execution order (guaranteed to run in this sequence)
            .configure_sets(
                Update,
                (
                    PhysicsSet::Input,
                    PhysicsSet::Forces,
                    PhysicsSet::Movement,
                    PhysicsSet::Collision,
                    PhysicsSet::Response,
                ).chain().run_if(in_state(GameState::Playing))
            )
            // Register systems in their appropriate sets
            .add_systems(
                Update,
                (
                    // Input/AI phase
                    enemy_ai_movement.in_set(PhysicsSet::Input),

                    // Forces phase
                    apply_gravity.in_set(PhysicsSet::Forces),

                    // Movement phase
                    apply_velocity.in_set(PhysicsSet::Movement),

                    // Collision detection phase
                    (
                        check_hero_block_collisions,
                        check_entity_collisions,
                    ).in_set(PhysicsSet::Collision),

                    // Response phase (handled by collision systems)
                )
            );

        info!("PhysicsPlugin loaded with System Sets");
    }
}
