use crate::components::block::BiomeType;
use bevy::prelude::*;

/// Resource tracking current playing biome music
#[derive(Resource)]
pub struct BiomeMusicPlayer {
    pub current_biome: Option<BiomeType>,
    pub music_entity: Option<Entity>,  // Current playing music
    pub fading_entity: Option<Entity>, // Old music that's fading out
    pub fade_timer: Option<Timer>,
}

impl Default for BiomeMusicPlayer {
    fn default() -> Self {
        Self {
            current_biome: None,
            music_entity: None,
            fading_entity: None,
            fade_timer: None,
        }
    }
}

/// Biome music tracks
#[derive(Resource)]
pub struct BiomeMusicTracks {
    pub plains: Handle<AudioSource>,
    pub forest: Handle<AudioSource>,
    pub desert: Handle<AudioSource>,
    pub mountains: Handle<AudioSource>,
    pub underground: Handle<AudioSource>,
    pub nether: Handle<AudioSource>,
    pub end: Handle<AudioSource>,
    pub iceland: Handle<AudioSource>,
}

impl BiomeMusicTracks {
    /// Get the track for a biome
    pub fn get_track(&self, biome: BiomeType) -> Handle<AudioSource> {
        match biome {
            BiomeType::Plains => self.plains.clone(),
            BiomeType::Forest => self.forest.clone(),
            BiomeType::Desert => self.desert.clone(),
            BiomeType::Mountains => self.mountains.clone(),
            BiomeType::Underground => self.underground.clone(),
            BiomeType::Nether => self.nether.clone(),
            BiomeType::End => self.end.clone(),
            BiomeType::Iceland => self.iceland.clone(),
        }
    }
}
