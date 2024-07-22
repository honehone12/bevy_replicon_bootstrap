pub mod component_cache;
pub mod event_cache;

use std::marker::PhantomData;
use serde::{Serialize, de::DeserializeOwned};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::{ClientBootSet, NetworkEvent, ServerBootSet};

pub use component_cache::*;
pub use event_cache::*;

pub struct ClientEventCachePlugin<E: NetworkEvent>(PhantomData<E>);

impl<E: NetworkEvent> ClientEventCachePlugin<E> {
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<E>)
    }
} 

impl<E: NetworkEvent> Plugin for ClientEventCachePlugin<E> {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PreUpdate, 
                server_populate_client_event_cache::<E>
                .in_set(ServerBootSet::UnboxEvent)    
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PostUpdate, 
                client_populate_client_event_cache::<E>
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct ComponentCachePlugin<C>(PhantomData<C>)
where C: Component + Serialize + DeserializeOwned + Clone;

impl<C> ComponentCachePlugin<C>
where C: Component + Serialize + DeserializeOwned + Clone {
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<C>)
    }
}

impl<C> Plugin for ComponentCachePlugin<C>
where C: Component + Serialize + DeserializeOwned + Clone {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PostUpdate,
                server_populate_component_cache::<C>
                .in_set(ServerBootSet::Cache)
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, 
                client_populate_component_cache::<C>
                .in_set(ClientBootSet::UnboxReplication)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
