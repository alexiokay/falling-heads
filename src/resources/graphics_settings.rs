use bevy::prelude::*;
use serde::{Deserialize, Serialize};

const GRAPHICS_SETTINGS_FILE: &str = "graphics-settings.json";
/// Fields missing from an older or newer save keep their defaults instead of
/// the whole file being rejected.
#[derive(Resource, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct GraphicsSettings {
    pub lighting: bool,
    pub dlss: bool,
    #[serde(default)]
    pub dlss_45_performance: bool,
    #[serde(default)]
    pub dlss_model_45: bool,
    #[serde(default)]
    pub dlss_balanced: bool,
    #[serde(default)]
    pub frame_generation: bool,
    pub ray_tracing: bool,
    pub shadows: bool,
    pub ambient_occlusion: bool,
    pub bloom: bool,
    pub fog: bool,
    pub weather_lighting: bool,
    pub local_lights: bool,
    pub anti_aliasing: bool,
    pub weather_particles: bool,
    pub distortion: bool,
    pub liquid_animation: bool,
}
impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            lighting: true,
            dlss: true,
            dlss_45_performance: false,
            dlss_model_45: true,
            dlss_balanced: false,
            frame_generation: false,
            ray_tracing: false,
            shadows: true,
            ambient_occlusion: true,
            bloom: true,
            fog: true,
            weather_lighting: true,
            local_lights: true,
            anti_aliasing: true,
            weather_particles: true,
            distortion: true,
            liquid_animation: true,
        }
    }
}
impl GraphicsSettings {
    pub fn load() -> Self {
        crate::resources::settings::read_settings_file(GRAPHICS_SETTINGS_FILE)
            .and_then(|s| Self::from_json(&s))
            .unwrap_or_default()
    }
    pub fn save(&self) {
        if !crate::resources::settings::persistence_enabled() {
            return;
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            crate::resources::settings::write_settings_file(GRAPHICS_SETTINGS_FILE, &s);
        }
    }
    fn from_json(s: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(s).ok()?;
        let mut settings: Self = serde_json::from_value(value.clone()).ok()?;
        // One supported SR model; retain the saved resolution preference.
        settings.dlss_model_45 = true;
        Some(settings)
    }
    pub fn quality_step(&self) -> u8 {
        if self.dlss_45_performance {
            2
        } else if self.dlss_balanced {
            1
        } else {
            0
        }
    }
    pub fn set_quality_step(&mut self, n: u8) {
        self.dlss_45_performance = n == 2;
        self.dlss_balanced = n == 1;
    }
    pub fn quality_name(&self) -> &'static str {
        match self.quality_step() {
            0 => "Quality",
            1 => "Balanced",
            _ => "Performance",
        }
    }
    pub fn label(&self, n: usize) -> String {
        let value = match n {
            14 => {
                if self.dlss_45_performance {
                    "Performance"
                } else {
                    "Quality"
                }
            }
            15 => {
                if self.dlss_model_45 {
                    "4.5 (M)"
                } else {
                    "4 (K)"
                }
            }
            _ => {
                if self.value(n) {
                    "ON"
                } else {
                    "OFF"
                }
            }
        };
        format!("{}: {}", LABELS[n], value)
    }
    pub fn value(&self, n: usize) -> bool {
        match n {
            0 => self.lighting,
            1 => self.shadows,
            2 => self.ambient_occlusion,
            3 => self.bloom,
            4 => self.fog,
            5 => self.weather_lighting,
            6 => self.local_lights,
            7 => self.anti_aliasing,
            8 => self.weather_particles,
            9 => self.distortion,
            10 => self.liquid_animation,
            11 => self.dlss,
            12 => self.ray_tracing,
            13 => self.frame_generation,
            14 => self.dlss_45_performance,
            15 => self.dlss_model_45,
            _ => panic!("Invalid graphics setting"),
        }
    }
    pub fn toggle(&mut self, n: usize) {
        let p = match n {
            0 => &mut self.lighting,
            1 => &mut self.shadows,
            2 => &mut self.ambient_occlusion,
            3 => &mut self.bloom,
            4 => &mut self.fog,
            5 => &mut self.weather_lighting,
            6 => &mut self.local_lights,
            7 => &mut self.anti_aliasing,
            8 => &mut self.weather_particles,
            9 => &mut self.distortion,
            10 => &mut self.liquid_animation,
            11 => &mut self.dlss,
            12 => &mut self.ray_tracing,
            13 => &mut self.frame_generation,
            14 => &mut self.dlss_45_performance,
            15 => &mut self.dlss_model_45,
            _ => panic!("Invalid graphics setting"),
        };
        *p = !*p;
    }
}
pub const LABELS: [&str; 16] = [
    "Lighting",
    "Shadows",
    "Ambient occlusion",
    "Bloom",
    "Atmospheric haze",
    "Weather / moon lighting",
    "Torches / lava / glowstone",
    "Anti-aliasing (FXAA)",
    "Weather particles",
    "Teleport distortion",
    "Animated liquids",
    "DLSS upscaling",
    "Ray tracing (Solari)",
    "NVIDIA frame generation (2x)",
    "DLSS resolution (RT off)",
    "DLSS SR model (RT off)",
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_slider_stops_roundtrip_without_affecting_ray_tracing() {
        for step in 0..3 {
            let mut s = GraphicsSettings::default();
            s.ray_tracing = true;
            s.set_quality_step(step);
            let loaded = GraphicsSettings::from_json(&serde_json::to_string(&s).unwrap()).unwrap();
            assert_eq!(loaded.quality_step(), step);
            assert!(loaded.ray_tracing);
            assert!(loaded.dlss_model_45);
        }
    }
    #[test]
    fn old_performance_setting_migrates_to_model_m_and_quality_is_independent() {
        let mut value = serde_json::to_value(GraphicsSettings::default()).unwrap();
        value.as_object_mut().unwrap().remove("dlss_model_45");
        value["dlss_45_performance"] = true.into();
        let mut settings = GraphicsSettings::from_json(&value.to_string()).unwrap();
        assert!(settings.dlss_model_45);
        settings.toggle(14);
        assert!(!settings.dlss_45_performance);
        assert!(settings.dlss_model_45);
        assert!(settings.label(14).ends_with("Quality"));
    }
    #[test]
    fn a_save_missing_any_field_keeps_the_other_choices() {
        let mut old = serde_json::to_value(GraphicsSettings {
            bloom: false,
            ..GraphicsSettings::default()
        })
        .unwrap();
        old.as_object_mut().unwrap().remove("shadows");
        let restored = GraphicsSettings::from_json(&old.to_string()).unwrap();
        assert!(!restored.bloom, "saved choice kept");
        assert!(restored.shadows, "missing field uses its default");
    }
    #[test]
    fn settings_from_before_frame_generation_keep_user_preferences() {
        let mut old = serde_json::to_value(GraphicsSettings::default()).unwrap();
        old.as_object_mut().unwrap().remove("frame_generation");
        old.as_object_mut().unwrap().remove("dlss_45_performance");
        old["bloom"] = false.into();
        old["dlss"] = false.into();
        let restored: GraphicsSettings = serde_json::from_value(old).unwrap();
        assert!(!restored.frame_generation);
        assert!(!restored.dlss_45_performance);
        assert!(!restored.bloom);
        assert!(!restored.dlss);
        assert!(restored.shadows);
    }
    #[test]
    fn graphics_switches_roundtrip_and_are_independent() {
        let mut s = GraphicsSettings::default();
        s.ray_tracing = true;
        s.frame_generation = true;
        s.dlss_45_performance = true;
        s.dlss_model_45 = true;
        for i in 0..LABELS.len() {
            s.toggle(i);
            assert!(!s.value(i));
        }
        let json = serde_json::to_string(&s).unwrap();
        let restored: GraphicsSettings = serde_json::from_str(&json).unwrap();
        for i in 0..LABELS.len() {
            assert!(!restored.value(i));
        }
        s.toggle(0);
        assert!(s.lighting);
        assert!(!s.shadows);
    }
}
