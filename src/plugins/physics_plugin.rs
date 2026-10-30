//! Coordinates Update steering/combat and authoritative Avian terrain contacts.
use crate::components::block::{Block, BlockType};
use crate::components::buff::{Buff, BuffType};
use crate::components::chunk::ChunkMember;
use crate::components::enemy::Enemy;
use crate::components::explosive::Tnt;
use crate::components::hero::Hero;
use crate::components::snow_layer::SnowLayer;
use crate::components::snow_slide::{SnowSlide, Step as SlideStep, Surface as SlideSurface};
use crate::components::surface_grip::{SurfaceGrip, snow_carry};
use crate::components::villager::Villager;
use crate::events::block_events::BlockDamaged;
use crate::events::damage_events::EntityDamaged;
use crate::resources::damage_cooldown::{DamageCooldown, MeleeAttackCooldown};
use crate::resources::spatial_hash::SpatialHash;
use crate::systems::buffs::buff_core::apply_block_buff;
use crate::utils::constants::*;
use avian3d::prelude::*;
use bevy::prelude::*;
#[path = "physics/terrain.rs"]
mod terrain;
use terrain::*;
#[path = "physics/movement.rs"]
mod movement;
use movement::*;
#[path = "physics/combat.rs"]
mod combat;
use combat::*;
#[path = "physics/tnt.rs"]
mod tnt;
use tnt::*;
#[path = "physics/terrain_motion.rs"]
mod terrain_motion;
#[cfg(test)]
#[path = "physics/tests.rs"]
mod tests;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    AI,        // Enemy AI movement
    Collision, // Detect and resolve collisions
}

#[derive(Resource, Default)]
pub struct ContactTuning {
    pub break_through: bool,
}
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ContactTuning>()
            .init_resource::<DamageCooldown>()
            .init_resource::<MeleeAttackCooldown>()
            .insert_resource(SpatialHash::new(256.0)) // 256 units per spatial chunk (4 blocks)
            .configure_sets(
                Update,
                (PhysicsSet::AI, PhysicsSet::Collision)
                    .chain()
                    .run_if(in_state(crate::states::InGame)),
            )
            .add_systems(
                Update,
                (hero_ai_movement, villager_ai_movement, enemy_ai_movement)
                    .in_set(PhysicsSet::AI)
                    .after(crate::systems::target_index::TargetsReady),
            )
            // Keep spatial hash - blocks use custom spatial hash (faster than Avian for 15k+ static blocks)
            .add_systems(Update, rebuild_spatial_hash.in_set(PhysicsSet::Collision))
            .add_systems(
                Update,
                (
                    check_tnt_block_collisions,     // TNT needs block collision
                    handle_avian_collision_melee,   // Hero melee damage via Avian collision events
                    handle_enemy_damage_collisions, // Enemy damage to hero/villagers via Avian collision events
                    clamp_entity_velocities, // Prevent extreme velocities from cumulative collision forces
                )
                    .in_set(PhysicsSet::Collision)
                    .after(rebuild_spatial_hash),
            )
            .add_systems(
                Update,
                crate::resources::damage_cooldown::prune_cooldowns.run_if(
                    bevy::time::common_conditions::on_timer(std::time::Duration::from_secs(5)),
                ),
            );

        install_terrain_contacts(app);
        info!("PhysicsPlugin loaded");
    }
}
