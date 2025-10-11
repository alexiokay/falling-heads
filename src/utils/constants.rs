// Physics constants
pub const STANDARD_GRAVITY: f32 = -500.0;
pub const ARROW_GRAVITY: f32 = -600.0;  // Stronger gravity for realistic arc

// Entity sizes (in world units)
pub const BLOCK_SIZE: f32 = 64.0;
pub const HERO_SIZE: f32 = 64.0;
pub const VILLAGER_SIZE: f32 = 56.0;
pub const ENEMY_SIZE: f32 = 64.0;
pub const RESOURCE_SIZE: f32 = 32.0;
pub const ARROW_WIDTH: f32 = 90.0;  // MUCH bigger for testing
pub const ARROW_HEIGHT: f32 = 48.0;  // MUCH bigger for testing

// Mass values
pub const CHARACTER_MASS: f32 = 1.0;  // Light for easy bouncing
pub const TNT_MASS: f32 = 0.7;  // Very light so characters can push it

// Bounce/restitution values


// Bounce/restitution values
pub const HERO_BOUNCE: f32 = 1.0;  // Extra bouncy for better gameplay
pub const VILLAGER_BOUNCE: f32 = 1.0;
pub const ENEMY_BOUNCE: f32 = 1.0;  // Same as villagers
pub const STANDARD_BOUNCE: f32 = 1.0;
pub const NO_BOUNCE: f32 = 0.0;
pub const MIN_BOUNCE_VELOCITY: f32 = 400.0;  // Very asmall minimum to prevent tiny bounces
pub const MAX_BOUNCE_VELOCITY: f32 = 550.0;  // High limit to allow strong bounces from high falls
pub const MAX_ENTITY_HORIZONTAL_SPEED: f32 = 1200.0;  // Absolute max horizontal velocity (prevents teleportation from cumulative forces)
pub const MAX_ENTITY_VERTICAL_SPEED: f32 = 1500.0;    // Absolute max vertical velocity (allows high falls)

// Enemy behavior
pub const SKELETON_SHOOT_INTERVAL: f32 = 2.0;  // seconds between shots
pub const SKELETON_SHOOT_RANGE: f32 = 700.0;   // max distance to shoot (increased for world scale)
pub const SKELETON_IDEAL_RANGE: f32 = 400.0;   // Skeletons stay at this distance (good shooting distance)
pub const SKELETON_MIN_RANGE: f32 = 300.0;     // Too close - back away
pub const SKELETON_MAX_RANGE: f32 = 600.0;     // Too far - move closer
pub const ARROW_SPEED: f32 = 1200.0;  // Must be fast enough to fly horizontally before gravity pulls it down
pub const ARROW_LIFETIME: f32 = 1000.0;           // seconds before despawn
pub const ARROW_IMPACT_PUSH: f32 = 200.0;  // Horizontal push force when arrow hits
pub const ARROW_IMPACT_LIFT: f32 = 100.0;  // Upward push force when arrow hits

// Combat
pub const HERO_MELEE_DAMAGE: u32 = 1;          // Hero melee damage (1 heart)
pub const HERO_MELEE_COOLDOWN: f32 = 0.3;      // Per-enemy attack cooldown (industry standard)
pub const BLOCK_DAMAGE_COOLDOWN: f32 = 0.0;   // Block damage cooldown (prevents multi-frame hits)

// AI Movement
pub const VILLAGER_MOVE_SPEED: f32 = 100.0;  // Horizontal movement speed
pub const VILLAGER_MAX_SPEED: f32 = 200.0;   // Max horizontal velocity
pub const VILLAGER_DIRECTION_CHANGE_INTERVAL: f32 = 4.0;  // Seconds between direction changes
pub const ENEMY_MOVE_SPEED: f32 = 80.0;  // Slower than villagers
pub const ENEMY_MAX_SPEED: f32 = 150.0;  // Max horizontal velocity (slower than villagers)
pub const ENEMY_DIRECTION_CHANGE_INTERVAL: f32 = 3.0;  // Seconds between direction changes

// Spawn rates (probability 0.0 - 1.0)
pub const NETHER_ENEMY_SPAWN_RATE: f32 = 0.80;
pub const UNDERGROUND_ENEMY_SPAWN_RATE: f32 = 0.70;
pub const MOUNTAINS_ENEMY_SPAWN_RATE: f32 = 0.40;
pub const END_ENEMY_SPAWN_RATE: f32 = 0.50;

// Buff application
pub const BUFF_APPLICATION_CHANCE: f32 = 0.2;  // 20% chance on block collision

// Chunk generation
pub const CAVE_FREQUENCY_MULTIPLIER_NETHER: f32 = 1.5;
pub const CAVE_THRESHOLD_NETHER: f32 = 0.45;
pub const WORM_TUNNEL_CHANCE_NETHER: f32 = 0.6;

// TNT
pub const TNT_FUSE_TIME: f32 = 3.0;            // seconds
pub const TNT_EXPLOSION_RADIUS: f32 = 342.0;   // world units
pub const TNT_DAMAGE: u32 = 3;                 // damage to entities (1.5 hearts)

// Audio volumes (0.0 to 1.0)
pub const EXPLOSION_VOLUME: f32 = 0.3;         // TNT explosion sounds

// Laser
pub const LASER_DURATION: f32 = 0.5;           // seconds
pub const LASER_WIDTH: f32 = 320.0;
pub const LASER_HEIGHT: f32 = 2000.0;

// Spatial partitioning
pub const SPATIAL_HASH_CHUNK_SIZE: f32 = 256.0;  // 4 blocks per spatial chunk

// Resource rotation
pub const RESOURCE_ROTATION_SPEED: f32 = 180.0;  // degrees per second
pub const RESOURCE_DROP_VELOCITY_Y: f32 = 50.0;  // upward velocity when dropped
