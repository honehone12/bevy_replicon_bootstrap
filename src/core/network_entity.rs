use bevy::prelude::*;
use bevy_replicon::prelude::*;

#[derive(Component)]
pub struct NetworkEntity(ClientId);

impl NetworkEntity {
    #[inline]
    pub fn new(client_id: &ClientId) -> Self {
        Self(*client_id)
    }

    #[inline]
    pub fn client_id(&self) -> ClientId {
        self.0
    }
}

#[derive(Component)]
pub struct Owning;
