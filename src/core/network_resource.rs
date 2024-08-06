use bevy::prelude::*;
use bevy_replicon::prelude::*;

#[derive(Resource)]
pub struct Server;

#[derive(Resource)]
pub struct Client(u64);

impl Client {
    #[inline]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    #[inline]
    pub fn id(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn this_client(&self, client_id: &ClientId) -> bool {
        self.0 == client_id.get()
    }
}