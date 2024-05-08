use bevy::{prelude::*, utils::HashMap};
use bevy_replicon::core::ClientId;
use anyhow::bail;

#[derive(Resource, Default)]
pub struct PlayerEntityMap(HashMap<ClientId, Entity>);

impl PlayerEntityMap {
    #[inline]
    pub fn try_insert(&mut self, client_id: ClientId, entity: Entity)
    -> anyhow::Result<()> {
        match self.0.try_insert(client_id, entity) {
            Ok(_) => Ok(()),
            Err(e) => bail!("{e}") 
        }
    }

    #[inline]
    pub fn get(&self, client_id: &ClientId) -> Option<&Entity> {
        self.0.get(client_id)
    }

    #[inline]
    pub fn remove(&mut self, client_id: &ClientId) {
        self.0.remove(client_id);
    }
}

#[derive(Resource, Default)]
pub struct PlayerEntitiesMap(HashMap<ClientId, Vec<Entity>>);

impl PlayerEntitiesMap {
    #[inline]
    pub fn insert(&mut self, client_id: ClientId, entity: Entity) {
        match self.0.get_mut(&client_id) {
            Some(v) => {
                v.push(entity);
            }
            None => {
                self.0.insert(client_id, vec![entity]);
            }
        }
    }

    #[inline]
    pub fn get_mut(&mut self, client_id: &ClientId) -> Option<&mut Vec<Entity>> {
        self.0.get_mut(client_id)
    }

    #[inline]
    pub fn clear(&mut self, client_id: &ClientId) {
        if let Some(v) = self.0.get_mut(client_id) {
            v.clear();
            self.0.remove(client_id);
        }
    }
}
