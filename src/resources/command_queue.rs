use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Resource, Default)]
pub struct CommandQueue {
    pub pending: VecDeque<PowerCommand>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerCommand {
    Tnt,
    Fast,
    Slow,
    Fuss,
    Bolt,
    Arrow,
    Laser,
    Ghast,
    Box,
    Magnet,
    Villagers, // Spawn 5 villagers around hero
}
