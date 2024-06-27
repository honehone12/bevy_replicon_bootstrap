use bevy::prelude::*;
use bevy_replicon::{
    prelude::*, 
    server::server_tick::ServerTick
};
use super::{ee_map::*, CullingSet};
use crate::core::*;

#[derive(Component)]
pub enum Culling {
    Default,
    /// disable culling
    Disable,
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

impl Default for Culling {
    fn default() -> Self {
        Self::Default
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

fn calculate_distance_system(
    query: Query<(Entity, &Transform), (Changed<Transform>, With<Culling>)>,
    player_views: Query<(Entity, &Transform), With<PlayerView>>,
    mut distance_map: ResMut<DistanceMap>,
    server_tick: Res<ServerTick>
) {
    for (player_e, player_t) in player_views.iter() {    
        let tick = server_tick.get();
        for (e, t) in query.iter() {
            if player_e == e {
                continue;
            }

            if let Some(d) = distance_map.get(player_e, e) {
                if d.tick == tick {
                    continue;
                }
            }

            let distance = player_t.translation.distance_squared(t.translation);
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

fn culling_system(
    query: Query<(Entity, &Culling)>,
    player_views: Query<(Entity, &NetworkEntity), With<PlayerView>>,
    distance_map: Res<DistanceMap>,
    culling_config: Res<CullingConfig>,
    mut connected_clients: ResMut<ConnectedClients>
) {
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

            let (addition, multiplier) = match culling {
                &Culling::Default => (0.0, 0.0),
                &Culling::Modify { addition, multiplier } => (addition, multiplier),
                &Culling::Disable => {
                    if !visibility.is_visible(e) {
                        visibility.set_visibility(e, true);
                    }
                    continue;    
                }
                
            };

            let distance = match distance_map.get(player_e, e) {
                Some(d) => d.distance,
                None => 0.0
            };

            debug!(
                "checking {player_e:?}:{e:?} distance: {} addition: {} multiplier: {}", 
                distance,
                addition,
                multiplier
            );

            let result = addition + distance * multiplier;
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
pub struct ReplicationCullingPlugin {
    pub culling_threshold: f32,
    pub auto_clean: bool
}

impl Plugin for ReplicationCullingPlugin {
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
                calculate_distance_system,
                culling_system
            ).chain(
            ).in_set(CullingSet));

            if self.auto_clean {
                app.add_systems(PreUpdate, 
                    handle_player_entity_event
                    .after(ServerBootSet::PlayerEntityEvent)
                );
            }
        } else {
            panic!("could not find replicon server");
        }     
    }
}
