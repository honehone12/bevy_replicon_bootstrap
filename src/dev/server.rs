use bevy::{prelude::*, utils::Uuid};
use bevy_replicon::{core::replicon_tick::RepliconTick, prelude::*};
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::ClientId as RenetClientId;
use anyhow::anyhow;
use crate::{
    prelude::*,
    dev::{*, config::DEV_MAX_BUFFER_SIZE}
};

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerEntityMap::default())
        .add_systems(Update, (
            handle_transport_error,
            handle_server_event
        ).chain());
    }
}

fn handle_server_event(
    mut commands: Commands,
    mut events: EventReader<ServerEvent>,
    mut entity_map: ResMut<PlayerEntityMap>,
    netcode_server: Res<NetcodeServerTransport>,
    replicon_tick: Res<RepliconTick> 
) {
    for e in events.read() {
        match e {
            ServerEvent::ClientConnected { client_id } => {
                let user_data = match netcode_server.user_data(
                    RenetClientId::from_raw(client_id.get())
                ) {
                    Some(u) => u,
                    None => {
                        error(anyhow!("no user data for client: {}", client_id.get()));
                        return;
                    }
                };

                let uuid = match Uuid::from_slice(&user_data[0..16]) {
                    Ok(u) => u,
                    Err(e) => {
                        error(e.into());
                        return;
                    }
                };

                let tick = replicon_tick.get();
                let entity = commands.spawn((
                    NetworkEntity::new(client_id),
                    PlayerPresentation::random(),
                    NetworkTranslation2DWithSnapshots::new(
                        default(), 
                        tick, 
                        DEV_MAX_BUFFER_SIZE
                    ).expect("check system time of the computer")
                ))
                .id();

                match entity_map.try_insert(*client_id, entity) {
                    Ok(()) => (),
                    Err(e) => {
                        error(e.into());
                        return;
                    }
                }                
                info!("client: {client_id:?} uuid: {uuid} connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                match entity_map.get(client_id) {
                    Some(e) => {
                        commands.entity(*e).despawn();
                        entity_map.remove(client_id);
                    }
                    None => ()
                }
                info!("client: {client_id:?} disconnected with reason: {reason}");
            }
        }
    }
}
