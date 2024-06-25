use anyhow::bail;
use bevy::{
    prelude::*,
    utils::HashMap
};
use bevy_replicon::prelude::*;
use super::network_entity::NetworkEntity;

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

#[derive(SystemSet, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PlayerEntityEventSet;

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

fn handle_server_event(
    mut commands: Commands,
    mut server_evetns: EventReader<ServerEvent>,
    mut player_entity_events: EventWriter<PlayerEntityEvent>, 
    mut player_entities: ResMut<PlayerEntitiesMap>,
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
                player_entities.insert(client_id, entity);
                
                let visibility = match connected_clients.get_client_mut(client_id) {
                    Some(c) => c.visibility_mut(),
                    None => {
                        // this is fatal, client can not see it's entity
                        panic!("could not find client vivibility, wrong scheduling?");
                    }
                };
                visibility.set_visibility(entity, true);

                player_entity_events.send(PlayerEntityEvent::Spawned { 
                    client_id, 
                    entity
                });
            }
            &ServerEvent::ClientDisconnected { client_id, reason: _ } => {
                if let Some(v) = player_entities.get(&client_id) {
                    for &entity in v.iter() {
                        commands.entity(entity)
                        .despawn();
                        player_entity_events.send(PlayerEntityEvent::Despawned{
                            client_id,
                            entity
                        });
                    }
                    player_entities.clear(&client_id);
                }
            }
        }
    }
} 

pub trait PlayerEntityAppExt {
    fn use_player_entity_event(&mut self) -> &mut Self;
}

impl PlayerEntityAppExt for App {
    fn use_player_entity_event(&mut self) -> &mut Self {
        if self.world.contains_resource::<RepliconServer>() {
            self.configure_sets(PreUpdate, 
                PlayerEntityEventSet
                .after(ServerSet::Receive)
            )
            .insert_resource(PlayerEntitiesMap::default())
            .add_event::<PlayerEntityEvent>()
            .add_systems(PreUpdate, 
                handle_server_event
                .after(PlayerEntityEventSet)
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}