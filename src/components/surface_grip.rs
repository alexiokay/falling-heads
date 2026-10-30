use bevy::prelude::*;

/// Brief momentum carry after landing. Uses the paused gameplay clock and does
/// not occupy a buff slot. Snow keeps normal bounce; ice keeps its low bounce.
#[derive(Component, Clone, Copy)]
pub struct SurfaceGrip {
    pub until: f32,
    pub counter_steering: f32,
}
impl SurfaceGrip {
    pub fn snow(now: f32) -> Self {
        Self {
            until: now + 0.65,
            counter_steering: 0.55,
        }
    }
    pub fn ice(now: f32) -> Self {
        Self {
            until: now + 1.0,
            counter_steering: 0.2,
        }
    }
    pub fn steering(self, velocity: f32, direction: f32, now: f32) -> f32 {
        if now < self.until && velocity * direction < -1. {
            self.counter_steering
        } else {
            1.
        }
    }
}
/// Noticeable but capped lateral carry; repeated landings never accelerate a
/// slow head indefinitely or slow down an already faster power/collision impulse.
pub fn snow_carry(velocity: f32) -> f32 {
    let speed = velocity.abs();
    if speed <= 1. || speed >= 280. {
        return velocity;
    }
    // Low-speed landings need enough carry to be visible across a 64px tile.
    // Fade this assistance out by 100px/s, and ramp it in above rest so tiny
    // collision jitter cannot turn into a sudden sideways launch.
    let slow_carry = 40. * (1. - speed / 100.).clamp(0., 1.) * ((speed - 1.) / 19.).clamp(0., 1.);
    velocity.signum() * (speed + (speed * 0.18).clamp(12., 30.) + slow_carry).min(280.)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slow_landings_have_visible_carry_without_launching_stationary_heads() {
        for direction in [-1., 1.] {
            assert!((snow_carry(direction * 20.) - direction * 64.).abs() < 0.01);
            assert!((snow_carry(direction * 40.) - direction * 76.).abs() < 0.01);
            assert_eq!(snow_carry(direction * 0.5), direction * 0.5);
            // Existing brisk movement is unaffected by the low-speed assistance.
            assert!((snow_carry(direction * 150.) - direction * 177.).abs() < 0.01);
        }
        // Carry remains monotonic: moving faster must never yield less drift.
        let mut previous = 0.;
        for i in 0..=3000 {
            let speed = i as f32 / 10.;
            let carried = snow_carry(speed);
            assert!(carried >= previous);
            previous = carried;
        }
        // Compare real steering over the first 0.1s after a slow landing.
        // Even while reversing, snow retains a visible fraction of a tile.
        for hz in [60., 120.] {
            let dt = 1. / hz;
            let mut speed = snow_carry(20.);
            let mut distance = 0.;
            for frame in 0..(hz as usize / 10) {
                speed -= 800. * SurfaceGrip::snow(0.).steering(speed, -1., frame as f32 * dt) * dt;
                distance += speed * dt;
            }
            assert!(speed > 15. && distance > 3.5);
        }
    }
    #[test]
    fn snow_and_ice_have_distinct_temporary_counter_steering() {
        fn distance(grip: Option<SurfaceGrip>, dt: f32) -> f32 {
            let (mut x, mut v, mut t) = (0., 150., 0.);
            while t < 0.15 - dt * 0.1 {
                let scale = grip.map_or(1., |g| g.steering(v, -1., t));
                v -= 800. * scale * dt;
                x += v * dt;
                t += dt;
            }
            x
        }
        for hz in [60., 120.] {
            let dry = distance(None, 1. / hz);
            let snow = distance(Some(SurfaceGrip::snow(0.)), 1. / hz);
            let ice = distance(Some(SurfaceGrip::ice(0.)), 1. / hz);
            assert!(snow > dry + 3. && ice > snow + 2.);
        }
        assert_eq!(SurfaceGrip::snow(0.).steering(150., -1., 0.7), 1.);
        assert_eq!(SurfaceGrip::snow(0.).steering(150., 1., 0.1), 1.);
        assert_eq!(snow_carry(0.), 0.);
        assert_eq!(snow_carry(500.), 500.);
        assert!((snow_carry(150.) - 177.).abs() < 0.01);
        assert_eq!(snow_carry(-150.), -snow_carry(150.));
        let mut speed = 150.;
        for _ in 0..100 {
            speed = snow_carry(speed);
        }
        assert_eq!(speed, 280.);
    }
}
