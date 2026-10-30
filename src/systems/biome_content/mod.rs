//! Biome content uses the existing block families, damage events and actor physics.
//! The palette is separate from terrain topology so previews and streaming agree.
use bevy::prelude::*;
mod art;
pub(crate) mod catalog;
mod enemies;
mod interactions;
pub(crate) mod probe;
mod projectiles;
pub(crate) mod terrain;
#[cfg(test)]
mod tests;
pub(crate) use catalog::Tile;
pub(crate) use enemies::{BiomeFoe, spawn as spawn_specialist};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ContentArtReady;
pub struct BiomeContentPlugin;
impl Plugin for BiomeContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, art::setup)
            .add_systems(
                Update,
                (
                    art::animate_magma,
                    enemies::think.after(super::target_index::TargetsReady),
                    projectiles::advance,
                    interactions::broken,
                    super::plant_rules::maintain,
                    interactions::pickups,
                    interactions::hazards,
                    enemies::deaths,
                    enemies::hurt_audio,
                )
                    .chain()
                    .run_if(in_state(crate::states::InGame)),
            )
            .add_systems(
                PostUpdate,
                (art::finish, enemies::attach)
                    .chain()
                    .in_set(ContentArtReady)
                    .before(TransformSystems::Propagate)
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}
