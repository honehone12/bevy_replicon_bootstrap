use std::marker::PhantomData;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::prelude::*;

pub trait RelevantGroup: Component {
    fn is_relevant(&self, rhs: &Self) -> bool;
}

#[derive(Component, Default)]
pub struct Relevant<G: RelevantGroup>(PhantomData<G>);

fn relevancy_system<G: RelevantGroup>(
    player_views: Query<
        (Entity, &NetworkEntity, &G), 
        (With<Relevant<G>>, With<PlayerView>)
    >,
    query: Query<(Entity, &G), With<Relevant<G>>>,
    mut connected_clients: ResMut<ConnectedClients>
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

pub trait RelevancyAppExt {
    fn use_relevancy<G: RelevantGroup>(&mut self) -> &mut Self;
}

impl RelevancyAppExt for App {
    fn use_relevancy<G: RelevantGroup>(&mut self) -> &mut Self {
        if self.world.contains_resource::<RepliconServer>() {
            self.add_systems(PostUpdate, 
                relevancy_system::<G>
                .after(CullingSet)
                .before(ServerSet::Send)
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}