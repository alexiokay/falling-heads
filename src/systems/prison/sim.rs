//! Zero-gravity prison cell, simulated in the cell's own frame (the cell spins;
//! the frame turns with it). Pure logic in fixed steps, so it is testable and
//! does not touch the world's physics. Units are world pixels (64 per block).
//!
//! On top of the spin, magnets, blasts, buttons and guards: an alarm that rises
//! every 30 s (faster spin, more guards and blasts, a guard captain at level 2),
//! magnet overloads that pin everyone to the walls and then fling them, floating
//! power-ups (thruster fuel, a shield, a key card) and villagers that go for the
//! buttons and dash for any open trapdoor.
use bevy::math::Vec2;

/// Interior half size: a 10x10-block room inside 1-block bedrock walls.
pub const HALF: f32 = 320.;
pub const WALL: f32 = 64.;
/// Head radius (heads are 52 px cubes).
pub const RADIUS: f32 = 26.;
/// Half width of a trapdoor opening (two blocks wide).
pub const DOOR_HALF: f32 = 64.;
pub const STEP: f32 = 1. / 120.;
/// Escapes faster than this earn a speed bonus (2 XP per second left).
pub const QUICK_ESCAPE: f32 = 75.;
/// A button keeps its trapdoor open this long (a key card makes it `KEY_OPEN`).
pub const DOOR_OPEN: f32 = 1.8;
pub const KEY_OPEN: f32 = 5.;
/// Spin speed range (radians per second); it changes every few seconds and
/// sometimes reverses, easing toward the new speed. The alarm speeds it up.
const SPIN_MIN: f32 = 0.75;
const SPIN_MAX: f32 = 1.35;
const SPIN_EASE: f32 = 0.8;
/// How strongly the spinning room's own forces act (1 = real physics): heads
/// are flung outward and curve as the walls sweep past them.
const SPIN_FORCES: f32 = 0.7;
const BLAST_RADIUS: f32 = 270.;
const BLAST_SPEED: f32 = 1100.;
/// The four electromagnets around the cell stay put (right, up, left, down in
/// the world); one at a time pulls every head toward its side.
pub const MAGNETS: [Vec2; 4] = [Vec2::X, Vec2::Y, Vec2::NEG_X, Vec2::NEG_Y];
const MAGNET_PULL: f32 = 560.;
const MAGNET_ON: f32 = 1.4;
const MAGNET_GAP: f32 = 0.6;
/// Overload: all magnets at once pin every head to the walls, then fling them.
pub const OVERLOAD_TIME: f32 = 2.2;
const OVERLOAD_PULL: f32 = 950.;
const OVERLOAD_BURST: f32 = 950.;
const DRAG: f32 = 0.18;
const THRUST: f32 = 720.;
const MAX_SPEED: f32 = 1500.;
const BOUNCE: f32 = 0.85;
const GUARD_EVERY: f32 = 11.;
const GUARD_THRUST: f32 = 320.;
const MAX_GUARDS: usize = 3;
const GUARD_HIT_COOLDOWN: f32 = 1.5;
/// The alarm rises a level this often, up to `MAX_ALARM`.
pub const ALARM_EVERY: f32 = 30.;
pub const MAX_ALARM: u32 = 4;
const CAPTAIN_THRUST: f32 = 250.;
const CAPTAIN_HIT_COOLDOWN: f32 = 2.5;
/// A captain's baton stuns the hero (no thrust) this long.
const STUN: f32 = 1.2;
/// Ramming the captain this fast knocks his key card loose.
const RAM_SPEED: f32 = 650.;
const VILLAGER_THRUST: f32 = 300.;
const MAX_POWERUPS: usize = 2;
const POWERUP_LIFE: f32 = 20.;
pub const FUEL_TIME: f32 = 5.;
pub const SHIELD_TIME: f32 = 15.;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Hero,
    Villager,
    Guard,
    /// Bigger, slower guard from alarm level 2: his baton stuns; ram him to
    /// knock a key card loose.
    Captain,
}

