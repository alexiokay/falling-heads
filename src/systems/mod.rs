// pub mod physics;  // Removed - physics systems now in physics_plugin.rs
pub mod combat;
pub mod gameplay;
pub mod powers;
pub mod rendering;
// Organized folders
pub mod audio;
pub mod buffs;
pub mod progression;
pub mod ui;
pub mod weather;

// Core systems
pub mod atlas_generator;
pub mod background_system;
pub mod biome_music_system;
pub mod block_damage_overlay_system;
pub mod cobweb_system;
pub mod collision_debug_system;
pub mod damage_visual_system;
pub mod death_particle_system;
pub mod enemy_ai_system;
pub mod entity_spawn;
pub mod health_system;
pub mod lootbox_system;
pub mod pause_menu_system;
pub mod performance_diagnostics;
pub mod projectile_system;
pub mod random_events;
pub mod randombox_system;
pub mod settings_menu_system;
pub mod world_generation;

pub mod biome_teleport;

pub mod energy_vfx;

pub mod mirror_vfx;

pub mod drop_terrain;

pub mod crowd_slide;

pub mod teleport_vfx;

pub mod fluids;

pub mod world_portals;

pub mod biome_climate;
pub mod graphics;

pub mod dragon_flyover;

pub mod native_gpu;

pub mod biome_encounters;
pub mod biome_mist;
pub mod forest_trees;
pub mod launcher;
pub mod rocket_audio;
pub mod spiders;
pub mod terrain_cracks;

#[cfg(target_os = "windows")]
pub mod frame_generation;

pub mod bats;

pub(crate) mod head_geometry;

pub(crate) mod world_shapes;

pub(crate) mod actor_depth;

pub(crate) mod target_index;

pub(crate) mod biome_content;

pub(crate) mod plant_rules;
pub(crate) mod slime_physics;

pub(crate) mod asset_packs;
pub(crate) mod character_select;
pub(crate) mod combos;
pub(crate) mod extra_powers;
pub(crate) mod game_modes;
pub(crate) mod play_styles;
pub(crate) mod power_cooldowns;
pub(crate) mod prison;
pub(crate) mod terrain_shape;
pub(crate) mod window_icon;
