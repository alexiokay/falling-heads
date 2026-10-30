use bevy::prelude::*;

/// Lootbox that can be opened by the hero for random rewards
#[derive(Component, Debug)]
pub struct Lootbox {
    pub is_open: bool,
}

/// RandomBox - bouncing entity that triggers random events when touched by hero
#[derive(Component, Debug)]
pub struct RandomBox;

/// Marker component for lootbox UI
#[derive(Component)]
pub struct LootboxUI;

/// Component for the roulette spinner
#[derive(Component)]
pub struct RouletteSpinner {
    pub items: Vec<LootboxReward>,
    pub spin_speed: f32,
    pub current_offset: f32,
    pub is_spinning: bool,
    pub spin_timer: Timer,
    pub selected_index: usize,
    pub last_item_index: usize,     // Track last item for tick sound
    pub item_entities: Vec<Entity>, // Store item entities to query their positions
}

/// Marker component for roulette item
#[derive(Component)]
pub struct RouletteItem {
    pub index: usize,
}

impl RouletteSpinner {
    pub fn new_with_speed(
        items: Vec<LootboxReward>,
        selected_index: usize,
        spin_speed: f32,
    ) -> Self {
        Self {
            items,
            spin_speed,
            current_offset: 0.0,
            is_spinning: true,
            spin_timer: Timer::from_seconds(3.0, TimerMode::Once),
            selected_index,
            last_item_index: 0,        // Start at first item
            item_entities: Vec::new(), // Will be populated when spawning items
        }
    }
}

/// Possible rewards from lootbox
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LootboxReward {
    Teleport,
    MonsterHorde, // Spawns enemies (bad)
    // TreasureRain,      // REMOVED - Spawns resources (good)
    TravelersArrive, // Spawns villagers (good)
    EliteEnemy,      // Spawns elite enemy (bad)
    MegaTreasure,    // Extra resources (very good)
    MagicBarrier,    // Magic barrier that reflects projectiles (good)
    ArrowBarrage,    // Fires 48 arrows in all directions (good)
    ThunderStrike,   // Lightning strikes that damage blocks (neutral/bad)
    TntRain,         // Spawns TNT above hero (bad/chaotic)
    LaserBeam,       // Powerful laser beam (good)
    Magnet,          // Attracts nearby resources (good)
    Nothing,         // Nothing happens (neutral)
}

impl LootboxReward {
    /// Get display name for the reward
    pub fn name(&self) -> &str {
        match self {
            LootboxReward::Teleport => "Biome Portal",
            LootboxReward::MonsterHorde => "Monster Horde",
            // LootboxReward::TreasureRain => "Treasure Rain", // REMOVED
            LootboxReward::TravelersArrive => "Travelers Arrive",
            LootboxReward::EliteEnemy => "Elite Enemy",
            LootboxReward::MegaTreasure => "Mega Treasure",
            LootboxReward::MagicBarrier => "Magic Barrier",
            LootboxReward::ArrowBarrage => "Arrow Barrage",
            LootboxReward::ThunderStrike => "Thunder Strike",
            LootboxReward::TntRain => "TNT Rain",
            LootboxReward::LaserBeam => "Laser Beam",
            LootboxReward::Magnet => "Magnet",
            LootboxReward::Nothing => "Nothing",
        }
    }

    /// Get icon handle for the reward
    pub fn icon(&self, icons: &crate::resources::asset_handles::LootboxIcons) -> Handle<Image> {
        match self {
            LootboxReward::Teleport => icons.teleport.clone(),
            LootboxReward::MonsterHorde => icons.monster_horde.clone(),
            LootboxReward::TravelersArrive => icons.travelers_arrive.clone(),
            LootboxReward::EliteEnemy => icons.elite_enemy.clone(),
            LootboxReward::MegaTreasure => icons.mega_treasure.clone(),
            LootboxReward::MagicBarrier => icons.magic_barrier.clone(),
            LootboxReward::ArrowBarrage => icons.arrow_barrage.clone(),
            LootboxReward::ThunderStrike => icons.thunder_strike.clone(),
            LootboxReward::TntRain => icons.tnt_rain.clone(),
            LootboxReward::LaserBeam => icons.laser_beam.clone(),
            LootboxReward::Magnet => icons.magnet.clone(),
            LootboxReward::Nothing => icons.nothing.clone(),
        }
    }

    /// Get all possible rewards for the roulette
    pub fn all_rewards() -> Vec<Self> {
        vec![
            LootboxReward::Teleport,
            LootboxReward::MonsterHorde,
            LootboxReward::TravelersArrive,
            LootboxReward::EliteEnemy,
            LootboxReward::MegaTreasure,
            LootboxReward::MagicBarrier,
            LootboxReward::ArrowBarrage,
            LootboxReward::ThunderStrike,
            LootboxReward::TntRain,
            LootboxReward::LaserBeam,
            LootboxReward::Magnet,
            LootboxReward::Nothing,
        ]
    }

    /// Get weighted random reward
    pub fn random() -> Self {
        use rand::RngExt;
        let mut rng = rand::rng();
        let roll = rng.random_range(0..100);

        match roll {
            0..11 => LootboxReward::MonsterHorde, // 11%
            11..16 => LootboxReward::Teleport,
            16..36 => LootboxReward::TravelersArrive, // 25% (increased from 11% + removed TreasureRain 14%)
            36..42 => LootboxReward::EliteEnemy,      // 6%
            42..53 => LootboxReward::MegaTreasure,    // 11%
            53..62 => LootboxReward::MagicBarrier,    // 9%
            62..71 => LootboxReward::ArrowBarrage,    // 9%
            71..77 => LootboxReward::ThunderStrike,   // 6%
            77..82 => LootboxReward::TntRain,         // 5%
            82..88 => LootboxReward::LaserBeam,       // 6%
            88..97 => LootboxReward::Magnet,          // 9%
            _ => LootboxReward::Nothing,              // 3%
        }
    }
}
