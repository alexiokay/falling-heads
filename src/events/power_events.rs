use bevy::prelude::*;
use crate::resources::command_queue::PowerCommand;

#[derive(Message)]
pub struct PowerActivated {
    pub command: PowerCommand,
}
