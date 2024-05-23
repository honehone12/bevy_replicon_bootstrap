use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::prelude::*;

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
    mut player_entities: ResMut<PlayerEntitiesMap>
) {
    for e in server_evetns.read() {
        match e {
            &ServerEvent::ClientConnected { client_id } => {
                let entity = commands.spawn((
                    NetworkEntity::new(client_id),
                    Replicated,
                    PlayerView,
                    Importance::<Distance>::default(),
                ))
                .id();
                player_entities.insert(client_id, entity);
                
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
            self.insert_resource(PlayerEntitiesMap::default())
            .add_event::<PlayerEntityEvent>()
            .add_systems(PreUpdate, 
                handle_server_event
                .after(ServerSet::SendEvents)
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}