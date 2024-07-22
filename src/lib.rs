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
    pub replication_config: ReplicationConfig,
    pub interpolation_config: InterpolationConfig,
    pub prediction_config: PredictionConfig,
}

impl Plugin for NetworkBootPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.transform_axis.clone())
        .insert_resource(self.replication_config.clone())
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
        .configure_sets(PostUpdate, 
            ClientBootSet::Cache
            .before(ClientSet::Send)
        )
        .configure_sets(PreUpdate, 
            ServerBootSet::UnboxEvent
            .after(ServerSet::Receive)
        )
        .configure_sets(PreUpdate, 
            ServerBootSet::PlayerEntityEvent
            .after(ServerSet::Receive)
        )
        .configure_sets(PreUpdate, 
            ServerBootSet::CorrectReplication
            .after(ServerBootSet::UnboxEvent)
        )
        .configure_sets(PostUpdate, 
            ServerBootSet::Grouping
            .before(ServerSet::Send)
        )
        .configure_sets(PostUpdate, 
            ServerBootSet::Culling
            .before(ServerBootSet::Grouping)
        )
        .configure_sets(PostUpdate, 
            ServerBootSet::Cache
            .before(ServerSet::Send)
        )
        .configure_sets(PostUpdate, 
            ServerBootSet::ApplyLocalChange
            .before(ServerBootSet::Cache)
        )
        .replicate::<NetworkEntity>();

        if app.world().contains_resource::<RepliconClient>() {
            app.insert_resource(LatestConfirmedTick::default())
            .add_systems(PreUpdate, 
                latest_confirmed_tick_system
                .in_set(ClientBootSet::UnboxReplication)
            );
        }
    }
}

pub struct DefaultPlayerEntityEventPlugin;

impl Plugin for DefaultPlayerEntityEventPlugin {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<RepliconServer>() {
            app.insert_resource(PlayerEntityMap::default())
            .insert_resource(EntityPlayerMap::default())
            .add_event::<PlayerEntityEvent>()
            .add_systems(PreUpdate, 
                player_entity_event_system
                .in_set(ServerBootSet::PlayerEntityEvent)
            );
        } else {
            panic!("could not find replicon server");
        }
    }
}

pub struct NetworkTranslationPlugin<T>(PhantomData<T>)
where T: NetworkTranslation;

impl<T> NetworkTranslationPlugin<T>
where T: NetworkTranslation {
    
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<T>)
    }
}

impl<T> Plugin for NetworkTranslationPlugin<T>
where T: NetworkTranslation {
    fn build(&self, app: &mut App) {
        app.replicate::<T>()
        .add_plugins(ComponentSnapshotPlugin::<T>::new());

        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PostUpdate, 
                apply_transform_translation_system::<T>
                .in_set(ServerBootSet::ApplyLocalChange)
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, 
                apply_network_translation_system::<T>
                .in_set(ClientBootSet::ApplyReplication)
            )
            .add_systems(PostUpdate, 
                cache_translation_system::<T>
                .in_set(ClientBootSet::Cache)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkRotationPlugin<R>(PhantomData<R>)
where R: NetworkRotation;

impl<R> NetworkRotationPlugin<R>
where R: NetworkRotation {
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<R>)
    }
}

impl<R> Plugin for NetworkRotationPlugin<R>
where R: NetworkRotation {
    fn build(&self, app: &mut App) {
        app.replicate::<R>()
        .add_plugins(ComponentSnapshotPlugin::<R>::new());

        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PostUpdate, 
                apply_transform_rotation_system::<R>
                .in_set(ServerBootSet::ApplyLocalChange)
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, 
                apply_network_rotation_system::<R>
                .in_set(ClientBootSet::ApplyReplication)
            )
            .add_systems(PostUpdate, 
                cache_rotation_system::<R>
                .in_set(ClientBootSet::Cache)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkCharacterTranslationPlugin<T, E>(PhantomData<T>, PhantomData<E>)
where
T: NetworkTranslation,
E: NetworkMovement;

impl<T, E> NetworkCharacterTranslationPlugin<T, E>
where 
T: NetworkTranslation,
E: NetworkMovement {
    #[inline]
    fn new() -> Self {
        Self(PhantomData::<T>, PhantomData::<E>)
    }
} 

impl<T, E> Plugin for NetworkCharacterTranslationPlugin<T,E>
where 
T: NetworkTranslation,
E: NetworkMovement {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkTranslationPlugin::<T>::new())
        .add_server_event::<ForceReplicateTranslation<T>>(ChannelKind::Ordered);

        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PreUpdate,
                correct_translation_error_system::<T, E>
                .in_set(ServerBootSet::CorrectReplication)
            );   
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, (
                handle_correct_translation::<T, E>,
            ).in_set(ClientBootSet::ApplyReplication));
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkCharacterRotationPlugin<R, E>(PhantomData<R>, PhantomData<E>)
where 
R: NetworkRotation,
E: NetworkMovement;

impl<R, E> NetworkCharacterRotationPlugin<R, E>
where 
R: NetworkRotation,
E: NetworkMovement {
    #[inline]
    fn new() -> Self {
        Self(PhantomData::<R>, PhantomData::<E>)
    }
} 

impl<R, E> Plugin for NetworkCharacterRotationPlugin<R, E>
where 
R: NetworkRotation,
E: NetworkMovement {
    fn build(&self, app: &mut App) {
        app.add_plugins(NetworkRotationPlugin::<R>::new())
        .add_server_event::<ForceReplicateRotation<R>>(ChannelKind::Ordered);
    
        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(FixedPreUpdate, 
                correct_rotation_error_system::<R, E>
                .in_set(ServerBootSet::CorrectReplication)
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, (
                handle_correct_rotation::<R, E>
            ).in_set(ClientBootSet::ApplyReplication));
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkLinearVelocityPlugin<L>(PhantomData<L>)
where L: NetworkLinearVelocity;

impl<L> NetworkLinearVelocityPlugin<L>
where L: NetworkLinearVelocity {
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<L>)
    }
}

impl<L> Plugin for NetworkLinearVelocityPlugin<L>
where L: NetworkLinearVelocity {
    fn build(&self, app: &mut App) {
        app.replicate::<L>();

        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PostUpdate, 
                apply_rb_linear_velocity_system::<L>
                .in_set(ServerBootSet::ApplyLocalChange)
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, 
                apply_netrb_linear_velocity_system::<L>
                .in_set(ClientBootSet::ApplyReplication)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub struct NetworkAngularVelocityPlugin<A>(PhantomData<A>)
where A: NetworkAngularVelocity;

impl<A> NetworkAngularVelocityPlugin<A>
where A: NetworkAngularVelocity {
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<A>)
    }
}

impl<A> Plugin for NetworkAngularVelocityPlugin<A>
where A: NetworkAngularVelocity {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<RepliconServer>() {
            app.add_systems(PostUpdate, 
                apply_rb_angular_velocity_system::<A>
                .in_set(ServerBootSet::ApplyLocalChange)
            );
        } else if app.world().contains_resource::<RepliconClient>() {
            app.add_systems(PreUpdate, 
                apply_netrb_angular_velocity_system::<A>
                .in_set(ClientBootSet::ApplyReplication)
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
        app.add_plugins(ClientEventSnapshotPlugin::<E>::new())
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
