use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;

#[derive(Component, Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Copy)]
pub struct NetworkEntity(ClientId);

impl NetworkEntity {
    #[inline]
    pub fn new(client_id: ClientId) -> Self {
        Self(client_id)
    }

    #[inline]
    pub fn client_id(&self) -> ClientId {
        self.0
    }
}

#[derive(Component)]
pub struct PlayerView;

#[derive(Component)]
pub struct Owning;

pub(crate) fn mark_owning_system(
    mut commands: Commands,
    query: Query<(Entity, &NetworkEntity), Added<NetworkEntity>>,
    client: Res<Client>
) {
    for (e, net_e) in query.iter() {
        if net_e.client_id().get() == client.id() {
            commands.entity(e)
            .insert(Owning);
        }     
    }
}