use crate::resources::command_queue::PowerCommand;
use bevy::prelude::*;

#[derive(Message)]
pub struct PowerActivated {
    pub command: PowerCommand,
}
