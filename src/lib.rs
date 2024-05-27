pub mod dev;
pub mod component_snapshot; 
pub mod event_snapshot;
pub mod interpolation;
pub mod network_event;
pub mod network_entity;
pub mod prediction;
pub mod culling;
pub mod client_builder;
pub mod server_builder;
pub mod network_transform;
pub mod player_entity;
pub mod relevancy;

pub mod prelude {
    pub use crate::{
        network_entity::*,
        network_event::*,
        network_transform::*,
        component_snapshot::*, 
        event_snapshot::*,
        interpolation::*,
        prediction::*,
        culling::*,
        player_entity::*,
        server_builder::*,
        client_builder::*,
        relevancy::*,
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

        if app.world.contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, 
                mark_owning_system
                .after(ClientSet::Receive)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unimplemented_test() {
        unimplemented!("tests are not ready");
    }
}
