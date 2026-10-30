//! Authoritative biome environment values. Visual settings never change simulation.
use crate::components::block::BiomeType;
use bevy::prelude::Vec3;

/// Art direction is independent of temperature and simulation weather.
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    pub sun: f32,
    pub ambient: f32,
    pub environment: f32,
    pub key: Vec3,
    pub fill: Vec3,
    pub backdrop: Vec3,
    pub fog: Vec3,
}
impl Atmosphere {
    pub fn blend(self, to: Self, t: f32) -> Self {
        Self {
            sun: self.sun + (to.sun - self.sun) * t,
            ambient: self.ambient + (to.ambient - self.ambient) * t,
            environment: self.environment + (to.environment - self.environment) * t,
            key: self.key.lerp(to.key, t),
            fill: self.fill.lerp(to.fill, t),
            backdrop: self.backdrop.lerp(to.backdrop, t),
            fog: self.fog.lerp(to.fog, t),
        }
    }
}
pub fn atmosphere(b: BiomeType, day: f32) -> Atmosphere {
    // Nether, End and deep caves have their own illumination, not an outdoor sun.
    let (night, daylight, key, fill, backdrop, fog) = match b {
        BiomeType::Plains => (
            [800., 180., 240.],
            [5400., 380., 800.],
            [1., 0.95, 0.86],
            [0.79, 0.87, 1.],
            [0.37, 0.38, 0.36],
            [0.18, 0.23, 0.27],
        ),
        BiomeType::Forest => (
            [500., 135., 170.],
            [2200., 240., 430.],
            [0.83, 0.91, 0.73],
            [0.55, 0.72, 0.64],
            [0.25, 0.31, 0.25],
            [0.12, 0.21, 0.17],
        ),
        BiomeType::Desert => (
            [750., 170., 230.],
            [5700., 340., 730.],
            [1., 0.89, 0.69],
            [0.90, 0.78, 0.62],
            [0.40, 0.34, 0.25],
            [0.30, 0.24, 0.16],
        ),
        BiomeType::Mountains => (
            [700., 155., 220.],
            [3900., 290., 610.],
            [0.80, 0.88, 1.],
            [0.63, 0.75, 0.92],
            [0.29, 0.34, 0.39],
            [0.19, 0.25, 0.34],
        ),
        BiomeType::Underground => (
            [550., 120., 190.],
            [550., 120., 190.],
            [0.66, 0.75, 0.85],
            [0.44, 0.53, 0.65],
            [0.25, 0.27, 0.30],
            [0.07, 0.10, 0.13],
        ),
        BiomeType::Nether => (
            [650., 105., 140.],
            [650., 105., 140.],
            [1., 0.48, 0.20],
            [0.65, 0.26, 0.12],
            [0.23, 0.16, 0.14],
            [0.11, 0.035, 0.018],
        ),
        BiomeType::End => (
            [650., 130., 190.],
            [650., 130., 190.],
            [0.70, 0.61, 0.94],
            [0.46, 0.39, 0.65],
            [0.25, 0.20, 0.32],
            [0.075, 0.045, 0.13],
        ),
        BiomeType::Iceland => (
            [800., 180., 260.],
            [4000., 320., 690.],
            [0.76, 0.88, 1.],
            [0.64, 0.82, 1.],
            [0.30, 0.37, 0.43],
            [0.22, 0.33, 0.42],
        ),
    };
    let day = day.clamp(0., 1.);
    let outdoor = !matches!(
        b,
        BiomeType::Nether | BiomeType::End | BiomeType::Underground
    );
    let night_tint = if outdoor {
        Vec3::new(0.65, 0.76, 1.).lerp(Vec3::ONE, day)
    } else {
        Vec3::ONE
    };
    Atmosphere {
        sun: night[0] + (daylight[0] - night[0]) * day,
        ambient: night[1] + (daylight[1] - night[1]) * day,
        environment: night[2] + (daylight[2] - night[2]) * day,
        key: Vec3::from_array(key) * night_tint,
        fill: Vec3::from_array(fill) * night_tint,
        backdrop: Vec3::from_array(backdrop) * if outdoor { 0.82 + day * 0.18 } else { 1. },
        fog: Vec3::from_array(fog),
    }
}

pub fn mist_style(b: BiomeType) -> (Vec3, f32) {
    let (color, strength) = match b {
        BiomeType::Forest => ([0.48, 0.53, 0.50], 0.48),
        BiomeType::Underground => ([0.16, 0.22, 0.27], 0.12),
        BiomeType::Iceland => ([0.31, 0.43, 0.54], 0.19),
        BiomeType::Mountains => ([0.25, 0.32, 0.40], 0.10),
        BiomeType::Desert => ([0.36, 0.27, 0.16], 0.07),
        BiomeType::Nether => ([0.25, 0.09, 0.025], 0.055),
        BiomeType::End => ([0.18, 0.10, 0.29], 0.08),
        BiomeType::Plains => ([0.26, 0.32, 0.31], 0.035),
    };
    (Vec3::from_array(color), strength)
}
#[derive(Clone, Copy, Debug)]
pub struct Climate {
    pub temperature_c: f32,
    pub humidity: f32,
    pub haze: f32,
    pub wind: f32,
    pub tint: [f32; 3],
}
pub fn climate(b: BiomeType) -> Climate {
    let (temperature_c, humidity, haze, wind, tint) = match b {
        BiomeType::Plains => (20., 0.5, 0.08, 0.4, [0.95, 0.98, 1.]),
        BiomeType::Forest => (18., 0.85, 0.20, 0.2, [0.80, 0.95, 0.86]),
        BiomeType::Desert => (38., 0.12, 0.15, 0.65, [1., 0.83, 0.61]),
        BiomeType::Mountains => (2., 0.4, 0.24, 0.9, [0.76, 0.86, 1.]),
        BiomeType::Underground => (13., 0.72, 0.30, 0.08, [0.75, 0.81, 0.90]),
        BiomeType::Nether => (65., 0.04, 0.46, 0.45, [1., 0.39, 0.18]),
        BiomeType::End => (5., 0.16, 0.36, 0.35, [0.65, 0.55, 0.98]),
        BiomeType::Iceland => (-15., 0.35, 0.28, 0.75, [0.65, 0.83, 1.]),
    };
    Climate {
        temperature_c,
        humidity,
        haze,
        wind,
        tint,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn climate_extremes_are_explicit() {
        assert!(climate(BiomeType::Iceland).temperature_c < 0.);
        assert!(climate(BiomeType::Desert).humidity < climate(BiomeType::Forest).humidity);
    }
    #[test]
    fn enclosed_realms_do_not_receive_the_outdoor_daylight_wash() {
        for biome in [BiomeType::Nether, BiomeType::End, BiomeType::Underground] {
            let a = atmosphere(biome, 0.);
            let b = atmosphere(biome, 1.);
            assert_eq!(a.sun, b.sun);
            assert_eq!(a.fill, b.fill);
            assert_eq!(a.backdrop, b.backdrop);
            assert!(a.sun < atmosphere(BiomeType::Plains, 1.).sun / 4.);
        }
        assert!(atmosphere(BiomeType::Forest, 1.).sun < atmosphere(BiomeType::Desert, 1.).sun);
        assert!(mist_style(BiomeType::Forest).1 > mist_style(BiomeType::Plains).1);
    }
}
