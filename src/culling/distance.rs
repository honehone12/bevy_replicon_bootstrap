use std::marker::PhantomData;
use bevy::prelude::*;
use bevy_replicon::{
    prelude::*, 
    server::server_tick::ServerTick
};
use super::{ee_map::*, CullingSet};
use crate::core::*;

#[derive(Component, Default)]
pub struct Culling<C>
where C: DistanceCalculatable + Default {
    pub modifier: CullingModifier,
    phantom: PhantomData<C>
}

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

#[derive(Default, Clone, Copy)]
pub struct DistanceAt {
    pub tick: u32,
    pub distance: f32
}

pub type DistanceMap = EntityPairMap<DistanceAt>;

#[derive(Resource)]
pub struct CullingConfig {
    pub culling_threshold: f32,
    pub clean_up_on_disconnect: bool,
}

fn calculate_distance_system<C>(
    query: Query<(Entity, &C), (Changed<C>, With<Culling<C>>)>,
    player_views: Query<(Entity, &C), With<PlayerView>>,
    mut distance_map: ResMut<DistanceMap>,
    server_tick: Res<ServerTick>
)
where C: DistanceCalculatable + Default {
    for (player_e, player_c) in player_views.iter() {    
        let tick = server_tick.get();
        for (e, c) in query.iter() {
            if player_e == e {
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

fn culling_system<C>(
    query: Query<(Entity, &Culling<C>)>,
    player_views: Query<(Entity, &NetworkEntity), With<PlayerView>>,
    distance_map: Res<DistanceMap>,
    culling_config: Res<CullingConfig>,
    mut connected_clients: ResMut<ConnectedClients>
)
where C: DistanceCalculatable + Default {
    if !distance_map.is_changed() {
        return;
    }

    for (player_e, player_net_e) in player_views.iter() {
        let client_id = player_net_e.client_id();
        let visibility = match connected_clients.get_client_mut(client_id) {
            Some(c) => c.visibility_mut(),
            None => {
                error!("client is not mapped in connected_clients, disconnected?");
                continue;
            }
        };
        
        for (e, culling) in query.iter() {
            if player_e == e {
                continue;
            }

            let (multiplier, addition) = match culling.modifier {
                CullingModifier::Always => {
                    if !visibility.is_visible(e) {
                        visibility.set_visibility(e, true);
                    }
                    continue;    
                }
                CullingModifier::Modify { multiplier, addition } 
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

#[derive(Default)]
pub struct ReplicationCullingPlugin<C>
where C: Component + DistanceCalculatable {
    pub culling_threshold: f32,
    pub auto_clean: bool,
    pub phantom: PhantomData<C>
}

impl<C> Plugin for ReplicationCullingPlugin<C>
where C: DistanceCalculatable + Default {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.configure_sets(PostUpdate, 
                CullingSet
                .before(ServerSet::Send)
            )
            .insert_resource(DistanceMap::default())
            .insert_resource(CullingConfig{
                culling_threshold: self.culling_threshold,
                clean_up_on_disconnect: self.auto_clean
            })
            .add_systems(PostUpdate, (
                calculate_distance_system::<C>,
                culling_system::<C>
            ).chain(
            ).in_set(CullingSet));

            if self.auto_clean {
                app.add_systems(PreUpdate, 
                    handle_player_entity_event
                    .after(PlayerEntityEventSet)
                );
            }
        } else {
            panic!("could not find replicon server");
        }     
    }
}
