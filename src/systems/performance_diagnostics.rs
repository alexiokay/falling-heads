use crate::components::background::Background;
use crate::components::block::Block;
use crate::components::chunk::Chunk;
use crate::components::damage_overlay::BlockDamageOverlay;
use crate::components::enemy::Enemy;
use crate::components::explosive::Tnt;
use crate::components::hero::Hero;
use crate::components::projectile::Arrow;
use crate::components::resource::Resource;
use crate::components::villager::Villager;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

#[derive(Resource)]
pub struct PerformanceTracker {
    pub last_log_time: f32,
    pub log_interval: f32,
    pub start_time: f32,
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self {
            last_log_time: 0.0,
            log_interval: 2.0, // Log every 2 seconds for detailed analysis
            start_time: 0.0,
        }
    }
}

pub fn log_performance_metrics(
    time: Res<Time>,
    mut tracker: ResMut<PerformanceTracker>,
    diagnostics: Res<DiagnosticsStore>,
    // Main gameplay entities
    blocks: Query<&Block>,
    enemies: Query<&Enemy>,
    villagers: Query<&Villager>,
    heroes: Query<&Hero>,
    // World entities (use ParamSet for arrows, tnt, chunks)
    mut world_entities: ParamSet<(Query<&Arrow>, Query<&Tnt>, Query<&Chunk>)>,
    // UI entities
    backgrounds: Query<&Background>,
    resources: Query<&Resource>,
    overlays: Query<&BlockDamageOverlay>,
    // All entities for total count
    all_entities: Query<Entity>,
    materials: Res<Assets<StandardMaterial>>,
    meshes: Res<Assets<Mesh>>,
) {
    let elapsed = time.elapsed_secs();

    if tracker.start_time == 0.0 {
        tracker.start_time = elapsed;
    }

    if elapsed - tracker.last_log_time >= tracker.log_interval {
        tracker.last_log_time = elapsed;

        let runtime = elapsed - tracker.start_time;

        // Get FPS
        let fps = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|fps| fps.smoothed())
            .unwrap_or(0.0);

        // Get frame time
        let frame_time = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
            .and_then(|ft| ft.smoothed())
            .map(|ft| ft * 1000.0) // Convert to ms
            .unwrap_or(0.0);

        // Count main entities
        let block_count = blocks.iter().count();
        let enemy_count = enemies.iter().count();
        let villager_count = villagers.iter().count();
        let hero_count = heroes.iter().count();

        // Count world entities from ParamSet
        let arrow_count = world_entities.p0().iter().count();
        let tnt_count = world_entities.p1().iter().count();
        let chunk_count = world_entities.p2().iter().count();

        // Count UI entities
        let background_count = backgrounds.iter().count();
        let resource_count = resources.iter().count();
        let overlay_count = overlays.iter().count();

        let total_entities = all_entities.iter().count();

        let known_entities = block_count
            + enemy_count
            + villager_count
            + hero_count
            + arrow_count
            + tnt_count
            + chunk_count
            + background_count
            + resource_count
            + overlay_count;
        let unknown_entities = total_entities.saturating_sub(known_entities);

        // Count assets
        let material_count = materials.iter().count();
        let mesh_count = meshes.iter().count();

        info!(
            "========== PERFORMANCE METRICS (Runtime: {:.1}s) ==========",
            runtime
        );
        info!("  FPS: {:.1} | Frame Time: {:.2}ms", fps, frame_time);
        info!(
            "  Entities: {} blocks, {} overlays, {} enemies, {} villagers, {} hero",
            block_count, overlay_count, enemy_count, villager_count, hero_count
        );
        info!(
            "  World: {} chunks, {} arrows, {} tnt | {} resources, {} backgrounds",
            chunk_count, arrow_count, tnt_count, resource_count, background_count
        );
        info!(
            "  ALL Entities: {} total ({} UNKNOWN - healthbars/hearts/buffs/etc)",
            total_entities, unknown_entities
        );
        info!(
            "  Assets: {} materials, {} mesh assets",
            material_count, mesh_count
        );
        info!("==========================================================");
    }
}
