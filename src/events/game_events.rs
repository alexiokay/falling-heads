use bevy::prelude::*;

#[derive(Event)]
pub struct GameOver {
    pub final_score: u32,
}

#[derive(Event)]
pub struct ScoreChanged {
    pub new_score: u32,
}
