pub mod component_snapshot;
pub mod event_snapshot;

use std::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::NetworkEvent;

pub use component_snapshot::*;
pub use event_snapshot::*;

pub struct EventSnapshotPlugin<E: NetworkEvent>{
    pub channel_kind: ChannelKind,
    phantom: PhantomData<E>
}

impl<E: NetworkEvent> EventSnapshotPlugin<E> {
    #[inline]
    pub fn new(channel_kind: ChannelKind) -> Self {
        Self { 
            channel_kind, 
            phantom: PhantomData::<E> 
        }
    }
} 

impl<E: NetworkEvent> Plugin for EventSnapshotPlugin<E> {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.add_client_event::<E>(self.channel_kind)
            .add_systems(PreUpdate, 
                server_populate_client_event_snapshots::<E>
                .after(ServerSet::Receive)    
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            app.add_client_event::<E>(self.channel_kind)
            .add_systems(PostUpdate, 
                client_populate_client_event_snapshots::<E>
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct ComponentSnapshotPlugin<C>(PhantomData<C>)
where C: Component + Serialize + DeserializeOwned + Clone;

impl<C> ComponentSnapshotPlugin<C>
where C: Component + Serialize + DeserializeOwned + Clone {
    pub fn new() -> Self {
        Self(PhantomData::<C>)
    }
}

impl<C> Plugin for ComponentSnapshotPlugin<C>
where C: Component + Serialize + DeserializeOwned + Clone {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.replicate::<C>()
            .add_systems(PostUpdate,
                server_populate_component_snapshots::<C>
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            app.replicate::<C>()
            .add_systems(PreUpdate, 
                client_populate_component_snapshots::<C>
                .after(ClientSet::Receive)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
