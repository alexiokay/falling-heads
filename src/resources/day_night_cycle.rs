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

    pub fn is_night(&self) -> bool {
        !self.is_day()
    }

    pub fn get_time_of_day(&self) -> &str {
        if self.is_day() {
            "Day"
        } else {
            "Night"
        }
    }

    pub fn get_progress(&self) -> f32 {
        self.current_time / self.cycle_duration
    }

    pub fn update(&mut self, delta: f32) {
        self.current_time = (self.current_time + delta) % self.cycle_duration;
    }
}
