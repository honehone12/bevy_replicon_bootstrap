use std::marker::PhantomData;
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
pub struct Culling<T: DistanceCalculatable>(PhantomData<T>);

#[derive(Component)]
pub struct PlayerView;

#[derive(Component)]
pub enum CullingModifier {
    /// disable culling
    Always,
    /// modify distance
    Modify {
        /// added to distance
        /// -100.0 means disble culling within 100 unit
        addition: f32,
        /// multiplied to distance
        /// 0.5 makes distance 2x shorter
        multiplier: f32
    }
}

impl Default for CullingModifier {
    fn default() -> Self {
        Self::Modify {
            addition: 0.0, 
            multiplier: 1.0 
        }
    }
}

pub trait DistanceCalculatable: Component {
    fn distance(&self, rhs: &Self) -> f32;
}

#[derive(Default, Clone, Copy)]
pub struct DistanceAt {
    pub tick: u32,
    pub distance: f32
}

#[derive(Resource, Default)]
pub struct DistanceMap(HashMap<(Entity, Entity), DistanceAt>);

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

#[derive(Resource)]
pub struct CullingConfig {
    pub culling_threshold: f32,
    pub clean_up_on_disconnect: bool
}

#[derive(SystemSet, Clone, Eq, PartialEq, Hash, Debug)]
pub struct CullingSet;

fn calculate_distance_system<C>(
    query: Query<
        (Entity, &C), 
    (
        Or<(Changed<C>, Added<C>)>, 
        With<Culling<NetworkTranslation2D>>
    )>,
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

fn culling_system(
    query: Query<(Entity, &CullingModifier)>,
    player_views: Query<(Entity, &NetworkEntity), With<PlayerView>>,
    distance_map: Res<DistanceMap>,
    culling_config: Res<CullingConfig>,
    mut connected_clients: ResMut<ConnectedClients>
) {
    if distance_map.is_changed() {
        for (player_e, player_net_e) in player_views.iter() {
            let client_id = player_net_e.client_id();
            let visibility = match connected_clients.get_client_mut(client_id) {
                Some(c) => c.visibility_mut(),
                None => {
                    error!("client is not mapped in connected_clients, disconnected?");
                    continue;
                }
            };
            
            for (e, modifier) in query.iter() {
                if player_e == e {
                    continue;
                }

                let (multiplier, addition) = match modifier {
                    &CullingModifier::Always => {
                        if !visibility.is_visible(e) {
                            visibility.set_visibility(e, true);
                        }
                        continue;    
                    }
                    &CullingModifier::Modify { multiplier, addition } 
                    => (multiplier, addition)
                };

                let distance = match distance_map.get(player_e, e) {
                    Some(d) => d.distance,
                    None => 0.0
                };

                debug!(
                    "checking {player_e:?}:{e:?} distance: {} multiplier: {} addition: {}", 
                    distance,
                    multiplier,
                    addition
                );

                let result = distance * multiplier + addition;
                if result >= culling_config.culling_threshold {
                    if visibility.is_visible(e) {
                        visibility.set_visibility(e, false);
                    }
                } else {
                    if !visibility.is_visible(e) {
                        visibility.set_visibility(e, true);
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

pub trait ReplicationCullingAppExt {
    fn use_replication_culling<C>(
        &mut self,
        culling_config: CullingConfig
    ) -> &mut Self
    where C: Component + DistanceCalculatable;
}

impl ReplicationCullingAppExt for App {
    fn use_replication_culling<C>(
        &mut self,
        culling_config: CullingConfig
    ) -> &mut Self
    where C: Component + DistanceCalculatable {
        if self.world.contains_resource::<RepliconServer>() {
            let clean_up = culling_config.clean_up_on_disconnect;

            self.configure_sets(PostUpdate, 
                CullingSet
                .before(ServerSet::Send)
            )
            .insert_resource(DistanceMap::default())
            .insert_resource(culling_config)
            .add_systems(PostUpdate, (
                calculate_distance_system::<C>,
                culling_system
            ).chain().in_set(CullingSet));

            if clean_up {
                self.add_systems(PreUpdate, 
                    handle_player_entity_event
                    .after(PlayerEntityEventSet)
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
