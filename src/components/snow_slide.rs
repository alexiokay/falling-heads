//! A short supported slide, followed by the original landing rebound.
//! No parent attachment, gravity change or repeated landing impulse is needed.
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct SnowSlide {
    pub release_at: f32,
    pub rebound: f32,
}
#[derive(Clone, Copy)]
pub(crate) enum Surface {
    Snow,
    Ice,
    Ground,
}
#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    Hold,
    Rebound(f32),
    Leave,
}
impl SnowSlide {
    pub fn new(now: f32, rebound: f32) -> Self {
        // A brief skid (50–80 ms), then the normal rebound. Stronger impacts
        // retain a little more contact without the previous sticky pause.
        let duration = 0.05 + 0.03 * ((rebound - 400.) / 150.).clamp(0., 1.);
        Self {
            release_at: now + duration,
            rebound,
        }
    }
    pub fn step(self, now: f32, support: Option<Surface>, vertical_speed: f32) -> Step {
        // Respect an external upward impulse, a teleport, or a lost ledge.
        if vertical_speed > 80. {
            return Step::Leave;
        }
        match support {
            None => Step::Leave,
            Some(Surface::Snow) if now < self.release_at => Step::Hold,
            Some(Surface::Ice) => Step::Rebound((self.rebound * 0.22).clamp(30., 85.)),
            Some(_) => Step::Rebound(self.rebound),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delay_is_bounded_and_cannot_be_refreshed_by_adjacent_tiles() {
        for rebound in [400., 475., 550.] {
            let slide = SnowSlide::new(10., rebound);
            assert!(slide.release_at >= 10.049 && slide.release_at <= 10.081);
            assert_eq!(slide.step(10.025, Some(Surface::Snow), -12.), Step::Hold);
            assert_eq!(
                slide.step(10.081, Some(Surface::Snow), -12.),
                Step::Rebound(rebound)
            );
            assert_eq!(slide.step(10.1, None, -12.), Step::Leave);
            assert_eq!(slide.step(10.1, Some(Surface::Snow), 150.), Step::Leave);
            assert_eq!(
                slide.step(10.1, Some(Surface::Ground), -12.),
                Step::Rebound(rebound)
            );
            assert_eq!(
                slide.step(10.1, Some(Surface::Ice), -12.),
                Step::Rebound(85.)
            );
        }
    }
}
