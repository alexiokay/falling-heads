//! Finite-volume liquids on the game's block grid. Simulation is independent of
//! rendering, deterministic, and restricted to loaded chunks. Unloaded liquid
//! state is retained for the session; sources have finite reservoirs.
use crate::{
    components::{
        block::{BiomeType, Block, BlockType},
        buff::{Buff, BuffType},
        chunk::{ChunkMember, LoadedChunks},
        enemy::Health,
    },
    resources::{material_cache::MaterialCache, mesh_cache::MeshCache},
};
use avian3d::prelude::LinearVelocity;
use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
type Cell = (i32, i32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liquid {
    Water,
    Lava,
}
#[derive(Component)]
pub struct FluidSeed {
    pub cell: Cell,
    pub kind: Liquid,
}
#[derive(Clone, Copy, Debug)]
struct Volume {
    kind: Liquid,
    amount: u8,
    reserve: u16,
}
#[derive(Clone, Copy, Debug)]
struct FrozenWater {
    ticks: u16,
    water: Volume,
}
#[derive(Resource, Default)]
pub struct FluidWorld {
    cells: BTreeMap<Cell, Volume>,
    seeded: HashSet<Cell>,
    cooled: BTreeSet<Cell>,
    solids: HashMap<Cell, Entity>,
    tick: u64,
    falling: BTreeMap<Cell, (Liquid, u8)>,
    fall_life: BTreeMap<Cell, u8>,
    fall_offset: BTreeMap<Cell, f32>,
    supported: HashSet<Cell>,
    steam: BTreeMap<Cell, u8>,
    ice: BTreeMap<Cell, FrozenWater>,
    frost_exposure: BTreeMap<Cell, u16>,
}
fn chunk(p: Cell) -> Cell {
    (p.0.div_euclid(16), p.1.div_euclid(16))
}
pub fn spring_kind(x: i32, y: i32, seed: u32, biome: BiomeType) -> Option<Liquid> {
    let h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed;
    let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    if h % 31 != 0 {
        return None;
    }
    match biome {
        BiomeType::Nether => Some(if (h / 31) % 7 == 0 {
            Liquid::Water
        } else {
            Liquid::Lava
        }),
        BiomeType::Plains | BiomeType::Forest | BiomeType::Underground | BiomeType::Mountains => {
            Some(Liquid::Water)
        }
        _ => None,
    }
}
/// Include an extra margin so new solids cannot catch a moving/growing head.
#[cfg(test)]
fn occupied_cells(bodies: impl Iterator<Item = (Vec2, f32)>) -> HashSet<Cell> {
    let mut result = HashSet::new();
    for (position, half) in bodies {
        let lo = ((position - Vec2::splat(half + 8.)) / 64.)
            .round()
            .as_ivec2();
        let hi = ((position + Vec2::splat(half + 8.)) / 64.)
            .round()
            .as_ivec2();
        for y in lo.y..=hi.y {
            for x in lo.x..=hi.x {
                result.insert((x, y));
            }
        }
    }
    result
}
impl FluidWorld {
    pub(crate) fn clear_transport_cells(&mut self, cells: &[(i32, i32)]) {
        for p in cells {
            self.cells.remove(p);
            self.seeded.insert(*p);
            self.cooled.remove(p);
            self.solids.remove(p);
            self.falling.remove(p);
            self.fall_life.remove(p);
            self.fall_offset.remove(p);
            self.supported.remove(p);
            self.steam.remove(p);
            self.ice.remove(p);
            self.frost_exposure.remove(p);
        }
    }
    fn freeze(&mut self, p: Cell, ticks: u16, occupied: &HashSet<Cell>) -> bool {
        if occupied.contains(&p) || !self.supported.contains(&p) {
            return false;
        }
        // Frost fills a supported grid cell, including shallow water. Occupancy
        // is checked against the entire new block, and thawing returns exactly
        // the captured water volume instead of creating a new full source.
        if !self
            .cells
            .get(&p)
            .is_some_and(|v| v.kind == Liquid::Water && v.amount > 0)
        {
            return false;
        }
        let water = self.cells.remove(&p).unwrap();
        self.falling.remove(&p);
        self.ice.insert(p, FrozenWater { ticks, water });
        true
    }

    fn thaw(&mut self, p: Cell) -> bool {
        if let Some(frozen) = self.ice.remove(&p) {
            self.cells.insert(p, frozen.water);
            return true;
        }
        false
    }

    pub(crate) fn lava_light_positions(&self, min: Vec2, max: Vec2) -> Vec<Vec2> {
        let mut p: Vec<_> = self
            .cells
            .iter()
            .filter(|(_, v)| v.kind == Liquid::Lava && v.amount >= 4)
            .map(|(p, v)| {
                Vec2::new(
                    p.0 as f32 * 64.,
                    p.1 as f32 * 64. - 32. + v.amount as f32 * 4.,
                )
            })
            .filter(|p| p.cmpge(min).all() && p.cmple(max).all())
            .collect();
        p.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        let mut result = Vec::new();
        for v in p {
            if result
                .iter()
                .all(|q: &Vec2| q.distance_squared(v) > 192_f32.powi(2))
            {
                result.push(v);
            }
            if result.len() == 24 {
                break;
            }
        }
        result
    }

    /// Add finite liquid only to a compatible cell. Callers check terrain first.
    pub(crate) fn deposit(&mut self, p: Cell, kind: Liquid, amount: u8) -> u8 {
        if self.cooled.contains(&p) || self.ice.contains_key(&p) {
            return 0;
        }
        let v = self.cells.entry(p).or_insert(Volume {
            kind,
            amount: 0,
            reserve: 0,
        });
        if v.kind != kind {
            return 0;
        }
        let added = amount.min(8 - v.amount);
        v.amount += added;
        added
    }
    /// Shared heat reaction for breath, fireballs and fiery weather. Finite
    /// water is consumed, never replaced with an infinite source.
    pub(crate) fn heat_cell(&mut self, p: Cell) -> bool {
        let mut wet = false;
        if let Some(v) = self.cells.get_mut(&p) {
            if v.kind == Liquid::Water && v.amount > 0 {
                wet = true;
                v.amount -= 1;
                v.reserve = 0;
                self.steam.insert(p, 30);
            }
        }
        if self.cells.get(&p).is_some_and(|v| v.amount == 0) {
            self.cells.remove(&p);
        }
        if let Some((kind, amount)) = self.falling.get_mut(&p) {
            if *kind == Liquid::Water && *amount > 0 {
                wet = true;
                *amount -= 1;
                self.steam.insert(p, 30);
            }
        }
        self.falling.retain(|_, (_, amount)| *amount > 0);
        if let Some(ice) = self.ice.get_mut(&p) {
            ice.ticks = ice.ticks.saturating_sub(30);
            wet = true;
        }
        wet
    }
    pub(crate) fn melt_cap(&mut self, p: Cell) {
        self.deposit(p, Liquid::Water, 1);
        self.supported.insert(p);
        self.steam.insert(p, 30);
    }

    pub(crate) fn is_water_at(&self, position: Vec2) -> bool {
        let p = (position / 64.).round().as_ivec2();
        self.cells
            .get(&(p.x, p.y))
            .is_some_and(|v| v.kind == Liquid::Water && v.amount > 0)
    }
    /// Surface-height-aware wetness, including shallow pools and falling streams.
    pub(crate) fn covers_surface(&self, point: Vec2) -> bool {
        let p = (point / 64.).round().as_ivec2();
        let cell = (p.x, p.y);
        self.falling.contains_key(&cell)
            || self.cells.get(&cell).is_some_and(|v| {
                v.amount > 0 && point.y < p.y as f32 * 64. - 32. + v.amount as f32 * 8. + 0.01
            })
    }
    pub fn has_liquid_at(&self, x: i32, y: i32) -> bool {
        if self.cooled.contains(&(x, y)) || self.ice.contains_key(&(x, y)) {
            return false;
        }
        self.cells.get(&(x, y)).is_some_and(|v| v.amount > 0) || self.falling.contains_key(&(x, y))
    }
    fn react_weather(
        &mut self,
        weather: &crate::components::weather::ActiveWeather,
        center: Vec2,
        blocked: &HashSet<Cell>,
        occupied: &HashSet<Cell>,
    ) {
        use crate::components::weather::WeatherType;
        if !weather.is_snowing() {
            self.frost_exposure.clear();
        }
        if weather.intensity < 0.1 || weather.duration_remaining <= 0. {
            return;
        }
        let period = match weather.weather_type {
            WeatherType::Rain => 240,
            WeatherType::FireRain => 20,
            WeatherType::Snow => 1,
            _ => return,
        };
        let period = if weather.weather_type == WeatherType::Snow {
            1
        } else {
            (period as f32 / weather.intensity.clamp(0.1, 1.)).round() as u64
        };
        if self.tick % period != 0 {
            return;
        }
        let exposed: Vec<_> = self
            .cells
            .keys()
            .copied()
            .filter(|p| {
                Vec2::new(p.0 as f32 * 64., p.1 as f32 * 64.).distance_squared(center)
                    < 2000_f32.powi(2)
                    && (1..=2).all(|dy| {
                        !blocked.contains(&(p.0, p.1 + dy))
                            && !self.ice.contains_key(&(p.0, p.1 + dy))
                            && !self.has_liquid_at(p.0, p.1 + dy)
                    })
            })
            .collect();
        if weather.weather_type == WeatherType::Snow {
            let eligible: HashSet<_> = exposed
                .iter()
                .copied()
                .filter(|p| {
                    self.cells[p].kind == Liquid::Water
                        && self.cells[p].amount >= 6
                        && self.supported.contains(p)
                        && !occupied.contains(p)
                })
                .collect();
            self.frost_exposure.retain(|p, _| eligible.contains(p));
            for p in eligible {
                let exposure = self.frost_exposure.entry(p).or_default();
                *exposure = exposure
                    .saturating_add((weather.intensity.clamp(0.1, 1.) * 10.).round() as u16);
                if *exposure >= 1200 {
                    self.freeze(p, 180, occupied);
                    self.frost_exposure.remove(&p);
                }
            }
            return;
        }
        // Local cave weather collects on exposed ledges, with a strict per-tick
        // budget. No reservoir: puddles obey the same finite flow and ageing rules.
        if weather.weather_type == WeatherType::Rain {
            let mut ledges: Vec<_> = blocked
                .iter()
                .map(|p| (p.0, p.1 + 1))
                .filter(|p| {
                    !blocked.contains(p)
                        && !self.cooled.contains(p)
                        && (1..=2).all(|dy| !blocked.contains(&(p.0, p.1 + dy)))
                        && Vec2::new(p.0 as f32 * 64., p.1 as f32 * 64.).distance_squared(center)
                            < 1200_f32.powi(2)
                })
                .collect();
            ledges.sort_unstable();
            if !ledges.is_empty() {
                let offset = (self.tick / period) as usize % ledges.len();
                ledges.rotate_left(offset);
                for p in ledges
                    .into_iter()
                    .filter(|p| !self.has_liquid_at(p.0, p.1))
                    .take(2)
                    .collect::<Vec<_>>()
                {
                    if self.deposit(p, Liquid::Water, 1) > 0 {
                        self.supported.insert(p);
                    }
                }
            }
        }
        for p in exposed {
            let v = self.cells[&p];
            match (weather.weather_type, v.kind) {
                (WeatherType::Rain, Liquid::Lava) => {
                    self.cells.remove(&p);
                    if v.amount >= 4 {
                        self.cooled.insert(p);
                    }
                    self.steam.insert(p, 30);
                }
                (WeatherType::FireRain, Liquid::Water) => {
                    self.heat_cell(p);
                }
                _ => {}
            }
        }
    }

    fn transfer(
        &mut self,
        from: Cell,
        to: Cell,
        limit: u8,
        blocked: &HashSet<Cell>,
        loaded: &impl Fn(Cell) -> bool,
    ) {
        if blocked.contains(&to)
            || self.cooled.contains(&to)
            || self.ice.contains_key(&to)
            || !loaded(to)
        {
            return;
        }
        let Some(a) = self.cells.get(&from).copied() else {
            return;
        };
        let b = self.cells.get(&to).copied().unwrap_or(Volume {
            kind: a.kind,
            amount: 0,
            reserve: 0,
        });
        if a.kind != b.kind {
            return;
        }
        let amount = a.amount.min(8 - b.amount).min(limit);
        if amount == 0 {
            return;
        }
        self.cells.get_mut(&from).unwrap().amount -= amount;
        self.cells.entry(to).or_insert(b).amount += amount;
        if to.1 == from.1 && to.0 != from.0 {
            // Preserve the actual lip of the source block, not the destination cell centre.
            let half = if a.kind == Liquid::Water { 6. } else { 9. };
            self.fall_offset
                .insert(to, (from.0 - to.0) as f32 * (32. - half));
        }
        if to.1 < from.1 {
            let offset = self.fall_offset.get(&from).copied().unwrap_or(0.);
            self.fall_offset.insert(to, offset);
            for p in [from, to] {
                self.fall_life
                    .insert(p, if a.kind == Liquid::Water { 3 } else { 6 });
                self.falling
                    .entry(p)
                    .and_modify(|v| v.1 = v.1.max(amount))
                    .or_insert((a.kind, amount));
            }
        }
    }
    fn step(&mut self, blocked: &HashSet<Cell>, loaded: impl Fn(Cell) -> bool) {
        self.tick += 1;
        self.steam.retain(|_, ttl| {
            *ttl = ttl.saturating_sub(1);
            *ttl > 0
        });
        // Transport persists briefly between simulation steps, rather than
        // deleting the connecting stream every time a parcel changes cells.
        let _lava_step = self.tick % 4 == 0;
        self.fall_life.retain(|p, ttl| {
            *ttl = ttl.saturating_sub(1);
            *ttl > 0 && !blocked.contains(p) && !self.cooled.contains(p)
        });
        self.falling.retain(|p, _| self.fall_life.contains_key(p));
        // Re-streamed terrain must not contain a retained liquid cell. Displace it
        // into adjacent loaded space; any trapped remainder is absorbed by the solid.
        let embedded: Vec<_> = self
            .cells
            .keys()
            .copied()
            .filter(|p| blocked.contains(p))
            .collect();
        for p in embedded {
            for q in [(p.0, p.1 + 1), (p.0 - 1, p.1), (p.0 + 1, p.1)] {
                self.transfer(p, q, 8, blocked, &loaded);
            }
            self.cells.remove(&p);
            self.falling.remove(&p);
        }
        // Stable ordered coordinates avoid hash iteration affecting flow direction.
        let active: Vec<_> = self.cells.keys().copied().filter(|&p| loaded(p)).collect();
        for &p in &active {
            let Some(a) = self.cells.get(&p).copied() else {
                continue;
            };
            if a.kind != Liquid::Lava || a.amount == 0 {
                continue;
            }
            for q in [
                (p.0 - 1, p.1),
                (p.0 + 1, p.1),
                (p.0, p.1 - 1),
                (p.0, p.1 + 1),
            ] {
                if loaded(q)
                    && self
                        .cells
                        .get(&q)
                        .is_some_and(|b| b.kind == Liquid::Water && b.amount > 0)
                {
                    let enough = a.amount >= 2 && self.cells[&q].amount >= 1;
                    self.cells.remove(&p);
                    let water = self.cells.get_mut(&q).unwrap();
                    water.amount = water.amount.saturating_sub(if enough { 2 } else { 1 });
                    if enough {
                        self.cooled.insert(p);
                    }
                    self.steam.insert(p, 30);
                    break;
                }
            }
        }
        for p in active {
            if blocked.contains(&p) {
                continue;
            }
            let Some(a) = self.cells.get_mut(&p) else {
                continue;
            };
            if a.kind == Liquid::Lava && self.tick % 4 != 0 {
                continue;
            }
            let supply = (8 - a.amount).min(a.reserve.min(8) as u8);
            a.amount += supply;
            a.reserve -= supply as u16;
            let lava = a.kind == Liquid::Lava;
            self.transfer(
                p,
                (p.0, p.1 - 1),
                if lava { 2 } else { 8 },
                blocked,
                &loaded,
            );
            let below = (p.0, p.1 - 1);
            // Gravity wins: no sideways shelves while the cell below can accept liquid.
            let support = blocked.contains(&below)
                || self.cooled.contains(&below)
                || self.ice.contains_key(&below)
                || (self.supported.contains(&below)
                    && self.cells.get(&below).is_some_and(|v| v.amount == 8));
            if !support {
                continue;
            }
            let directions = if self.tick % 2 == 0 { [-1, 1] } else { [1, -1] };
            for dx in directions {
                let q = (p.0 + dx, p.1);
                let own = self.cells.get(&p).map_or(0, |v| v.amount);
                let neighbour = self.cells.get(&q).map_or(0, |v| v.amount);
                let balance = own.saturating_sub(neighbour) / 2;
                self.transfer(
                    p,
                    q,
                    balance.min(if lava { 1 } else { 2 }),
                    blocked,
                    &loaded,
                );
            }
        }
        self.cells.retain(|_, v| v.amount > 0 || v.reserve > 0);
        // A full parcel in free fall is not a supporting pool. Resolve support
        // from solid ground upward (BTreeMap orders each column bottom to top).
        self.supported.clear();
        for &p in self.cells.keys() {
            let below = (p.0, p.1 - 1);
            if blocked.contains(&below)
                || self.cooled.contains(&below)
                || self.ice.contains_key(&below)
                || (self.supported.contains(&below)
                    && self.cells.get(&below).is_some_and(|v| v.amount == 8))
            {
                self.supported.insert(p);
            }
        }
        self.fall_offset
            .retain(|p, _| self.cells.contains_key(p) || self.falling.contains_key(p));
    }
}

mod render;
mod runtime;
pub use render::FluidMaterial;
use render::*;
use runtime::*;
#[cfg(test)]
mod tests;
/// Freeze all water within `radius` of `center` into walkable ice for `ticks`
/// simulation steps (10 a second). Sent by the ICEAGE power.
#[derive(Message, Clone, Copy, Debug)]
pub struct FreezeWaterNear {
    pub center: Vec2,
    pub radius: f32,
    pub ticks: u16,
}

pub struct FluidPlugin;
impl Plugin for FluidPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FluidWorld>()
            .add_message::<FreezeWaterNear>()
            .init_resource::<FluidOccupancy>()
            .init_resource::<FluidRenderRevision>()
            .add_plugins(MaterialPlugin::<FluidMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (
                    update_occupancy,
                    bucket_powers,
                    ingest,
                    frost_projectiles,
                    freeze_requests,
                    simulate,
                    contact,
                    update_visual_revision,
                    render,
                )
                    .chain()
                    .after(crate::systems::world_generation::generate_chunks_around_hero)
                    .after(crate::systems::world_generation::despawn_distant_chunks)
                    .after(crate::plugins::physics_plugin::PhysicsSet::Collision)
                    .run_if(in_state(crate::states::InGame)),
            );
    }
}