impl Kind {
    pub fn is_guard(self) -> bool {
        matches!(self, Kind::Guard | Kind::Captain)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub kind: Kind,
    pub p: Vec2,
    pub v: Vec2,
    pub escaped: bool,
    /// Seconds the hero is stunned (no thrust).
    pub stun: f32,
    hit_cooldown: f32,
    /// Villagers: seconds to the next change of plan and where they head.
    think: f32,
    aim: Vec2,
    /// Captain: the key card was knocked loose already.
    dropped: bool,
}

impl Body {
    fn new(kind: Kind, p: Vec2, v: Vec2) -> Self {
        Self {
            kind,
            p,
            v,
            escaped: false,
            stun: 0.,
            hit_cooldown: 0.,
            think: 0.,
            aim: Vec2::ZERO,
            dropped: false,
        }
    }
}

/// A trapdoor in the middle of a wall; `normal` points out of the cell.
#[derive(Clone, Copy, Debug)]
pub struct Door {
    pub normal: Vec2,
    /// Seconds it stays open; 0 is shut.
    pub open: f32,
}
/// A button on a wall's inner face that opens one trapdoor.
#[derive(Clone, Copy, Debug)]
pub struct Button {
    pub at: Vec2,
    /// The wall it sits on, pointing out of the cell.
    pub normal: Vec2,
    pub door: usize,
    /// Seconds it stays lit after a press.
    pub lit: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerKind {
    /// Double thrust for `FUEL_TIME` seconds.
    Fuel,
    /// Blocks the next baton hit (for up to `SHIELD_TIME` seconds).
    Shield,
    /// The next button press holds its trapdoor open for `KEY_OPEN` seconds.
    Key,
}

/// A power-up drifting through the cell until the hero grabs it.
#[derive(Clone, Copy, Debug)]
pub struct PowerUp {
    pub id: u32,
    pub kind: PowerKind,
    pub p: Vec2,
    v: Vec2,
    age: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Blast(Vec2),
    Button(usize),
    GuardIn(usize),
    CaptainIn(usize),
    HeroHit,
    Stunned,
    ShieldBlock,
    VillagerOut,
    Alarm(u32),
    Overload,
    OverloadRelease,
    PowerUpTaken(PowerKind),
    KeyDrop,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Outcome {
    Escaped { seconds: f32, saved: u32 },
    Caught,
}

pub struct Cell {
    pub time: f32,
    pub angle: f32,
    /// Current spin, radians per second (negative turns the other way).
    pub omega: f32,
    omega_target: f32,
    next_spin_change: f32,
    /// The pulling magnet (index into `MAGNETS`) and seconds it stays on.
    pub magnet: Option<usize>,
    last_magnet: Option<usize>,
    magnet_left: f32,
    next_magnet: f32,
    /// Seconds left of a magnet overload (all four at once).
    pub overload: f32,
    next_overload: f32,
    /// Alarm level 0..=MAX_ALARM.
    pub alarm: u32,
    next_alarm: f32,
    captain_in: bool,
    pub bodies: Vec<Body>,
    pub doors: [Door; 3],
    pub buttons: [Button; 3],
    pub powerups: Vec<PowerUp>,
    next_powerup: f32,
    next_id: u32,
    /// Seconds of double thrust left.
    pub fuel: f32,
    /// Seconds the shield lasts (0: none).
    pub shield: f32,
    /// A key card is held.
    pub key: bool,
    pub events: Vec<Event>,
    pub outcome: Option<Outcome>,
    pub saved: u32,
    /// Seconds before the guards throw you out; `None` (the default): no limit.
    pub limit: Option<f32>,
    next_blast: f32,
    next_guard: f32,
    rng: u32,
}

/// Turns `v` by `angle` radians.
pub fn rotate(v: Vec2, angle: f32) -> Vec2 {
    Vec2::from_angle(angle).rotate(v)
}

impl Cell {
    pub fn new(seed: u32, villagers: usize) -> Self {
        let mut bodies = vec![Body::new(Kind::Hero, Vec2::new(0., -40.), Vec2::ZERO)];
        for i in 0..villagers {
            bodies.push(Body::new(
                Kind::Villager,
                Vec2::new(if i % 2 == 0 { -150. } else { 150. }, 60.),
                Vec2::ZERO,
            ));
        }
        let up = Vec2::Y;
        let left = Vec2::NEG_X;
        let right = Vec2::X;
        Self {
            time: 0.,
            angle: 0.,
            omega: 0.9,
            omega_target: 0.9,
            next_spin_change: 6.,
            magnet: None,
            last_magnet: None,
            magnet_left: 0.,
            next_magnet: 1.5,
            overload: 0.,
            next_overload: 18.,
            alarm: 0,
            next_alarm: ALARM_EVERY,
            captain_in: false,
            bodies,
            // Top, left and right trapdoors.
            doors: [
                Door {
                    normal: up,
                    open: 0.,
                },
                Door {
                    normal: left,
                    open: 0.,
                },
                Door {
                    normal: right,
                    open: 0.,
                },
            ],
            // Each button sits across the room from its trapdoor, so a press
            // has to be followed by a long flight (a blast helps).
            buttons: [
                Button {
                    at: Vec2::new(0., -HALF),
                    normal: Vec2::NEG_Y,
                    door: 0,
                    lit: 0.,
                },
                Button {
                    at: Vec2::new(-HALF, 200.),
                    normal: left,
                    door: 2,
                    lit: 0.,
                },
                Button {
                    at: Vec2::new(HALF, -200.),
                    normal: right,
                    door: 1,
                    lit: 0.,
                },
            ],
            powerups: Vec::new(),
            next_powerup: 8.,
            next_id: 0,
            fuel: 0.,
            shield: 0.,
            key: false,
            events: Vec::new(),
            outcome: None,
            saved: 0,
            limit: None,
            next_blast: 2.,
            next_guard: 6.,
            rng: seed | 1,
        }
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    #[cfg(test)]
    pub fn hero(&self) -> &Body {
        &self.bodies[0]
    }

    pub fn guards(&self) -> usize {
        self.bodies.iter().filter(|b| b.kind.is_guard()).count()
    }

    /// One fixed step. `steer` is the player's input in screen space.
    pub fn tick(&mut self, steer: Vec2) {
        if self.outcome.is_some() {
            // The cell keeps turning while the hero flies away (or is thrown out).
            self.angle += self.omega * STEP;
            return;
        }
        let dt = STEP;
        self.time += dt;
        self.escalate();
        let alarm = self.alarm as f32;
        if self.time >= self.next_spin_change {
            self.next_spin_change = self.time + 5. + self.random() * 4.;
            let speed = (SPIN_MIN + self.random() * (SPIN_MAX - SPIN_MIN)) * (1. + 0.15 * alarm);
            let keep = self.random() > 0.35;
            self.omega_target = speed * self.omega.signum() * if keep { 1. } else { -1. };
        }
        let before = self.omega;
        self.omega += (self.omega_target - self.omega).clamp(-SPIN_EASE * dt, SPIN_EASE * dt);
        let alpha = (self.omega - before) / dt;
        self.angle += self.omega * dt;
        let overloading = self.overload_step(dt);
        self.magnet_left = (self.magnet_left - dt).max(0.);
        if self.magnet_left == 0. || overloading {
            self.magnet = None;
        }
        if self.time >= self.next_magnet && !overloading {
            let pick = (self.random() * 4.) as usize % 4;
            // Never the same magnet twice in a row.
            let pick = if Some(pick) == self.last_magnet {
                (pick + 1) % 4
            } else {
                pick
            };
            self.magnet = Some(pick);
            self.last_magnet = Some(pick);
            self.magnet_left = MAGNET_ON;
            self.next_magnet = self.time + MAGNET_ON + MAGNET_GAP;
        }
        let pull = self.magnet.map_or(Vec2::ZERO, |m| {
            rotate(MAGNETS[m], -self.angle) * MAGNET_PULL
        });
        let (w, alpha) = (self.omega * SPIN_FORCES, alpha * SPIN_FORCES);
        let fling = self.omega * self.omega * SPIN_FORCES;
        for d in &mut self.doors {
            d.open = (d.open - dt).max(0.);
        }
        for b in &mut self.buttons {
            b.lit = (b.lit - dt).max(0.);
        }
        self.fuel = (self.fuel - dt).max(0.);
        self.shield = (self.shield - dt).max(0.);
        if self.time >= self.next_blast {
            self.next_blast = self.time + (1.6 + self.random() * 1.2) * (1. - 0.12 * alarm);
            let at = Vec2::new(self.random() - 0.5, self.random() - 0.5) * (HALF - 60.) * 2.;
            self.blast(at);
        }
        if self.time >= self.next_guard {
            self.next_guard = self.time + (GUARD_EVERY - 1.8 * alarm).max(5.);
            if self.guards() < MAX_GUARDS + self.alarm as usize {
                let door = (self.random() * 3.) as usize % 3;
                self.guard_in(door);
            }
        }
        self.powerups_step(dt);
        self.villagers_think(dt);
        // Thrust is given in screen space; the room is turned by `angle`.
        let boost = if self.fuel > 0. { 2. } else { 1. };
        let thrust = rotate(steer.clamp_length_max(1.), -self.angle) * THRUST * boost;
        let hero = self.bodies[0].p;
        let exit = self
            .doors
            .iter()
            .find(|d| d.open > 0.)
            .map(|d| d.normal * (HALF + WALL * 2.));
        for b in &mut self.bodies {
            if b.escaped {
                continue;
            }
            // In the turning room: centrifugal (out), Coriolis (curving) and
            // the push of a speeding-up or slowing spin; plus the magnet(s).
            let centrifugal = b.p * fling;
            let coriolis = Vec2::new(b.v.y, -b.v.x) * 2. * w;
            let euler = Vec2::new(b.p.y, -b.p.x) * alpha;
            let pinned = if overloading {
                b.p.normalize_or_zero() * OVERLOAD_PULL
            } else {
                pull
            };
            let mut a = centrifugal + coriolis + euler + pinned;
            match b.kind {
                Kind::Hero if b.stun == 0. => a += thrust,
                Kind::Hero => {}
                Kind::Guard => a += (hero - b.p).normalize_or_zero() * GUARD_THRUST,
                Kind::Captain => a += (hero - b.p).normalize_or_zero() * CAPTAIN_THRUST,
                // An open trapdoor wins over the plan; pinned in an overload.
                Kind::Villager if overloading => {}
                Kind::Villager => {
                    a += (exit.unwrap_or(b.aim) - b.p).normalize_or_zero() * VILLAGER_THRUST;
                }
            }
            b.v += a * dt;
            b.v *= 1. - DRAG * dt;
            b.v = b.v.clamp_length_max(MAX_SPEED);
            b.p += b.v * dt;
            b.hit_cooldown = (b.hit_cooldown - dt).max(0.);
            b.stun = (b.stun - dt).max(0.);
        }
        self.collide_heads();
        for i in 0..self.bodies.len() {
            self.walls(i);
        }
        let hero = self.bodies[0];
        if hero.escaped {
            self.outcome = Some(Outcome::Escaped {
                seconds: self.time,
                saved: self.saved,
            });
        } else if self.limit.is_some_and(|l| self.time >= l) {
            self.outcome = Some(Outcome::Caught);
        }
    }

    /// The alarm rises every `ALARM_EVERY` seconds; at level 2 the captain comes.
    fn escalate(&mut self) {
        if self.alarm < MAX_ALARM && self.time >= self.next_alarm {
            self.alarm += 1;
            self.next_alarm += ALARM_EVERY;
            self.events.push(Event::Alarm(self.alarm));
        }
        if self.alarm >= 2 && !self.captain_in {
            self.captain_in = true;
            let door = (self.random() * 3.) as usize % 3;
            self.spawn_guard(door, Kind::Captain);
        }
    }

    /// Starts, runs and ends magnet overloads. True while one is on.
    fn overload_step(&mut self, dt: f32) -> bool {
        if self.overload > 0. {
            self.overload -= dt;
            if self.overload <= 0. {
                self.overload = 0.;
                // Release: everyone is flung back across the cell.
                for i in 0..self.bodies.len() {
                    if self.bodies[i].escaped {
                        continue;
                    }
                    let spread = (self.random() - 0.5) * 1.2;
                    let dir = rotate(-self.bodies[i].p.normalize_or(Vec2::Y), spread);
                    self.bodies[i].v = dir * OVERLOAD_BURST;
                }
                self.events.push(Event::OverloadRelease);
                return false;
            }
            return true;
        }
        if self.time >= self.next_overload {
            self.overload = OVERLOAD_TIME;
            self.next_overload = self.time + OVERLOAD_TIME + 22. + self.random() * 10.;
            self.events.push(Event::Overload);
            return true;
        }
        false
    }

    /// Spawns, moves, expires and hands out power-ups.
    fn powerups_step(&mut self, dt: f32) {
        if self.time >= self.next_powerup {
            self.next_powerup = self.time + 10. + self.random() * 5.;
            if self.powerups.len() < MAX_POWERUPS {
                let r = self.random();
                let kind = if r < 0.25 {
                    PowerKind::Key
                } else if r < 0.62 {
                    PowerKind::Fuel
                } else {
                    PowerKind::Shield
                };
                let p = Vec2::new(self.random() - 0.5, self.random() - 0.5) * 440.;
                let v = rotate(Vec2::X, self.random() * std::f32::consts::TAU) * 40.;
                self.add_powerup(kind, p, v);
            }
        }
        let hero = self.bodies[0];
        let mut taken = Vec::new();
        for (i, pu) in self.powerups.iter_mut().enumerate() {
            pu.age += dt;
            pu.p += pu.v * dt;
            for axis in 0..2 {
                if pu.p[axis].abs() > HALF - 30. {
                    pu.p[axis] = pu.p[axis].clamp(-(HALF - 30.), HALF - 30.);
                    pu.v[axis] = -pu.v[axis];
                }
            }
            if !hero.escaped && (pu.p - hero.p).length() < RADIUS + 22. {
                taken.push(i);
            }
        }
        for &i in taken.iter().rev() {
            let kind = self.powerups.remove(i).kind;
            match kind {
                PowerKind::Fuel => self.fuel = FUEL_TIME,
                PowerKind::Shield => self.shield = SHIELD_TIME,
                PowerKind::Key => self.key = true,
            }
            self.events.push(Event::PowerUpTaken(kind));
        }
        self.powerups.retain(|pu| pu.age < POWERUP_LIFE);
    }

    pub fn add_powerup(&mut self, kind: PowerKind, p: Vec2, v: Vec2) {
        self.next_id += 1;
        self.powerups.push(PowerUp {
            id: self.next_id,
            kind,
            p,
            v,
            age: 0.,
        });
    }

    /// Villagers pick a button to go for every couple of seconds.
    fn villagers_think(&mut self, dt: f32) {
        for i in 0..self.bodies.len() {
            if self.bodies[i].kind != Kind::Villager || self.bodies[i].escaped {
                continue;
            }
            self.bodies[i].think -= dt;
            if self.bodies[i].think <= 0. {
                self.bodies[i].think = 1.5 + self.random() * 1.5;
                let b = (self.random() * 3.) as usize % 3;
                self.bodies[i].aim = self.buttons[b].at;
            }
        }
    }

    /// A micro-explosion: pushes every head near it away, hard.
    pub fn blast(&mut self, at: Vec2) {
        self.events.push(Event::Blast(at));
        for i in 0..self.bodies.len() {
            let d = self.bodies[i].p - at;
            let dist = d.length();
            if dist < BLAST_RADIUS && !self.bodies[i].escaped {
                let dir = if dist > 1. {
                    d / dist
                } else {
                    rotate(Vec2::X, self.random() * std::f32::consts::TAU)
                };
                let push = BLAST_SPEED * (1. - dist / BLAST_RADIUS).sqrt();
                self.bodies[i].v += dir * push;
            }
        }
    }

    /// A guard comes in through a trapdoor, heading for the middle.
    pub fn guard_in(&mut self, door: usize) {
        self.spawn_guard(door, Kind::Guard);
    }

    pub fn spawn_guard(&mut self, door: usize, kind: Kind) {
        let n = self.doors[door].normal;
        self.doors[door].open = self.doors[door].open.max(0.6);
        self.bodies
            .push(Body::new(kind, n * (HALF - RADIUS - 4.), -n * 300.));
        self.events.push(if kind == Kind::Captain {
            Event::CaptainIn(door)
        } else {
            Event::GuardIn(door)
        });
    }

    fn collide_heads(&mut self) {
        let n = self.bodies.len();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (self.bodies[i], self.bodies[j]);
                if a.escaped || b.escaped {
                    continue;
                }
                let d = b.p - a.p;
                let dist = d.length();
                if dist >= RADIUS * 2. || dist < 0.001 {
                    continue;
                }
                let normal = d / dist;
                let overlap = RADIUS * 2. - dist;
                self.bodies[i].p -= normal * overlap * 0.5;
                self.bodies[j].p += normal * overlap * 0.5;
                let closing = (a.v - b.v).dot(normal);
                if closing > 0. {
                    let impulse = normal * closing * (1. + BOUNCE) * 0.5;
                    self.bodies[i].v -= impulse;
                    self.bodies[j].v += impulse;
                }
                for (guard, other) in [(i, j), (j, i)] {
                    let g = self.bodies[guard];
                    if !g.kind.is_guard() || self.bodies[other].kind != Kind::Hero {
                        continue;
                    }
                    // Rammed hard, the captain loses his key card (once).
                    // (Speeds from before this bump's impulse.)
                    let hero_speed = if other == i { a.v } else { b.v }.length();
                    if g.kind == Kind::Captain
                        && !g.dropped
                        && closing > RAM_SPEED * 0.6
                        && hero_speed > RAM_SPEED * 0.5
                    {
                        self.bodies[guard].dropped = true;
                        self.bodies[guard].hit_cooldown = 1.;
                        let at = g.p;
                        let v = (g.p - self.bodies[other].p).normalize_or_zero() * 120.;
                        self.add_powerup(PowerKind::Key, at, v);
                        self.events.push(Event::KeyDrop);
                        continue;
                    }
                    if g.hit_cooldown > 0. {
                        continue;
                    }
                    let captain = g.kind == Kind::Captain;
                    self.bodies[guard].hit_cooldown = if captain {
                        CAPTAIN_HIT_COOLDOWN
                    } else {
                        GUARD_HIT_COOLDOWN
                    };
                    let away = (self.bodies[other].p - g.p).normalize_or_zero();
                    if self.shield > 0. {
                        // The shield takes the hit and is used up.
                        self.shield = 0.;
                        self.bodies[other].v += away * 300.;
                        self.events.push(Event::ShieldBlock);
                        continue;
                    }
                    self.bodies[other].v += away * if captain { 650. } else { 450. };
                    self.events.push(Event::HeroHit);
                    if captain {
                        self.bodies[other].stun = STUN;
                        self.events.push(Event::Stunned);
                    }
                }
            }
        }
    }

