use bevy::prelude::*;

#[derive(Resource)]
pub struct DayNightCycle {
    pub cycle_duration: f32, // Total cycle duration in seconds
    pub current_time: f32,   // Current time in the cycle (0.0 to cycle_duration)
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            cycle_duration: 300.0, // 5 minutes per full cycle
            current_time: 0.0,
        }
    }
}

impl DayNightCycle {
    pub fn is_day(&self) -> bool {
        self.current_time < (self.cycle_duration / 2.0)
    }

    pub fn update(&mut self, delta: f32) {
        self.current_time = (self.current_time + delta) % self.cycle_duration;
    }
}
