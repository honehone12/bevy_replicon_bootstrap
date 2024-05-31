use std::hash::Hash;
use std::marker::PhantomData;
use bevy::{prelude::*, utils::{hashbrown::hash_map::Keys, HashMap}};
use bevy_replicon::prelude::*;
use crate::prelude::*;

pub trait RelevantGroup
: Component + Eq + PartialEq + Clone + Copy + Hash + Default {
    fn is_relevant(&self, rhs: &Self) -> bool;
}

#[derive(Resource, Default)]
pub struct RelevancyMap<G: RelevantGroup> {
    entity_map: HashMap<G, Vec<(Entity, NetworkEntity)>>,
    group_map: HashMap<(Entity, NetworkEntity), G>
}

impl<G: RelevantGroup> RelevancyMap<G> {
    #[inline]
    pub fn insert(&mut self, 
        entity: Entity, 
        net_entity: NetworkEntity,
        group: G 
    ) {
        if let Some(old) = self.group_map
        .insert((entity, net_entity), group) {
            // get by returned old group
            let v = self.entity_map.get_mut(&old)
            .unwrap();
            let idx = v.iter()
            // get by key of Some
            .position(|&(e, net_e)| e == entity && net_e == net_entity)
            .unwrap();
            v.swap_remove(idx);
        } 

        let v = self.entity_map.entry(group)
        .or_insert(default());
        v.push((entity, net_entity));
    }

    #[inline]
    pub fn get_entities(&mut self, group: &G) 
    -> Option<&Vec<(Entity, NetworkEntity)>> {
        self.entity_map.get(group)
    }

    #[inline]
    pub fn get_group(&mut self, entity: Entity, net_entity: NetworkEntity)
    -> Option<&G> {
        self.group_map.get(&(entity, net_entity))
    }

    #[inline]
    pub fn iter_group(&self) -> Keys<'_, G, Vec<(Entity, NetworkEntity)>> {
        self.entity_map.keys()
    }

    #[inline]
    pub fn remove(&mut self, entity: Entity, net_entity: NetworkEntity) {
        if let Some(g) = self.group_map.remove(&(entity, net_entity)) {
            // get by returned group
            let v = self.entity_map.get_mut(&g)
            .unwrap();
            let idx = v.iter()
            // get by key of Some
            .position(|&(e, net_e)| e == entity && net_e == net_entity)
            .unwrap();
            v.swap_remove(idx);
        }
    }
}

fn relevancy_system<G: RelevantGroup>(
    query: Query<(Entity, &NetworkEntity, &G), Changed<G>>,
    mut relevancy_map: ResMut<RelevancyMap<G>>
) {
    for (e, net_e, group) in query.iter() {
        relevancy_map.remove(e, *net_e);
        relevancy_map.insert(e, *net_e, *group);
    }
}

fn handle_player_entity_event<G: RelevantGroup>(
    mut events: EventReader<PlayerEntityEvent>,
    mut relevancy_map: ResMut<RelevancyMap<G>>
) {
    for e in events.read() {
        if let &PlayerEntityEvent::Despawned { client_id, entity } = e {
            relevancy_map.remove(entity, NetworkEntity::new(client_id));
        }
    }
}

fn relevancy_culling_system<G: RelevantGroup>(
    player_views: Query<
        (Entity, &NetworkEntity, &G), 
        With<PlayerView>
    >,
    query: Query<(Entity, &G)>,
    mut connected_clients: ResMut<ConnectedClients>,
    relevancy_map: Res<RelevancyMap<G>>
) {
    for (player_e, player_net_e, player_group) in player_views.iter() {
        let client_id = player_net_e.client_id();
        let visibility = match connected_clients.get_client_mut(client_id) {
            Some(c) => c.visibility_mut(),
            None => {
                error!("client is not mapped in connected_clients, disconnected?");
                continue;
            }
        };
        
        for (e, group) in query.iter() {
            if player_e == e {
                continue;
            }

            if !player_group.is_relevant(&group) {
                if visibility.is_visible(e) {
                    visibility.set_visibility(e, false);
                }
            }
        }
    }
}

pub struct RelevancyPlugin<G: RelevantGroup>(pub PhantomData<G>);

impl<G: RelevantGroup> Plugin for RelevancyPlugin<G> {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.insert_resource(RelevancyMap::<G>::default())
            .add_systems(PreUpdate, (
                relevancy_system::<G>,
                handle_player_entity_event::<G>
            ).chain().after(PlayerEntityEventSet))
            .add_systems(PostUpdate, 
                relevancy_culling_system::<G>
                .after(CullingSet)
                .before(ServerSet::Send)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}

pub trait RelevancyAppExt {
    fn use_relevant_event();
}