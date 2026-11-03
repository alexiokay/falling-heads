use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    /// The space prison event (`!PRISON`); the world is paused (and silent) meanwhile.
    Prison,
    Paused,
    Settings,
}

/// The game world is running: normal play. The space prison pauses it, like the
/// pause menu. World systems (physics, AI, powers, chunks)
/// use this; camera, hero keyboard input and the HUD stay `Playing`-only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InGame;

impl ComputedStates for InGame {
    type SourceStates = GameState;

    fn compute(state: GameState) -> Option<Self> {
        (state == GameState::Playing).then_some(InGame)
    }
}
