use bevy::{prelude::*, utils::Uuid};
use bevy_replicon::{core::replicon_tick::RepliconTick, prelude::*};
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::ClientId as RenetClientId;
use anyhow::anyhow;
use crate::{
    prelude::*,
    dev::{*, config::DEV_MAX_SNAPSHOT_SIZE}
};

use self::event::{NetworkFire, NetworkMovement2D};

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerEntityMap::default())
        .use_client_event_snapshot::<NetworkMovement2D>(ChannelKind::Unreliable)
        .use_component_snapshot::<NetworkTranslation2D>()
        .use_component_snapshot::<NetworkYaw>()
        .add_client_event::<NetworkFire>(ChannelKind::Ordered)
        .replicate::<NetworkEntity>()
        .replicate::<NetworkTranslation2D>()
        .replicate::<NetworkYaw>()
        .replicate::<PlayerPresentation>()
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
                let translation_bundle = match NetworkTranslation2DWithSnapshots::new(
                    default(), 
                    tick, 
                    DEV_MAX_SNAPSHOT_SIZE
                ) {
                    Ok(b) => b,
                    Err(e) => {
                        error(e.into());
                        return;
                    }
                };
                let yaw_bundle = match NetworkYawWithSnapshots::new(
                    default(), 
                    tick, 
                    DEV_MAX_SNAPSHOT_SIZE
                ) {
                    Ok(b) => b,
                    Err(e) => {
                        error(e.into());
                        return;
                    }
                };

                let entity = commands.spawn((
                    NetworkEntity::new(client_id),
                    Replication,
                    PlayerPresentation::random(),
                    translation_bundle,
                    yaw_bundle
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
