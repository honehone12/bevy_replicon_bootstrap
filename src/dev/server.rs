use bevy::{prelude::*, utils::Uuid};
use bevy_replicon::{core::replicon_tick::RepliconTick, prelude::*};
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::ClientId as RenetClientId;
use anyhow::anyhow;
use crate::{
    dev::{config::DEV_MAX_SNAPSHOT_SIZE, *}, 
    prelude::*, 
};

use self::event::{NetworkFire, NetworkMovement2D};

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameCommonPlugin)
        .insert_resource(PlayerEntityMap::default())
        .add_plugins(RepliconActionPlugin)
        .use_client_event_snapshot::<NetworkMovement2D>(ChannelKind::Unreliable)
        .use_replicated_component_snapshot::<NetworkTranslation2D>()
        .use_replicated_component_snapshot::<NetworkYaw>()
        .add_client_event::<NetworkFire>(ChannelKind::Ordered)
        .add_server_event::<ForceReplicate<NetworkTranslation2D>>(ChannelKind::Ordered)
        .add_systems(FixedUpdate, 
            move_2d_system
        )
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

                let movement_snaps = EventSnapshots::<NetworkMovement2D>
                ::with_capacity(DEV_MAX_SNAPSHOT_SIZE);

                let entity = commands.spawn((
                    NetworkEntity::new(client_id),
                    Replication,
                    PlayerPresentation::random(),
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

fn move_2d_system(
    mut query: Query<(
        &NetworkEntity,
        &mut NetworkTranslation2D,
        &ComponentSnapshots<NetworkTranslation2D>,
        &mut PredioctionError<NetworkTranslation2D>,
        &mut EventSnapshots<NetworkMovement2D>
    )>,
    fixed_time: Res<Time<Fixed>>,
    params: Res<PlayerMovementParams>,
    mut force_replication: EventWriter<ToClients<ForceReplicate<NetworkTranslation2D>>>
) {
    for (
        net_e,
        mut net_translation, snaps, mut prediction_error, 
        mut movements
    ) in query.iter_mut() {  
        movements.sort_with_index();
        let mut frontier = movements.frontier();
        if frontier.len() == 0 {
            continue;
        }

        let mut translation = net_translation.clone();
        // frontier is not empty
        let first = frontier.next().unwrap().event();

        let index = match snaps.iter().rposition(
            |s| s.timestamp() <= first.timestamp()
        ) {
            Some(idx) => idx,
            None => {
                panic!("could not find timestamp smaller than: {}", first.timestamp());
            }
        };
        // get by found index
        let server_translation = snaps.get(index).unwrap().component();
        let client_translation = first.current_translation;

        let error = server_translation.0.distance_squared(client_translation);
        info!("translation error: {error}");
        if error > params.translation_error_threashold {
            prediction_error.error_count += 1;
            
            warn!(
                "translation error is over threashold, now prediction error count: {}", 
                prediction_error.error_count
            );

            if prediction_error.error_count > params.prediction_error_count_threashold {
                warn!(
                    "prediction error count is over threashold"
                );

                force_replication.send(ToClients { 
                    mode: SendMode::Direct(net_e.client_id()), 
                    event: default()
                });

                prediction_error.error_count = 0;
            }
        } else {
            prediction_error.error_count = 0;
        }
        
        move_2d(&mut translation, first, &params, &fixed_time);

        while let Some(snap) = frontier.next() {
            move_2d(&mut translation, snap.event(), &params, &fixed_time)
        }

        net_translation.0 = translation.0;
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
                            "could not find timestamp smaller than {}, insert one on initialization",
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