    /// Bounces a head off the walls, presses buttons, lets it out of open doors.
    fn walls(&mut self, i: usize) {
        let b = self.bodies[i];
        if b.escaped {
            return;
        }
        let mut body = b;
        for axis in 0..2 {
            for sign in [-1., 1.] {
                let mut normal = Vec2::ZERO;
                normal[axis] = sign;
                let out = body.p.dot(normal);
                let along = body.p[1 - axis];
                let door = self.doors.iter().position(|d| d.normal == normal);
                let doorway = door.is_some_and(|d| self.doors[d].open > 0.)
                    && !body.kind.is_guard()
                    && along.abs() < DOOR_HALF - RADIUS * 0.4;
                if doorway {
                    // Through an open trapdoor: keep inside the opening's sides,
                    // out once past the wall.
                    if out > HALF + WALL + RADIUS {
                        body.escaped = true;
                        if body.kind == Kind::Villager {
                            self.saved += 1;
                            self.events.push(Event::VillagerOut);
                        }
                    } else if out > HALF - RADIUS {
                        let limit = DOOR_HALF - RADIUS;
                        if body.p[1 - axis].abs() > limit {
                            body.p[1 - axis] = body.p[1 - axis].clamp(-limit, limit);
                            body.v[1 - axis] *= -BOUNCE;
                        }
                    }
                    continue;
                }
                if out > HALF - RADIUS {
                    // A head caught in a closing doorway goes back in.
                    body.p -= normal * (out - (HALF - RADIUS));
                    let into = body.v.dot(normal);
                    if into > 0. {
                        if !body.kind.is_guard()
                            && let Some(button) = self.buttons.iter().position(|bt| {
                                bt.normal == normal && (bt.at - body.p).length() < RADIUS + 34.
                            })
                            && into > 60.
                        {
                            let target = self.buttons[button].door;
                            self.buttons[button].lit = 0.4;
                            // The hero's key card holds the trapdoor open longer.
                            let open = if body.kind == Kind::Hero && self.key {
                                self.key = false;
                                KEY_OPEN
                            } else {
                                DOOR_OPEN
                            };
                            self.doors[target].open = self.doors[target].open.max(open);
                            self.events.push(Event::Button(button));
                        }
                        // An overload pins heads: the walls soak up the bounce.
                        let bounce = if self.overload > 0. { 0.05 } else { BOUNCE };
                        body.v -= normal * into * (1. + bounce);
                    }
                }
            }
        }
        self.bodies[i] = body;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(cell: &mut Cell, seconds: f32) {
        for _ in 0..(seconds / STEP) as usize {
            cell.tick(Vec2::ZERO);
        }
    }

    /// A quiet cell: no blasts, guards, overloads, power-ups or alarm.
    fn quiet(seed: u32, villagers: usize) -> Cell {
        let mut cell = Cell::new(seed, villagers);
        cell.next_blast = f32::MAX;
        cell.next_guard = f32::MAX;
        cell.next_overload = f32::MAX;
        cell.next_powerup = f32::MAX;
        cell.next_alarm = f32::MAX;
        cell
    }

    #[test]
    fn heads_stay_inside_a_shut_cell_and_blasts_keep_them_moving() {
        let mut cell = Cell::new(7, 2);
        // No guards for this one.
        cell.next_guard = f32::MAX;
        run(&mut cell, 30.);
        for b in &cell.bodies {
            if !b.escaped {
                assert!(b.p.x.abs() <= HALF && b.p.y.abs() <= HALF, "{b:?}");
            }
        }
        assert!(
            cell.events
                .iter()
                .filter(|e| matches!(e, Event::Blast(_)))
                .count()
                >= 6
        );
    }

    #[test]
    fn a_button_opens_the_trapdoor_across_the_room_and_the_hero_escapes() {
        let mut cell = quiet(3, 0);
        // Fly into the bottom button: the top trapdoor opens.
        cell.bodies[0].p = Vec2::new(0., -HALF + RADIUS + 10.);
        cell.bodies[0].v = Vec2::new(0., -400.);
        for _ in 0..12 {
            cell.tick(Vec2::ZERO);
        }
        assert!(cell.doors[0].open > 0., "top trapdoor did not open");
        assert!(cell.events.contains(&Event::Button(0)));
        // Shot straight up through it before it shuts.
        cell.bodies[0].p = Vec2::new(0., HALF - 40.);
        cell.bodies[0].v = Vec2::new(0., 1400.);
        for _ in 0..60 {
            cell.tick(Vec2::ZERO);
        }
        assert!(matches!(cell.outcome, Some(Outcome::Escaped { .. })));
    }

    #[test]
    fn shut_trapdoors_hold_and_guards_never_leave() {
        let mut cell = quiet(5, 0);
        cell.bodies[0].p = Vec2::new(0., HALF - 40.);
        cell.bodies[0].v = Vec2::new(0., 1400.);
        for _ in 0..60 {
            cell.tick(Vec2::ZERO);
        }
        assert!(!cell.hero().escaped, "went through a shut trapdoor");
        cell.guard_in(0);
        cell.doors[0].open = DOOR_OPEN;
        let guard = cell.bodies.len() - 1;
        cell.bodies[guard].v = Vec2::new(0., 1400.);
        for _ in 0..60 {
            cell.tick(Vec2::ZERO);
        }
        assert!(!cell.bodies[guard].escaped);
    }

    #[test]
    fn magnets_take_turns_and_the_spin_varies_and_reverses() {
        let mut cell = Cell::new(9, 1);
        let mut used = Vec::new();
        let (mut slow, mut fast, mut flips) = (f32::MAX, 0f32, 0);
        let mut last = cell.omega.signum();
        for _ in 0..(60. / STEP) as usize {
            cell.tick(Vec2::ZERO);
            if let Some(m) = cell.magnet
                && used.last() != Some(&m)
            {
                used.push(m);
            }
            slow = slow.min(cell.omega.abs());
            fast = fast.max(cell.omega.abs());
            if cell.omega.signum() != last {
                flips += 1;
                last = cell.omega.signum();
            }
        }
        assert!(used.len() >= 10, "magnets fired {} times", used.len());
        assert!(used.windows(2).all(|p| p[0] != p[1]));
        assert!((0..4).all(|m| used.contains(&m)), "every magnet fires");
        assert!(flips >= 1, "the spin never reversed");
        assert!(fast > 0.9 && fast - slow > 0.3, "spin {slow}..{fast}");
    }

    #[test]
    fn guards_come_in_and_hit_the_hero_and_time_runs_out() {
        let mut cell = Cell::new(11, 0);
        cell.next_blast = f32::MAX;
        cell.next_powerup = f32::MAX;
        run(&mut cell, 20.);
        assert!(cell.guards() >= 1);
        assert!(cell.events.contains(&Event::HeroHit));
        assert!(cell.outcome.is_none(), "no time limit by default");
        cell.limit = Some(30.);
        run(&mut cell, 12.);
        assert_eq!(cell.outcome, Some(Outcome::Caught));
        assert!(cell.guards() <= MAX_GUARDS + cell.alarm as usize);
    }

    #[test]
    fn the_alarm_rises_to_its_top_level_and_brings_the_captain() {
        let mut cell = Cell::new(13, 0);
        // Held in the middle, so it cannot escape and end the run.
        let hold = |cell: &mut Cell, seconds: f32| {
            for _ in 0..(seconds / STEP) as usize {
                cell.bodies[0].p = Vec2::ZERO;
                cell.bodies[0].v = Vec2::ZERO;
                cell.tick(Vec2::ZERO);
            }
        };
        hold(&mut cell, ALARM_EVERY * 2. + 1.);
        assert_eq!(cell.alarm, 2);
        assert!(cell.events.contains(&Event::Alarm(1)));
        assert!(cell.events.contains(&Event::Alarm(2)));
        let captains = cell
            .bodies
            .iter()
            .filter(|b| b.kind == Kind::Captain)
            .count();
        assert_eq!(captains, 1, "one captain at alarm 2");
        hold(&mut cell, ALARM_EVERY * 3.);
        assert_eq!(cell.alarm, MAX_ALARM);
        assert_eq!(
            cell.bodies
                .iter()
                .filter(|b| b.kind == Kind::Captain)
                .count(),
            1,
            "never a second captain"
        );
    }

    #[test]
    fn an_overload_pins_heads_to_the_walls_then_flings_them() {
        let mut cell = quiet(17, 2);
        cell.next_overload = 0.5;
        run(&mut cell, 0.5 + OVERLOAD_TIME - 0.1);
        assert!(cell.events.contains(&Event::Overload));
        assert!(cell.magnet.is_none(), "no single magnet during an overload");
        for b in &cell.bodies {
            assert!(
                b.p.x.abs().max(b.p.y.abs()) > HALF - RADIUS - 40.,
                "not pinned to a wall: {b:?}"
            );
        }
        run(&mut cell, 0.2);
        assert!(cell.events.contains(&Event::OverloadRelease));
        assert!(cell.bodies.iter().all(|b| b.v.length() > 500.), "not flung");
    }

    #[test]
    fn power_ups_fuel_the_thrust_shield_a_hit_and_key_a_long_door() {
        let mut cell = quiet(19, 0);
        cell.add_powerup(PowerKind::Fuel, cell.bodies[0].p, Vec2::ZERO);
        cell.tick(Vec2::ZERO);
        assert!(cell.events.contains(&Event::PowerUpTaken(PowerKind::Fuel)));
        assert!(cell.fuel > 0. && cell.powerups.is_empty());
        // A shield takes a baton hit instead of the hero.
        cell.add_powerup(PowerKind::Shield, cell.bodies[0].p, Vec2::ZERO);
        cell.tick(Vec2::ZERO);
        assert!(cell.shield > 0.);
        cell.guard_in(0);
        let g = cell.bodies.len() - 1;
        cell.bodies[g].p = cell.bodies[0].p + Vec2::X * (RADIUS * 2. - 4.);
        cell.bodies[g].v = Vec2::NEG_X * 200.;
        cell.tick(Vec2::ZERO);
        assert!(cell.events.contains(&Event::ShieldBlock));
        assert!(!cell.events.contains(&Event::HeroHit));
        assert_eq!(cell.shield, 0.);
        // A key card holds the trapdoor open for KEY_OPEN.
        cell.bodies.truncate(1);
        cell.key = true;
        cell.bodies[0].p = Vec2::new(0., -HALF + RADIUS + 10.);
        cell.bodies[0].v = Vec2::new(0., -400.);
        for _ in 0..12 {
            cell.tick(Vec2::ZERO);
        }
        assert!(
            cell.doors[0].open > DOOR_OPEN,
            "door {}",
            cell.doors[0].open
        );
        assert!(!cell.key, "the key card is used up");
    }

    #[test]
    fn the_captain_stuns_and_drops_his_key_card_when_rammed() {
        let mut cell = quiet(23, 0);
        cell.spawn_guard(0, Kind::Captain);
        let c = cell.bodies.len() - 1;
        cell.bodies[0].p = Vec2::ZERO;
        cell.bodies[0].v = Vec2::ZERO;
        cell.bodies[c].p = Vec2::X * (RADIUS * 2. - 4.);
        cell.bodies[c].v = Vec2::NEG_X * 150.;
        cell.tick(Vec2::ZERO);
        assert!(cell.events.contains(&Event::Stunned));
        assert!(cell.bodies[0].stun > 0.);
        // Rammed hard: the key card comes loose (once).
        cell.bodies[0].p = Vec2::ZERO;
        cell.bodies[0].v = Vec2::X * 1200.;
        cell.bodies[c].p = Vec2::X * (RADIUS * 2. - 2.);
        cell.bodies[c].v = Vec2::ZERO;
        cell.tick(Vec2::ZERO);
        assert!(cell.events.contains(&Event::KeyDrop));
        assert!(cell.powerups.iter().any(|p| p.kind == PowerKind::Key));
    }

    #[test]
    fn villagers_dash_out_of_an_open_trapdoor() {
        let mut cell = quiet(29, 1);
        cell.next_magnet = f32::MAX;
        cell.omega = 0.;
        cell.omega_target = 0.;
        cell.next_spin_change = f32::MAX;
        cell.bodies[1].p = Vec2::new(0., 100.);
        cell.doors[0].open = 100.;
        run(&mut cell, 6.);
        assert_eq!(cell.saved, 1, "the villager did not get out");
    }
}
