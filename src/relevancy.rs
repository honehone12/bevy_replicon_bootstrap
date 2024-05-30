use std::marker::PhantomData;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::prelude::*;

pub trait RelevantGroup: Component {
    fn is_relevant(&self, rhs: &Self) -> bool;
}

#[derive(Component, Default)]
pub struct Relevant<G>(PhantomData<G>)
where G: RelevantGroup + Default;

fn relevancy_system<G>(
    player_views: Query<
        (Entity, &NetworkEntity, &G), 
        (With<Relevant<G>>, With<PlayerView>)
    >,
    query: Query<(Entity, &G), With<Relevant<G>>>,
    mut connected_clients: ResMut<ConnectedClients>
)
where G: RelevantGroup + Default {
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

pub struct RelevancyPlugin<G>(pub PhantomData<G>)
where G: RelevantGroup + Default;

impl<G> Plugin for RelevancyPlugin<G>
where G: RelevantGroup + Default {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.add_systems(PostUpdate, 
                relevancy_system::<G>
                .after(CullingSet)
                .before(ServerSet::Send)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
