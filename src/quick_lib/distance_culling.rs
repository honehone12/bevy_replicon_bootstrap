use bevy::{
    prelude::*, 
    utils::HashMap
};
use bevy_replicon::{
    prelude::*, 
    server::server_tick::ServerTick
};
use crate::prelude::*;

#[derive(Component, Default)]
pub struct PlayerView;

#[derive(Default)]
pub struct Distance;

pub trait DistanceCalculatable {
    fn distance(&self, rhs: &Self) -> f32;
}

#[derive(Default, Clone, Copy)]
pub struct DistanceAt {
    pub tick: u32,
    pub distance: f32
}

#[derive(Resource, Default)]
pub struct DistanceMap(HashMap<(Entity, Entity), DistanceAt>);

#[derive(Resource)]
pub struct DistanceCullingConfig {
    pub culling_threshold: f32,
    pub clean_up_on_disconnect: bool
}

impl DistanceMap {
    #[inline]
    pub fn insert(
        &mut self,
        key_l: Entity, key_r: Entity,
        distance_at: DistanceAt
    ) -> Option<DistanceAt> {
        let key = if key_l >= key_r {
            (key_l, key_r)
        } else {
            (key_r, key_l)
        };

        self.0.insert(key, distance_at)
    }

    #[inline]
    pub fn get(
        &self,
        key_l: Entity, key_r: Entity
    ) -> Option<&DistanceAt> {
        let key = if key_l >= key_r {
            (key_l, key_r)
        } else {
            (key_r, key_l)
        };

        return self.0.get(&key)
    }

    #[inline]
    pub fn remove(&mut self, key: Entity) {
        self.0.retain(|k, _| k.0 != key && k.1 != key);
    }
}

fn calculate_distance_system<C>(
    query: Query<
        (Entity, &C), 
        (Or<(Changed<C>, Added<C>)>, With<Importance<Distance>>)
    >,
    player_views: Query<
        (Entity, &C), 
        With<PlayerView>
    >,
    mut distance_map: ResMut<DistanceMap>,
    server_tick: Res<ServerTick>
)
where C: Component + DistanceCalculatable {
    if !query.is_empty() {
        let tick = server_tick.get();
        for (player_e, player_c) in player_views.iter() {    
            for (e, c) in query.iter() {
                if e == player_e {
                    continue;
                }
    
                if let Some(d) = distance_map.get(player_e, e) {
                    if d.tick == tick {
                        continue;
                    }
                }
    
                let distance = player_c.distance(&c);
                let distance_at = DistanceAt{
                    tick,
                    distance
                };
                
                distance_map.insert(player_e, e, distance_at);
                debug!(
                    "updated distance from: {:?} to: {:?} tick: {} distance: {}",
                    player_e, e,
                    tick, 
                    distance
                );
            }        
        }
    }
}

fn distance_culling_system(
    query: Query<Entity, With<Importance<Distance>>>,
    player_views: Query<(Entity, &NetworkEntity), With<PlayerView>>,
    distance_map: Res<DistanceMap>,
    culling_config: Res<DistanceCullingConfig>,
    mut connected_clients: ResMut<ConnectedClients>
) {
    if distance_map.is_changed() {
        for (player_e, player_net_e) in player_views.iter() {
            let client_id = player_net_e.client_id();
            let client_visibility = match connected_clients.get_client_mut(client_id) {
                Some(c) => c.visibility_mut(),
                None => {
                    error!("client is not mapped in connected_clients, disconnected?");
                    continue;
                }
            };
            
            for e in query.iter() {
                if player_e == e {
                    continue;
                }

                let distance_at = match distance_map.get(player_e, e) {
                    Some(d) => d,
                    None => {
                        warn!("distance {player_e:?}:{e:?} not found");
                        continue;
                    }
                };

                debug!("checking {player_e:?}:{e:?} distance: {}", distance_at.distance);

                if distance_at.distance >= culling_config.culling_threshold {
                    if client_visibility.is_visible(e) {
                        client_visibility.set_visibility(e, false);
                    }
                } else {
                    if !client_visibility.is_visible(e) {
                        client_visibility.set_visibility(e, true);
                    }
                }
            }
        }
    }
}

fn handle_player_entity_event(
    mut events: EventReader<PlayerEntityEvent>,
    mut distance_map: ResMut<DistanceMap>
) {
    for e in events.read() {
        if let &PlayerEntityEvent::Despawned { client_id: _, entity } = e {
            distance_map.remove(entity);
        }
    }
}

pub trait DistanceCullingAppExt {
    fn use_distance_culling<C>(
        &mut self,
        culling_config: DistanceCullingConfig
    ) -> &mut Self
    where C: Component + DistanceCalculatable;
}

impl DistanceCullingAppExt for App {
    fn use_distance_culling<C>(
        &mut self,
        culling_config: DistanceCullingConfig
    ) -> &mut Self
    where C: Component + DistanceCalculatable {
        if self.world.contains_resource::<RepliconServer>() {
            let clean_up = culling_config.clean_up_on_disconnect;

            self.insert_resource(DistanceMap::default())
            .insert_resource(culling_config)
            .add_systems(PostUpdate, (
                calculate_distance_system::<C>,
                distance_culling_system
            ).chain().before(ServerSet::Send));

            if clean_up {
                self.add_systems(PreUpdate, 
                    handle_player_entity_event
                    .after(ServerSet::Receive)
                );
            }

            self
        } else if self.world.contains_resource::<RepliconClient>() {
            self
        } else {
            panic!("could not find replicon server nor client");
        }        
    }
}
