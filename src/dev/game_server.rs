use bevy::utils::Uuid;
use bevy_replicon::server::server_tick::ServerTick;
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::{ClientId as RenetClientId, RenetServer};
use level::*;
use super::*;

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(
            PlayerStartLines::new()
            .with_group(vec![
                PLAYER_START_0,
                PLAYER_START_1,
                PLAYER_START_2,
                PLAYER_START_3
            ])
        )
        .add_plugins(GameCommonPlugin)
        .add_plugins((
            DefaultPlayerEntityEventPlugin,
            DistanceCullingPlugin{
                culling_threshold: DISTANCE_CULLING_THREASHOLD, 
                auto_clean: true
            },
            RelevantGroupPlugin::<PlayerGroup>::new()
        ))
        .add_systems(Startup, 
            server_setup_floor
        )
        .add_systems(Update, (
            handle_transport_error,
            handle_server_event,
            handle_player_entity_event,
            handle_fire
        ).chain());
    }
}

fn handle_server_event(
    mut events: EventReader<ServerEvent>,
    netcode_server: Res<NetcodeServerTransport>,
    mut renet_server: ResMut<RenetServer>
) {
    for e in events.read() {
        match e {
            ServerEvent::ClientConnected { client_id } => {
                let renet_client_id = RenetClientId::from_raw(client_id.get());
                
                let user_data = match netcode_server.user_data(renet_client_id) {
                    Some(u) => u,
                    None => {
                        warn!("no user data for client: {:?}", client_id);
                        renet_server.disconnect(renet_client_id);
                        return;
                    }
                };

                let uuid = match Uuid::from_slice(&user_data[0..16]) {
                    Ok(u) => u,
                    Err(e) => {
                        warn!("malformatted uuid for client: {:?}: {e}", client_id);
                        renet_server.disconnect(renet_client_id);
                        return;
                    }
                };

                info!("client: {client_id:?} uuid: {uuid} connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("client: {client_id:?} disconnected with reason: {reason}");
            }
        }
    }
}

fn handle_player_entity_event(
    mut commands: Commands,
    mut events: EventReader<PlayerEntityEvent>,
    mut start_lines: ResMut<PlayerStartLines>,
    server_tick: Res<ServerTick>,
) {
    for e in events.read() {
        if let PlayerEntityEvent::Spawned { client_id, entity } = e {
            let tick = server_tick.get();
            let group = PlayerGroup::random();
            let player_start = start_lines.next(0)
            .expect("missing player start lines initialization");
            info!("player: {client_id:?} spawned for group: {}", group.group);
        
            commands.entity(*entity)
            .insert((
                PlayerPresentation::random(),
                PlayerView,
                Culling::default(),
                group,
                TransformBundle::from_transform(
                    Transform::from_translation(player_start.translation)
                ),
                CharacterControllerBundle::new(
                    CHARACTER_HALF_HIGHT,
                    CHARACTER_RADIUS,
                    CHARACTER_OFFSET,
                    CHARACTER_MASS
                ),
                Jump::default(),
                NetworkTranslationBundle::<NetworkCharacterController>::new(
                    player_start.translation,
                    default(), 
                    tick, 
                    DEV_LARGE_CACHE_SIZE
                ).expect("sytem time looks earlier than unix epoch"),
                NetworkRotationBundle::<NetworkAngle>::new(
                    default(), 
                    RotationAxis::Z,
                    tick, 
                    DEV_LARGE_CACHE_SIZE
                ).expect("sytem time looks earlier than unix epoch"),
                EventSnapshots::<NetworkMovement2_5D>::with_capacity(
                    DEV_LARGE_CACHE_SIZE
                ),
                EventSnapshots::<NetworkFire>::with_capacity(
                    DEV_MEDIUM_CACHE_SIZE
                )
            ));
        }
    }
}

fn handle_fire(
    mut shooters: Query<(
        &NetworkEntity, 
        &mut EventSnapshots<NetworkFire>
    )>,
    query: Query<(
        &NetworkEntity, 
        &ComponentSnapshots<NetworkCharacterController>
    )>,
) {
    // *******************************
    // this code should be improved
    // *******************************
    
    for (shooter, mut fire_snaps) in shooters.iter_mut() {
        for fire in fire_snaps.frontier_ref() {
            info!(
                "player: {:?} fired at {}",
                shooter.client_id(), 
                fire.sent_timestamp() 
            );
    
            for (net_e, snaps) in query.iter() {
                let is_shooter = net_e.client_id() == shooter.client_id();
    
                let cache = snaps.cache_ref();
                let index = match cache.iter()
                .rposition(|s| 
                    s.timestamp() <= fire.sent_timestamp()
                ) {
                    Some(idx) => idx,
                    None => {
                        if cfg!(debug_assertions) {
                            panic!(
                                "could not find timestamp smaller than {}",
                                fire.sent_timestamp()
                            );
                        } else {
                            warn!(
                                "could not find timestamp smaller than {}, skipping",
                                fire.sent_timestamp()
                            );
                            continue;
                        }
                    }
                };
    
                // get by found index
                let snap = cache.get(index).unwrap();
                info!(
                    "found latest snap: shooter: {}, index: {}, total: {}, timestamp: {}, translation: {}",
                    is_shooter, 
                    index,
                    cache.len(), 
                    snap.timestamp(), 
                    snap.component().0
                );
            }
        }

        fire_snaps.cache();
    }
}
