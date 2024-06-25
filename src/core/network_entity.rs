use serde::{Serialize, Deserialize};
use bevy::prelude::*;
use bevy_replicon::prelude::*;

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
