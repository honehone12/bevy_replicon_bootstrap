use anyhow::bail;
use bevy::{
    prelude::*,
    utils::HashMap
};
use bevy_replicon::prelude::*;
use super::network_entity::NetworkEntity;

#[derive(Resource, Default)]
pub struct EntityPlayerMap(HashMap<Entity, ClientId>);

impl EntityPlayerMap {
    #[inline]
    pub fn try_insert(&mut self, entity: Entity, client_id: ClientId)
    -> anyhow::Result<()> {
        match self.0.try_insert(entity, client_id) {
            Ok(_) => Ok(()),
            Err(e) => bail!("{e}")
        }
    }

    #[inline]
    pub fn get(&self, entity: &Entity) -> Option<&ClientId> {
        self.0.get(entity)
    }

    #[inline]
    pub fn remove(&mut self, entity: &Entity) {
        self.0.remove(entity);
    }
}

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
        let v = self.0.entry(client_id)
        .or_insert(default());
        v.push(entity);
    }

    #[inline]
    pub fn get(&mut self, client_id: &ClientId) -> Option<&Vec<Entity>> {
        self.0.get(client_id)
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

#[derive(Event)]
pub enum PlayerEntityEvent {
    Spawned {
        client_id: ClientId,
        entity: Entity
    },
    Despawned {
        client_id: ClientId, 
        entity: Entity
    }
}

pub(crate) fn player_entity_event_system(
    mut commands: Commands,
    mut server_evetns: EventReader<ServerEvent>,
    mut player_entity_events: EventWriter<PlayerEntityEvent>, 
    mut player_entity_map: ResMut<PlayerEntityMap>,
    mut entity_player_map: ResMut<EntityPlayerMap>,
    mut connected_clients: ResMut<ConnectedClients>
) {
    for e in server_evetns.read() {
        match e {
            &ServerEvent::ClientConnected { client_id } => {
                let entity = commands.spawn((
                    NetworkEntity::new(client_id),
                    Replicated,
                ))
                .id();
                if let Err(e) = player_entity_map.try_insert(client_id, entity) {
                    // fatal
                    panic!("same client id is already connected, {e}");
                }

                if let Err(e) = entity_player_map.try_insert(entity, client_id) {
                    // fatal
                    panic!("same entity is already mapped, {e}");
                }
                
                let visibility = match connected_clients.get_client_mut(client_id) {
                    Some(c) => c.visibility_mut(),
                    // fatal
                    None => panic!("could not find client vivibility, wrong scheduling?")
                };
                visibility.set_visibility(entity, true);

                player_entity_events.send(PlayerEntityEvent::Spawned { 
                    client_id, 
                    entity
                });
            }
            &ServerEvent::ClientDisconnected { client_id, reason: _ } => {
                if let Some(e) = player_entity_map.get(&client_id) {
                    commands.entity(*e)
                    .despawn();
                    
                    player_entity_events.send(PlayerEntityEvent::Despawned{
                        client_id,
                        entity: *e
                    });

                    entity_player_map.remove(e);
                    player_entity_map.remove(&client_id);
                }
            }
        }
    }
} 
