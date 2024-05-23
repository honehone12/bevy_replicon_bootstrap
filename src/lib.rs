pub mod dev;
pub mod quick_lib;
pub mod core;

pub mod prelude {
    pub use crate::{
        core::{
            component_snapshot::*, 
            event_snapshot::*,
            interpolation::*,
            network_event::*,
            player_entity_map::*,
            network_entity::*,
            prediction::*,
            importance::*
        },
        quick_lib::{
            client_builder::*,
            server_builder::*,
            network_transform::*,
            distance_culling::*,
            player_entity::*
        },
        RepliconActionPlugin
    };
}

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use prelude::*;

pub struct RepliconActionPlugin;

impl Plugin for RepliconActionPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<NetworkEntity>();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unimplemented_test() {
        unimplemented!("tests are not ready");
    }
}
