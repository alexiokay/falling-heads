//! How a friendly head plays "the ball": gravity, floor/wall bounces and how
//! hard it hits blocks. Heads without a `PlayStyle` behave like `Bouncy`.
use crate::utils::constants::{MAX_BOUNCE_VELOCITY, MIN_BOUNCE_VELOCITY};
use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum PlayStyle {
    /// The classic head: full bounces off every surface.
    #[default]
    Bouncy,
    /// Falls fast, barely bounces and hits blocks twice as hard on hard
    /// landings, so it digs down through the terrain.
    Heavy,
    /// Low gravity, soft and steady bounces: long hang time, drifts sideways.
    Floaty,
    /// Grabs walls on contact, hangs on briefly, then wall-jumps up and away.
    Sticky,
    /// Bounces like Bouncy; a hard landing splits it into two small heads
    /// that merge back after a few seconds.
    Splitter,
}

impl PlayStyle {
    pub const ALL: [PlayStyle; 5] = [
        Self::Bouncy,
        Self::Heavy,
        Self::Floaty,
        Self::Sticky,
        Self::Splitter,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "bouncy" | "classic" | "default" => Some(Self::Bouncy),
            "heavy" | "smasher" => Some(Self::Heavy),
            "floaty" | "glider" => Some(Self::Floaty),
            "sticky" | "climber" => Some(Self::Sticky),
            "splitter" | "split" => Some(Self::Splitter),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Bouncy => "Bouncy",
            Self::Heavy => "Heavy",
            Self::Floaty => "Floaty",
            Self::Sticky => "Sticky",
            Self::Splitter => "Splitter",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Avian gravity scale (characters default to 1.5).
    pub fn gravity_scale(self) -> f32 {
        match self {
            Self::Heavy => 2.4,
            Self::Floaty => 0.6,
            Self::Bouncy | Self::Sticky | Self::Splitter => 1.5,
        }
    }

    /// Upward speed after landing on a block with downward speed `vy` (< 0).
    pub fn floor_bounce(self, vy: f32, restitution: f32) -> f32 {
        match self {
            // Thuds and stays low instead of springing back up.
            Self::Heavy => (-vy * 0.3).clamp(90.0, 260.0),
            // Same rebound every time; low gravity turns it into long hops.
            Self::Floaty => (-vy * restitution).clamp(300.0, 420.0),
            Self::Bouncy | Self::Sticky | Self::Splitter => {
                (-vy * restitution).clamp(MIN_BOUNCE_VELOCITY, MAX_BOUNCE_VELOCITY)
            }
        }
    }

    /// Share of horizontal speed kept when bouncing off a wall.
    pub fn wall_bounce_factor(self) -> f32 {
        match self {
            Self::Heavy => 0.5,
            _ => 1.0,
        }
    }

    /// Block damage when landing on a block with downward speed `vy`.
    pub fn landing_damage(self, vy: f32) -> u32 {
        if self == Self::Heavy && vy < -450.0 {
            2
        } else {
            1
        }
    }
}

/// A Sticky head holding on to a wall. `side` is the wall normal (+1 = wall on
/// the left, so the hop goes right).
#[derive(Component, Clone, Copy, Debug)]
pub struct WallCling {
    pub side: f32,
    pub release_at: f32,
}

/// A Sticky head has used its wall grab for this jump. It can grab a wall
/// again only after landing on a floor, so it hops ledges but cannot climb a
/// sheer wall forever.
#[derive(Component, Clone, Copy, Debug)]
pub struct ClingSpent;

impl WallCling {
    pub const HOLD_SECONDS: f32 = 0.45;
    /// Velocity of the wall jump that ends a cling.
    pub fn hop(self) -> Vec2 {
        Vec2::new(self.side * 260.0, 480.0)
    }
}

/// Downward speed of a landing that splits a Splitter head.
pub const SPLIT_LANDING_SPEED: f32 = 550.0;
/// Seconds the two halves stay apart before the small twin flies back.
pub const SPLIT_SECONDS: f32 = 4.0;
/// Size of each half while split.
pub const SPLIT_SCALE: f32 = 0.62;

/// Set by the terrain solver on a Splitter's hard landing; the play-style
/// system decides whether it may split now.
#[derive(Component, Clone, Copy, Debug)]
pub struct SplitRequest {
    pub speed: f32,
}

/// A Splitter head that is currently split; its small twin is `twin`.
#[derive(Component, Clone, Copy, Debug)]
pub struct Split {
    pub twin: Entity,
    pub until: f32,
}

/// The small half that popped out of a Splitter head.
#[derive(Component, Clone, Copy, Debug)]
pub struct SplitTwin {
    pub parent: Entity,
}

/// A twin flying back to merge; terrain no longer stops it.
#[derive(Component, Clone, Copy, Debug)]
pub struct TwinReturning {
    pub since: f32,
}

/// No new split until this time, so a head does not split on every bounce.
#[derive(Component, Clone, Copy, Debug)]
pub struct SplitCooldown(pub f32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_parse_back_and_cycle_through_every_style() {
        for style in PlayStyle::ALL {
            assert_eq!(PlayStyle::parse(style.name()), Some(style));
        }
        assert_eq!(PlayStyle::parse(" HEAVY "), Some(PlayStyle::Heavy));
        assert_eq!(PlayStyle::parse("rocket"), None);
        let mut s = PlayStyle::Bouncy;
        for _ in 0..PlayStyle::ALL.len() {
            s = s.next();
        }
        assert_eq!(s, PlayStyle::Bouncy);
    }

    #[test]
    fn heavy_lands_low_and_hits_hard_floaty_keeps_a_steady_hop() {
        let fall = -700.0;
        let bouncy = PlayStyle::Bouncy.floor_bounce(fall, 1.0);
        assert!(PlayStyle::Heavy.floor_bounce(fall, 1.0) < bouncy / 2.0);
        assert_eq!(PlayStyle::Heavy.landing_damage(fall), 2);
        assert_eq!(PlayStyle::Heavy.landing_damage(-200.0), 1);
        assert_eq!(PlayStyle::Bouncy.landing_damage(fall), 1);
        assert_eq!(PlayStyle::Floaty.floor_bounce(-100.0, 1.0), 300.0);
        assert_eq!(PlayStyle::Floaty.floor_bounce(-2000.0, 1.0), 420.0);
        assert!(PlayStyle::Floaty.gravity_scale() < PlayStyle::Bouncy.gravity_scale());
    }
}
