//! Typed commands shared by the dev panel, API and gameplay messages.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PowerCommand {
    pub command_type: PowerCommandType,
    pub username: String, // Twitch/YouTube viewer username
    pub launcher: LauncherOptions,
    /// Play style requested with STYLE or VILLAGERS
    pub style: Option<crate::components::play_style::PlayStyle>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LaunchDirection {
    #[default]
    Auto,
    Left,
    Right,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LauncherOptions {
    pub direction: LaunchDirection,
    pub villagers: u8,
}
impl Default for LauncherOptions {
    fn default() -> Self {
        Self {
            direction: LaunchDirection::Auto,
            villagers: 1,
        }
    }
}

impl PowerCommand {
    pub fn new(command_type: PowerCommandType, username: String) -> Self {
        Self {
            command_type,
            username,
            launcher: LauncherOptions::default(),
            style: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PowerCommandType {
    WaterBucket,
    LavaBucket,
    Teleport, // Safe travel to the next biome
    Launcher,
    Tnt,
    Fast,
    Slow,
    Fuss,
    Bolt,
    Arrow,
    Laser,
    Ghast,
    Box,
    Magnet,
    Villagers,  // Spawn 5 villagers around hero
    Mirror,     // Magic barrier that reflects projectiles
    Dragon,     // Spawn a dragon boss enemy
    Thunder,    // Spawn multiple lightning strikes that damage random nearby blocks
    Lootbox,    // Spawn a lootbox near hero
    RandomBox,  // Spawn a bouncing randombox that triggers random events
    GrowBuff,   // Apply Grow buff to hero (20% bigger per stack, max 2)
    LaserBurst, // Spawn 19 small white laser beams in all directions
    Style,      // Set the sender's heads to a play style (bouncy, heavy, floaty, sticky)
    Feather,    // Low gravity for everyone for 8 s
    Anvil,      // Anvils fall around the hero and smash down through blocks
    Golem,      // Friendly iron golem that fights enemies for 30 s
    Pearl,      // Ender pearl: hero jumps to a random free spot nearby
    Quake,      // Earthquake: ores pop out as drops, blocks crack, heads jolt
    Shrink,     // Friendly heads half size for 10 s (no block damage)
    Wind,       // Gust pushes every head sideways for 3 s
    BlackHole,  // Vortex pulls things in for 10 s, then pops them outward
    IceAge,     // Snow and slippery floors for 15 s
    Bees,       // Swarm chases a random head and stings lightly
    Prison,     // Locked in a spinning zero-gravity space prison until you escape
}
