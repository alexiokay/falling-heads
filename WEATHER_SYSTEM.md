# 🌦️ Weather & Anomaly System

## Overview

A two-layer weather system that provides:
1. **Biome-Appropriate Weather** - Normal weather that fits each biome (rain in Plains, snow in Mountains)
2. **Rare Anomalies** - Cross-biome chaotic events (snow in Nether, fire rain anywhere)

---

## Weather Types

### 🌤️ Normal Weather
- **Clear** - No weather effects
- **Rain** - Water droplets falling (Plains, Forest)
- **Snow** - Snowflakes drifting down (Mountains)
- **Sandstorm** - Horizontal sand particles (Desert)
- **AshStorm** - Ember/ash particles (Nether)
- **VoidStorm** - Purple particles (End)

### ⚡ Anomalies (Rare Events)
- **FireRain** - Burning meteors falling from sky (any biome)
- **BloodMoon** - Red particles + increased enemy spawns
- **AuroraStorm** - Beautiful multicolor particles (Mountains/End)
- **Cross-Biome Weather** - Snow in Nether, rain in Desert, etc.

---

## Architecture

### Component Structure

```rust
pub enum WeatherType {
    Clear, Rain, Snow, Sandstorm, AshStorm, VoidStorm,
    FireRain, BloodMoon, AuroraStorm
}

pub enum WeatherCategory {
    Normal,   // Regular biome weather
    Anomaly,  // Rare cross-biome event
}

pub struct ActiveWeather {
    pub weather_type: WeatherType,
    pub category: WeatherCategory,
    pub intensity: f32,              // 0.0 to 1.0
    pub duration_remaining: f32,      // seconds
    pub transition_timer: f32,        // fade in/out
}
```

### Biome Weather Configuration

Each biome has:
- **Possible weather types** with probabilities
- **Duration range** (min/max seconds)
- **Anomaly check chance** (0.10 = 10% to roll for anomaly)

Example:
```rust
BiomeType::Nether => BiomeWeather {
    possible_weather: vec![
        (WeatherType::Clear, 0.7),
        (WeatherType::AshStorm, 0.3),
    ],
    min_duration: 20.0,
    max_duration: 60.0,
    anomaly_chance: 0.15,  // 15% chance to check anomalies
}
```

### Global Anomaly Table

Anomalies are defined globally and can:
- Work in **any biome** (allowed_biomes: None)
- Be **restricted to specific biomes** (allowed_biomes: Some(vec![...]))
- Have **individual probabilities** (probability: 0.02 = 2%)

Example:
```rust
AnomalyConfig {
    weather_type: WeatherType::Snow,
    probability: 0.02,  // 2% when anomaly check triggers
    min_duration: 15.0,
    max_duration: 45.0,
    allowed_biomes: Some(vec![BiomeType::Nether]),  // Only in Nether
}
```

---

## Probability System

**Two-Layer Probability**:
1. **Anomaly Check**: Does biome roll for anomaly? (biome.anomaly_chance)
2. **Anomaly Selection**: Which anomaly triggers? (anomaly.probability)

**Example: Snow in Nether**
- Nether anomaly chance: **15%**
- Snow anomaly probability: **2%**
- **Final chance**: 15% × 2% = **0.3%** per weather change
- **With 60s avg duration**: ~1 event every **5.5 hours** of gameplay

This makes anomalies truly **rare and special**!

---

## Weather Behavior by Biome

### 🌾 Plains
**Normal Weather:**
- Clear (70%)
- Rain (30%)

**Anomalies:**
- Fire Rain (rare)
- Blood Moon (3% chance)

### 🌲 Forest
**Normal Weather:**
- Clear (50%)
- Rain (50%)

**Anomalies:**
- Fire Rain (rare)
- Blood Moon (3% chance)

### 🏜️ Desert
**Normal Weather:**
- Clear (80%)
- Sandstorm (20%)

**Anomalies:**
- Rain (rare - unusual in desert!)
- Fire Rain (rare)

### ⛰️ Mountains
**Normal Weather:**
- Clear (60%)
- Snow (40%)

**Anomalies:**
- Aurora Storm (5% chance)
- Fire Rain (rare)

