use bevy::{
    prelude::*, 
    utils::Uuid
};
use bevy_replicon::{
    prelude::*, 
    server::server_tick::ServerTick
};
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::ClientId as RenetClientId;
use anyhow::anyhow;
use crate::{
    dev::{
        config::*,
        *
    },
    prelude::*
};

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameCommonPlugin)
        .insert_resource(PlayerEntityMap::default())
        .use_client_event_snapshot::<NetworkMovement2D>(ChannelKind::Unreliable)
        .add_systems(Update, (
            handle_transport_error,
            handle_server_event,
            handle_fire
        ).chain());
    }
}

fn handle_server_event(
    mut commands: Commands,
    mut events: EventReader<ServerEvent>,
    mut entity_map: ResMut<PlayerEntityMap>,
    netcode_server: Res<NetcodeServerTransport>,
    server_tick: Res<ServerTick>,
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

                let tick = server_tick.get();
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

                let movement_snaps = EventSnapshots::<NetworkMovement2D>
                ::with_capacity(DEV_MAX_SNAPSHOT_SIZE);

                let entity = commands.spawn((
                    NetworkEntity::new(client_id),
                    Replicated,
                    PlayerPresentation::random(),
                    PlayerView,
                    Importance::<Distance>::default(),
                    translation_bundle,
                    yaw_bundle,
                    movement_snaps
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

fn handle_fire(
    query: Query<(
        &NetworkEntity,
        &ComponentSnapshots<NetworkTranslation2D>
    )>,
    mut events: EventReader<FromClient<NetworkFire>>
) {
    for FromClient { client_id, event } in events.read() {
        info!(
            "player: {:?} fired at {}",
            client_id, event.timestamp() 
        );

        for (net_e, snaps) in query.iter() {
            let is_shooter = net_e.client_id() == *client_id;

            let index = match snaps.iter().rposition(
                |s| s.timestamp() <= event.timestamp()
            ) {
                Some(idx) => idx,
                None => {
                    if cfg!(debug_assertions) {
                        panic!(
                            "could not find timestamp smaller than {}, insert one at initialization",
                            event.timestamp()
                        );
                    } else {
                        warn!(
                            "could not find timestamp smaller than {}, skipping",
                            event.timestamp()
                        );
                        continue;
                    }
                }
            };

            // get by found index
            let snap = snaps.get(index).unwrap();
            info!(
                "found latest snap: shooter: {}, index: {}, timestamp: {}, translation: {}",
                is_shooter, index, snap.timestamp(), snap.component().0
            );
        }
    }
}
