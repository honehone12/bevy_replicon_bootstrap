pub mod dev;
pub mod core;
pub mod physics;
pub mod network_transform;
pub mod snapshot; 
pub mod culling;
pub mod character_movement_systems;
pub mod net_builder;

pub mod prelude {
    pub use crate::{
        core::*,
        physics::*,
        network_transform::*,
        snapshot::*,
        culling::*,
        character_movement_systems::*,
        net_builder::*,
        RepliconActionPlugin
    };
}

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use prelude::*;

pub struct RepliconActionPlugin;

impl Plugin for RepliconActionPlugin {
    fn build(&self, app: &mut App) {
        app.use_player_entity_event()
        .replicate::<NetworkEntity>();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unimplemented_test() {
        unimplemented!("tests are not ready");
    }
}