### 🕳️ Underground
**Normal Weather:**
- Clear (100%) - No weather underground

**Anomalies:**
- None (safe from weather)

### 🔥 Nether
**Normal Weather:**
- Clear (70%)
- Ash Storm (30%)

**Anomalies:**
- **Snow** (2% - iconic anomaly!)
- Sandstorm (4%)
- Fire Rain (rare)

### 🌌 End
**Normal Weather:**
- Clear (80%)
- Void Storm (20%)

**Anomalies:**
- Aurora Storm (5% chance)
- Fire Rain (rare)

---

## Visual Effects

### Particle System
- **Rain**: Fast falling droplets (800 units/sec downward)
- **Snow**: Slow drifting flakes (200 units/sec, horizontal drift)
- **Sandstorm**: Horizontal particles (300 units/sec)
- **Ash Storm**: Embers floating up and down
- **Fire Rain**: Meteors with trail effects
- **Blood Moon**: Red particles + red screen tint
- **Aurora**: Multicolor waves

### Particle Density
- **Intensity 0.0**: No particles
- **Intensity 1.0**: Max 50 particles on screen
- Smooth transitions over 3 seconds

### Anomaly Visual Indicator
- Anomalies have **pinkish glow** (Color::srgba(1.5, 1.0, 1.5))
- Increased particle count (60 instead of 50)
- Special particle colors/behaviors

---

## Gameplay Effects

### Weather Impact

**Rain**:
- No gameplay effects (purely visual)
- Could add: Slippery surfaces, reduced visibility

**Snow**:
- Slight movement slowdown (10% reduction)
- Matches existing Freezing buff from snow blocks

**Sandstorm**:
- Horizontal push (50 units/sec wind)
- Reduced visibility (future: fog effect)

**Ash Storm**:
- Small damage over time (1 HP every 3 seconds)
- Matches Nether's dangerous atmosphere

**Fire Rain** (Anomaly):
- Meteor particles deal 5 damage on contact
- Sets blocks on fire (future)

**Blood Moon** (Anomaly):
- 2x enemy spawn rate
- Enemies deal +1 damage
- Duration: 60-180 seconds

**Aurora Storm**:
- No negative effects (beautiful!)
- Could add: Speed buff, regeneration

---

## Implementation Files

### New Files
```
src/
├── components/
│   ├── weather.rs              - WeatherType, WeatherCategory, ActiveWeather
│   └── weather_particle.rs     - WeatherParticle component
├── systems/
│   └── weather_system.rs       - All weather logic
├── plugins/
│   └── weather_plugin.rs       - WeatherPlugin
└── resources/
    └── weather_config.rs       - GLOBAL_ANOMALIES table
```

### Modified Files
```
src/
├── components/mod.rs           - Export weather components
├── systems/mod.rs              - Export weather_system
├── plugins/mod.rs              - Export weather_plugin
└── main.rs                     - Add WeatherPlugin
```

---

## System Flow

```
1. Game starts
   ↓
2. ActiveWeather initialized (Clear, intensity 0.0)
   ↓
3. Weather update system runs every frame:
   - Decrease duration_remaining
   - Update transition_timer
   - Lerp intensity (smooth fade)
   ↓
4. Duration expires → Change weather:
   - Get current biome from camera Y position
   - Roll for anomaly (biome.anomaly_chance)
     ↓ Yes (15%)              ↓ No (85%)
   - Check GLOBAL_ANOMALIES  - Select normal weather
   - Filter by biome         - Use biome probabilities
   - Roll individual chance  - Random weighted selection
   - If success → Anomaly!   - New normal weather
   ↓
5. Spawn weather particles:
   - Based on weather_type
   - Scaled by intensity
   - Camera-relative positions
   ↓
6. Update particles:
   - Move based on velocity
   - Tick lifetime
   - Despawn if off-screen or expired
   ↓
7. Apply gameplay effects (optional):
   - Sandstorm push
   - Snow slowdown
   - Fire rain damage
   - Blood moon enemy boost
```

---

## Debug Commands

### Console Logs
- `"🌦️ Weather changed: Rain (Normal)"`
- `"🌪️ ANOMALY TRIGGERED: Snow in Nether!"`
- `"⚡ Weather anomaly active: FireRain"`

