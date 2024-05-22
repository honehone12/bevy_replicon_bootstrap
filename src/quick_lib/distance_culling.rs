use bevy::{
    ecs::entity::EntityHashMap,
    prelude::*
};
use bevy_replicon::{
    prelude::*, 
    server::server_tick::ServerTick
};
use crate::prelude::*;

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
pub struct DistanceMap(EntityHashMap<EntityHashMap<DistanceAt>>);

#[derive(Resource)]
pub struct DistanceCullingConfig {
    pub culling_threshold: f32
}

impl DistanceMap {
    pub fn insert(
        &mut self, 
        l: Entity, r: Entity, 
        distance_at: DistanceAt
    ) -> Option<DistanceAt> {
        if let Some(l_map) = self.0.get_mut(&l) {
            return l_map.insert(r, distance_at)
        }

        match self.0.get_mut(&r) {
            Some(r_map) => r_map.insert(l, distance_at),
            None => {
                self.0
                .entry(l)
                .or_insert(default())
                .insert(r, distance_at)
            }
        }
    }

    pub fn get(
        &self,
        l: &Entity, r: &Entity
    ) -> Option<&DistanceAt> {
        if let Some(l_map) = self.0.get(l) {
            if let Some(d) = l_map.get(r) {
                return Some(d)
            }
        }

        match self.0.get(r) {
            Some(r_map) => r_map.get(l),
            None => None
        }
    }

    pub fn remove() {
        todo!();
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
    for (e, c) in query.iter() {
        let tick = server_tick.get();

        for (player_e, player_c) in player_views.iter() {
            if e == player_e {
                continue;
            }

            if let Some(d) = distance_map.get(&player_e, &e) {
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
            info!(
                "updated distance from: {:?} to: {:?} tick: {} distance: {}",
                player_e, e,
                tick, 
                distance
            );
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

                let distance_at = match distance_map.get(&player_e, &e) {
                    Some(d) => d,
                    None => {
                        warn!("distance {player_e:?}:{e:?} not found");
                        continue;
                    }
                };

                if distance_at.distance >= culling_config.culling_threshold {
                    if client_visibility.is_visible(e) {
                        info!("{player_e:?}:{e:?} is not visible now");
                        client_visibility.set_visibility(e, false);
                    }
                } else {
                    if !client_visibility.is_visible(e) {
                        info!("{player_e:?}:{e:?} is visible now");
                        client_visibility.set_visibility(e, true);
                    }
                }
            }
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
            self.insert_resource(DistanceMap::default())
            .insert_resource(culling_config)
            .add_systems(PostUpdate, (
                calculate_distance_system::<C>,
                distance_culling_system
            ).chain().before(ServerSet::Send))
        } else if self.world.contains_resource::<RepliconClient>() {
            self
        } else {
            panic!("could not find replicon server nor client");
        }        
    }
}
