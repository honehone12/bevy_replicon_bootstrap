pub mod rapier3d;
pub mod network_rigidbody;
pub mod network_character_controller;

pub use rapier3d::*;
pub use network_rigidbody::*;
pub use network_character_controller::*;

use bevy::prelude::*;
use bevy_replicon::prelude::*;

pub struct NetworkPhysicsPlugin;

impl Plugin for NetworkPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<NetworkRigidBody>()
        .replicate::<NetworkCharacterController>();
    }
}