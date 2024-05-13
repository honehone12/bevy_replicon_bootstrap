pub mod dev;
pub mod quick_net;
pub mod core;

pub mod prelude {
    pub use crate::{
        core::{
            component_snapshot::*, 
            event_snapshot::*,
            interpolation::*,
            network_event::*,
            player_entity_map::*,
            network_entity::*
        },
        quick_net::{
            client::*,
            server::*,
            network_transform::*,
        }
    };
}

use core::network_entity::NetworkEntity;
use bevy::prelude::*;
use bevy_replicon::prelude::*;

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
