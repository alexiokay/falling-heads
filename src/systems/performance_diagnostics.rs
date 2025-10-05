use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::components::block::Block;
use crate::components::enemy::Enemy;
use crate::components::background::Background;
use crate::components::resource::Resource;
use crate::components::health_ui::HealthBar;

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
    blocks: Query<&Block>,
    enemies: Query<&Enemy>,
    backgrounds: Query<&Background>,
    resources: Query<&Resource>,
    health_bars: Query<&HealthBar>,
    all_entities: Query<Entity>,
    all_transforms: Query<&Transform>,
    all_mesh3d: Query<&Mesh3d>,
    materials: Res<Assets<StandardMaterial>>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
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

        // Count entities
        let block_count = blocks.iter().count();
        let enemy_count = enemies.iter().count();
        let background_count = backgrounds.iter().count();
        let resource_count = resources.iter().count();
        let healthbar_count = health_bars.iter().count();
        let total_entities = all_entities.iter().count();
        let _transform_count = all_transforms.iter().count();
        let _mesh_entities = all_mesh3d.iter().count();

        let known_entities = block_count + enemy_count + background_count + resource_count + healthbar_count;
        let unknown_entities = total_entities.saturating_sub(known_entities);

        // Count assets
        let material_count = materials.iter().count();
        let mesh_count = meshes.iter().count();
        let _image_count = images.iter().count();

        info!("========== PERFORMANCE METRICS (Runtime: {:.1}s) ==========", runtime);
        info!("  FPS: {:.1} | Frame Time: {:.2}ms", fps, frame_time);
        info!("  Entities: {} blocks, {} enemies, {} resources, {} healthbars",
              block_count, enemy_count, resource_count, healthbar_count);
        info!("  ALL Entities: {} total ({} UNKNOWN/LEAK!)", total_entities, unknown_entities);
        info!("  Assets: {} materials, {} mesh assets",
              material_count, mesh_count);
        info!("==========================================================");
    }
}
