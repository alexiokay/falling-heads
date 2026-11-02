//! Zero-gravity prison cell, simulated in the cell's own frame (the cell spins;
//! the frame turns with it). Pure logic in fixed steps, so it is testable and
//! does not touch the world's physics. Units are world pixels (64 per block).
use bevy::math::Vec2;

/// Interior half size: a 10x10-block room inside 1-block bedrock walls.
pub const HALF: f32 = 320.;
pub const WALL: f32 = 64.;
/// Head radius (heads are 52 px cubes).
pub const RADIUS: f32 = 26.;
/// Half width of a trapdoor opening (two blocks wide).
pub const DOOR_HALF: f32 = 64.;
pub const STEP: f32 = 1. / 120.;
/// Seconds to escape before the guards throw you out.
pub const TIME_LIMIT: f32 = 75.;
/// A button keeps its trapdoor open this long.
pub const DOOR_OPEN: f32 = 1.8;
/// Spin speed range (radians per second); it changes every few seconds and
/// sometimes reverses, easing toward the new speed.
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
const DRAG: f32 = 0.18;
const THRUST: f32 = 720.;
const MAX_SPEED: f32 = 1500.;
const BOUNCE: f32 = 0.85;
const GUARD_EVERY: f32 = 11.;
const GUARD_THRUST: f32 = 320.;
const MAX_GUARDS: usize = 3;
const GUARD_HIT_COOLDOWN: f32 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Hero,
    Villager,
    Guard,
}

#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub kind: Kind,
    pub p: Vec2,
    pub v: Vec2,
    pub escaped: bool,
    hit_cooldown: f32,
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Blast(Vec2),
    Button(usize),
    GuardIn(usize),
    HeroHit,
    VillagerOut,
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
    magnet_left: f32,
    next_magnet: f32,
    pub bodies: Vec<Body>,
    pub doors: [Door; 3],
    pub buttons: [Button; 3],
    pub events: Vec<Event>,
    pub outcome: Option<Outcome>,
    pub saved: u32,
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
        let body = |kind, p| Body {
            kind,
            p,
            v: Vec2::ZERO,
            escaped: false,
            hit_cooldown: 0.,
        };
        let mut bodies = vec![body(Kind::Hero, Vec2::new(0., -40.))];
        for i in 0..villagers {
            bodies.push(body(
                Kind::Villager,
                Vec2::new(if i % 2 == 0 { -150. } else { 150. }, 60.),
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
            magnet_left: 0.,
            next_magnet: 1.5,
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
            events: Vec::new(),
            outcome: None,
            saved: 0,
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
        self.bodies.iter().filter(|b| b.kind == Kind::Guard).count()
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
        if self.time >= self.next_spin_change {
            self.next_spin_change = self.time + 5. + self.random() * 4.;
            let speed = SPIN_MIN + self.random() * (SPIN_MAX - SPIN_MIN);
            let keep = self.random() > 0.35;
            self.omega_target = speed * self.omega.signum() * if keep { 1. } else { -1. };
        }
        let before = self.omega;
        self.omega += (self.omega_target - self.omega).clamp(-SPIN_EASE * dt, SPIN_EASE * dt);
        let alpha = (self.omega - before) / dt;
        self.angle += self.omega * dt;
        self.magnet_left = (self.magnet_left - dt).max(0.);
        if self.magnet_left == 0. {
            self.magnet = None;
        }
        if self.time >= self.next_magnet {
            let pick = (self.random() * 4.) as usize % 4;
            // Never the same magnet twice in a row.
            let pick = if Some(pick) == self.magnet {
                (pick + 1) % 4
            } else {
                pick
            };
            self.magnet = Some(pick);
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
        if self.time >= self.next_blast {
            self.next_blast = self.time + 1.6 + self.random() * 1.2;
            let at = Vec2::new(self.random() - 0.5, self.random() - 0.5) * (HALF - 60.) * 2.;
            self.blast(at);
        }
        if self.time >= self.next_guard {
            self.next_guard = self.time + GUARD_EVERY;
            if self.guards() < MAX_GUARDS {
                let door = (self.random() * 3.) as usize % 3;
                self.guard_in(door);
            }
        }
        // Thrust is given in screen space; the room is turned by `angle`.
        let thrust = rotate(steer.clamp_length_max(1.), -self.angle) * THRUST;
        let hero = self.bodies[0].p;
        for b in &mut self.bodies {
            if b.escaped {
                continue;
            }
            // In the turning room: centrifugal (out), Coriolis (curving) and
            // the push of a speeding-up or slowing spin; plus the magnet.
            let centrifugal = b.p * fling;
            let coriolis = Vec2::new(b.v.y, -b.v.x) * 2. * w;
            let euler = Vec2::new(b.p.y, -b.p.x) * alpha;
            let mut a = centrifugal + coriolis + euler + pull;
            match b.kind {
                Kind::Hero => a += thrust,
                Kind::Guard => a += (hero - b.p).normalize_or_zero() * GUARD_THRUST,
                Kind::Villager => {}
            }
            b.v += a * dt;
            b.v *= 1. - DRAG * dt;
            b.v = b.v.clamp_length_max(MAX_SPEED);
            b.p += b.v * dt;
            b.hit_cooldown = (b.hit_cooldown - dt).max(0.);
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
        } else if self.time >= TIME_LIMIT {
            self.outcome = Some(Outcome::Caught);
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
        let n = self.doors[door].normal;
        self.doors[door].open = self.doors[door].open.max(0.6);
        self.bodies.push(Body {
            kind: Kind::Guard,
            p: n * (HALF - RADIUS - 4.),
            v: -n * 300.,
            escaped: false,
            hit_cooldown: 0.,
        });
        self.events.push(Event::GuardIn(door));
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
                // A guard's baton: one hit, knocks the hero away.
                for (guard, other) in [(i, j), (j, i)] {
                    if self.bodies[guard].kind == Kind::Guard
                        && self.bodies[other].kind == Kind::Hero
                        && self.bodies[guard].hit_cooldown == 0.
                    {
                        self.bodies[guard].hit_cooldown = GUARD_HIT_COOLDOWN;
                        let away =
                            (self.bodies[other].p - self.bodies[guard].p).normalize_or_zero();
                        self.bodies[other].v += away * 450.;
                        self.events.push(Event::HeroHit);
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
                    && body.kind != Kind::Guard
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
                        if body.kind != Kind::Guard
                            && let Some(button) = self.buttons.iter().position(|bt| {
                                bt.normal == normal && (bt.at - body.p).length() < RADIUS + 34.
                            })
                            && into > 60.
                        {
                            let target = self.buttons[button].door;
                            self.buttons[button].lit = 0.4;
                            self.doors[target].open = DOOR_OPEN;
                            self.events.push(Event::Button(button));
                        }
                        body.v -= normal * into * (1. + BOUNCE);
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
        let mut cell = Cell::new(3, 0);
        cell.next_blast = f32::MAX;
        cell.next_guard = f32::MAX;
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
        let mut cell = Cell::new(5, 0);
        cell.next_blast = f32::MAX;
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
        run(&mut cell, 20.);
        assert!(cell.guards() >= 1);
        assert!(cell.events.contains(&Event::HeroHit));
        run(&mut cell, TIME_LIMIT);
        assert_eq!(cell.outcome, Some(Outcome::Caught));
        assert!(cell.guards() <= MAX_GUARDS);
    }
}