### Manual Weather Trigger (Future)
```rust
// Add to external API
POST /weather
{
  "type": "Snow",
  "duration": 30.0,
  "anomaly": true
}
```

---

## Configuration Constants

### Weather Timings
```rust
const WEATHER_TRANSITION_DURATION: f32 = 3.0;  // Fade in/out time
const MAX_PARTICLES_ON_SCREEN: usize = 50;     // Performance limit
const PARTICLE_SPAWN_RATE: f32 = 0.1;          // Spawn every 0.1 seconds
const PARTICLE_LIFETIME: f32 = 5.0;            // Despawn after 5 seconds
```

### Anomaly Probabilities
```rust
// Biome anomaly check chances
Plains: 0.10        (10%)
Forest: 0.10        (10%)
Desert: 0.08        (8%)
Mountains: 0.10     (10%)
Underground: 0.0    (0% - no weather)
Nether: 0.15        (15% - more chaotic!)
End: 0.12           (12%)

// Individual anomaly chances (when check triggers)
Snow in Nether: 0.02      (2%)
Fire Rain: 0.01           (1%)
Blood Moon: 0.03          (3%)
Aurora Storm: 0.05        (5%)
Cross-Biome: 0.04         (4%)
```

---

## Future Enhancements

### Phase 1 (Core) ✅
- [x] Weather types and components
- [x] Biome-to-weather mapping
- [x] Anomaly system
- [x] Particle spawning
- [x] Basic gameplay effects

### Phase 2 (Polish)
- [ ] Weather sounds (rain patter, wind howl)
- [ ] Screen effects (fog, tint)
- [ ] Lightning strikes during storms
- [ ] Weather affects AI (enemies seek shelter)

### Phase 3 (Advanced)
- [ ] Weather forecasting (see next weather in UI)
- [ ] Player-triggered weather (powers/items)
- [ ] Weather combos (Rain + Lightning = Thunderstorm)
- [ ] Seasonal weather patterns

---

## Performance Considerations

### Particle Limits
- Max 50 particles on screen
- Despawn off-screen particles immediately
- Pool and reuse particle entities (future optimization)

### Update Frequency
- Weather changes: Every 20-180 seconds (depends on biome)
- Particle spawn: Every 0.1 seconds during weather
- Particle update: Every frame (minimal overhead)

### Memory Usage
- ActiveWeather resource: ~100 bytes
- Weather particles: ~100 particles × 200 bytes = ~20 KB
- Anomaly table: Static data (~2 KB)

**Total overhead: < 25 KB** (negligible)

---

## Testing Checklist

### Normal Weather
- [ ] Rain in Plains
- [ ] Snow in Mountains
- [ ] Sandstorm in Desert
- [ ] Ash storm in Nether
- [ ] Void storm in End
- [ ] Clear weather in all biomes
- [ ] No weather underground

### Anomalies
- [ ] Snow in Nether (very rare)
- [ ] Fire rain in any biome
- [ ] Blood moon in Plains/Forest
- [ ] Aurora in Mountains/End
- [ ] Cross-biome weather

### Transitions
- [ ] Smooth fade in/out (3 seconds)
- [ ] Intensity lerping works
- [ ] No particle pop-in/pop-out

### Gameplay
- [ ] Sandstorm pushes player
- [ ] Snow slows movement
- [ ] Fire rain deals damage
- [ ] Blood moon increases enemy spawns

### Edge Cases
- [ ] Weather persists across pause/resume
- [ ] Weather changes when crossing biome boundaries
- [ ] Particles despawn properly
- [ ] No memory leaks

---

## Known Limitations

1. **No Weather Underground** - Design choice (immersion)
2. **Fixed Particle Count** - Performance trade-off
3. **No Weather Forecasting** - Could add in UI later
4. **Simple Particle Physics** - Good enough for now

---

## Credits

- **Design**: Biome-driven weather with anomaly system
- **Inspiration**: Terraria weather, Minecraft biomes, Hollow Knight atmosphere
- **Textures**: rain.png, snow.png (existing assets)

---

**Last Updated**: October 5, 2025
**Version**: 0.4.0
