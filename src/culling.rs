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
pub struct Culling<C>(PhantomData<C>)
where C: DistanceCalculatable + Default;

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
    pub clean_up_on_disconnect: bool,
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
where C: DistanceCalculatable + Default {
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

                // !!
                // todo!("join relebancy settin here !!"); 


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

fn handle_player_entity_event<C>(
    mut commands: Commands,
    mut events: EventReader<PlayerEntityEvent>,
    mut distance_map: ResMut<DistanceMap>
)
where C: DistanceCalculatable + Default {
    for e in events.read() {
        match e {
            &PlayerEntityEvent::Spawned { client_id: _, entity } => {
                commands.entity(entity)
                .insert((
                    Culling::<C>::default(),
                    CullingModifier::default()
                ));
            }
            &PlayerEntityEvent::Despawned { client_id: _, entity } => {
                distance_map.remove(entity);
            }
        }
    }
}

#[derive(Default)]
pub struct ReplicationCullingPlugin<C>
where C: Component + DistanceCalculatable {
    pub culling_threshold: f32,
    pub clean_up_on_disconnect: bool,
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
                clean_up_on_disconnect: self.clean_up_on_disconnect
            })
            .add_systems(PostUpdate, (
                calculate_distance_system::<C>,
                culling_system
            ).chain().in_set(CullingSet));

            if self.clean_up_on_disconnect {
                app.add_systems(PreUpdate, 
                    handle_player_entity_event::<C>
                    .after(PlayerEntityEventSet)
                );
            }
        } else {
            panic!("could not find replicon server");
        }     
    }
}
