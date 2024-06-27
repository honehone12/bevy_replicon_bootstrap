pub mod dev;
pub mod core;
pub mod physics;
pub mod control;
pub mod snapshot; 
pub mod culling;
pub mod net_builder;

pub mod prelude {
    pub use crate::{
        core::*,
        physics::*,
        control::*,
        snapshot::*,
        culling::*,
        net_builder::*,
        *
    };
}

use std::marker::PhantomData;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use prelude::*;

pub struct NetworkBootPlugin {
    pub transform_axis: TransformAxis,
    pub interpolation_config: InterpolationConfig,
    pub prediction_config: PredictionConfig,
}

impl Plugin for NetworkBootPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.transform_axis.clone())
        .insert_resource(self.interpolation_config.clone())
        .insert_resource(self.prediction_config.clone())
        .configure_sets(PreUpdate, 
            ClientBootSet::UnboxReplication
            .after(ClientSet::Receive)
        )
        .configure_sets(PreUpdate, 
            ClientBootSet::ApplyReplication
            .after(ClientBootSet::UnboxReplication)
        )
        .configure_sets(FixedUpdate, 
            ClientBootSet::Update
            .before(BEFORE_PHYSICS_SET)
        )
        .configure_sets(PostUpdate, 
            ClientBootSet::CacheLocalChange
            .before(ClientSet::Send)
        )
        .configure_sets(PreUpdate, 
            ServerBootSet::UnboxEvent
            .after(ServerSet::Receive)
        )
        .configure_sets(FixedPreUpdate, 
            ServerBootSet::CorrectReplication
        )
        .configure_sets(FixedUpdate, 
            ServerBootSet::Update
            .before(BEFORE_PHYSICS_SET)
        )
        .configure_sets(PostUpdate, 
            ServerBootSet::Cache
            .before(ServerSet::Send)
        )
        .configure_sets(FixedPostUpdate, 
            ServerBootSet::ApplyLocalChange
        )
        .add_plugins(PlayerEntityEventPlugin)
        .replicate::<NetworkEntity>();
    }
}

pub struct PlayerEntityEventPlugin;

impl Plugin for PlayerEntityEventPlugin {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.insert_resource(PlayerEntitiesMap::default())
            .add_event::<PlayerEntityEvent>()
            .add_systems(PreUpdate, 
                player_entity_event_system
                .in_set(ServerBootSet::UnboxEvent)
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkTranslationPlugin<T, E>(PhantomData<T>, PhantomData<E>)
where
T: NetworkTranslation,
E: NetworkMovement;

impl<T, E> NetworkTranslationPlugin<T, E>
where 
T: NetworkTranslation,
E: NetworkMovement {
    #[inline]
    fn new() -> Self {
        Self(PhantomData::<T>, PhantomData::<E>)
    }
} 

impl<T, E> Plugin for NetworkTranslationPlugin<T,E>
where 
T: NetworkTranslation,
E: NetworkMovement {
    fn build(&self, app: &mut App) {
        app.replicate::<T>()
        .add_plugins(ComponentSnapshotPlugin::<T>::new())
        .add_server_event::<ForceReplicateTranslation<T>>(ChannelKind::Ordered);

        if app.world.contains_resource::<RepliconServer>() {
            app.add_systems(FixedPreUpdate,
                correct_translation_error_system::<T, E>
                .in_set(ServerBootSet::CorrectReplication)
            )
            .add_systems(FixedPostUpdate,
                apply_transform_translation_system::<T>
                .in_set(ServerBootSet::ApplyLocalChange)
            );   
        } else if app.world.contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, (
                handle_correct_translation::<T>,
                apply_network_translation_system::<T>
            ).in_set(ClientBootSet::ApplyReplication))
            .add_systems(PostUpdate, 
                cache_translation_system::<T>
                .in_set(ClientBootSet::CacheLocalChange)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkRotationPlugin<R, E>(PhantomData<R>, PhantomData<E>)
where 
R: NetworkRotation,
E: NetworkMovement;

impl<R, E> NetworkRotationPlugin<R, E>
where 
R: NetworkRotation,
E: NetworkMovement {
    #[inline]
    fn new() -> Self {
        Self(PhantomData::<R>, PhantomData::<E>)
    }
} 

impl<R, E> Plugin for NetworkRotationPlugin<R, E>
where 
R: NetworkRotation,
E: NetworkMovement {
    fn build(&self, app: &mut App) {
        app.replicate::<R>()
        .add_plugins(ComponentSnapshotPlugin::<R>::new())
        .add_server_event::<ForceReplicateRotation<R>>(ChannelKind::Ordered);
    
        if app.world.contains_resource::<RepliconServer>() {
            app.add_systems(FixedPreUpdate, 
                correct_rotation_error_system::<R, E>
                .in_set(ServerBootSet::CorrectReplication)
            )
            .add_systems(FixedPostUpdate, 
                apply_transform_rotation_system::<R>
                .in_set(ServerBootSet::ApplyLocalChange)
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, (
                handle_correct_rotation::<R>,
                apply_network_rotation_system::<R>
            ).in_set(ClientBootSet::ApplyReplication))
            .add_systems(PostUpdate,
                cache_rotation_system::<R>
                .in_set(ClientBootSet::CacheLocalChange)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct ClientEventPlugin<E: NetworkEvent>{
    pub channel_kind: ChannelKind,
    phantom: PhantomData<E>
}

impl<E: NetworkEvent> ClientEventPlugin<E> {
    #[inline]
    pub fn new(channel_kind: ChannelKind) -> Self {
        Self { 
            channel_kind, 
            phantom: PhantomData::<E> 
        }
    }
}

impl<E: NetworkEvent> Plugin for ClientEventPlugin<E> {
    fn build(&self, app: &mut App) {
        app.add_plugins(EventSnapshotPlugin::<E>::new())
        .add_client_event::<E>(self.channel_kind);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unimplemented_test() {
        unimplemented!("can you help me ??");
    }
}
