use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    Rift,
    Paused,
    Settings,
}

/// The game world is running: normal play and the Slime Rift, whose course is
/// shown in front of the live world. World systems (physics, AI, powers, chunks)
/// use this; camera, hero keyboard input and the HUD stay `Playing`-only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InGame;

impl ComputedStates for InGame {
    type SourceStates = GameState;

    fn compute(state: GameState) -> Option<Self> {
        matches!(state, GameState::Playing | GameState::Rift).then_some(InGame)
    }
}
