use std::marker::PhantomData;
use bevy::prelude::*;
use bevy_replicon::{
    prelude::*,
    server::server_tick::ServerTick
};
use super::ee_map::*;
use crate::core::*;

pub trait RelevantGroup: Component + Default {
    fn is_relevant(&self, rhs: &Self) -> bool;
}

#[derive(Default)]
pub struct Relevancy<G: RelevantGroup> {
    pub is_relevant: bool,
    pub id_pair: (u64, u64),
    pub tick: u32,
    phantom: PhantomData<G>
}

impl<G: RelevantGroup> Relevancy<G> {
    #[inline]
    pub fn new(
        client_id_pair: (ClientId, ClientId), 
        tick: u32, 
        is_relevant: bool
    ) -> Self {
        Self { 
            is_relevant, 
            id_pair: (client_id_pair.0.get(), client_id_pair.1.get()), 
            tick, 
            phantom: PhantomData::<G> 
        }
    }

    #[inline]
    pub fn cient_id_pair(&self) -> (ClientId, ClientId) {
        (ClientId::new(self.id_pair.0), ClientId::new(self.id_pair.1))
    }
}

pub type RelevancyMap<G> = EntityPairMap<Relevancy<G>>;

fn relevancy_mapping_system<G: RelevantGroup>(
    changed: Query<(Entity, &NetworkEntity, &G), Changed<G>>,
    query: Query<(Entity, &NetworkEntity, &G)>,
    mut relevancy_map: ResMut<RelevancyMap<G>>,
    server_tick: Res<ServerTick>
) {
    for (changed_e, changed_net_e, changed_group) in changed.iter() {
        let tick = server_tick.get();
        for (e, net_e, group) in query.iter() {
            if changed_e == e {
                continue;
            }

            if let Some(r) = relevancy_map.get(changed_e, e) {
                if r.tick == tick {
                    continue;
                }
            }

            let is_relevant = changed_group.is_relevant(&group);
            let relevancy = Relevancy::<G>::new(
                (changed_net_e.client_id(), net_e.client_id()), 
                tick, 
                is_relevant
            );

            relevancy_map.insert(changed_e, e, relevancy);
            debug!(
                "updated relevency: {:?}:{:?} = {} tick: {}",
                changed_e, e,
                is_relevant,
                tick
            );
        }
    }    
}

fn handle_player_entity_event<G: RelevantGroup>(
    mut events: EventReader<PlayerEntityEvent>,
    mut relevancy_map: ResMut<RelevancyMap<G>>
) {
    for e in events.read() {
        if let &PlayerEntityEvent::Despawned { client_id: _, entity } = e {
            relevancy_map.remove(entity);
        }
    }
}

fn relevancy_culling_system<G: RelevantGroup>(
    player_views: Query<
        (Entity, &NetworkEntity), 
        (With<PlayerView>, With<G>)
    >,
    query: Query<Entity, With<G>>,
    mut connected_clients: ResMut<ConnectedClients>,
    relevancy_map: Res<RelevancyMap<G>>
) {
    for (player_e, player_net_e) in player_views.iter() {
        let client_id = player_net_e.client_id();
        let visibility = match connected_clients.get_client_mut(client_id) {
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

            match relevancy_map.get(player_e, e) {
                Some(r) => {
                    if r.is_relevant {
                        continue;
                    }
                }
                None => {
                    warn!(
                        "{:?}:{:?} pair is not mapped in relevancy map",
                        player_e, e
                    );
                }
            };
            
            if visibility.is_visible(e) {
                visibility.set_visibility(e, false);
            }
        }
    }
}

pub struct RelevantGroupPlugin<G: RelevantGroup>(PhantomData<G>);

impl<G: RelevantGroup> RelevantGroupPlugin<G> {
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData::<G>)
    }
} 

impl<G: RelevantGroup> Plugin for RelevantGroupPlugin<G> {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.insert_resource(RelevancyMap::<G>::default())
            .add_systems(PreUpdate, 
                handle_player_entity_event::<G>
                .after(ServerBootSet::PlayerEntityEvent))
            .add_systems(PostUpdate, (
                relevancy_mapping_system::<G>,
                relevancy_culling_system::<G>
            ).chain(
            ).in_set(ServerBootSet::Grouping));
        } else {
            panic!("could not find replicon server");
        }
    }
}
