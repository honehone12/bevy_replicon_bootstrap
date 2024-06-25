use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Component, Serialize, Deserialize, Clone, Default)]
pub struct NetworkCharacterController(pub Vec3);
