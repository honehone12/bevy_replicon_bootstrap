use anyhow::anyhow;
use bevy::utils::Uuid;
use bevy_replicon::server::server_tick::ServerTick;
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::ClientId as RenetClientId;
use bevy_rapier3d::prelude::*;
use super::*;

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameCommonPlugin)
        .add_plugins(DistanceCullingPlugin{
            culling_threshold: DISTANCE_CULLING_THREASHOLD, 
            auto_clean: true
        })
        .add_plugins(RelevantGroupPlugin(PhantomData::<PlayerGroup>))
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
    server_tick: Res<ServerTick>,
) {
    for e in events.read() {
        if let PlayerEntityEvent::Spawned { client_id, entity } = e {
            let tick = server_tick.get();
            
            let net_trans_bundle = match NetworkTranslationBundle
            ::<NetworkCharacterController>::new(
                CHARACTER_SPAWN_POSITION,
                default(), 
                tick, 
                DEV_MAX_UPDATE_SNAPSHOT_SIZE
            ) {
                Ok(b) => b,
                Err(e) => {
                    error(e.into());
                    return;
                }
            };
            
            let net_rot_bundle = match NetworkRotationBundle
            ::<NetworkAngle>::new(
                default(), 
                RotationAxis::Z,
                tick, 
                DEV_MAX_UPDATE_SNAPSHOT_SIZE
            ) {
                Ok(b) => b,
                Err(e) => {
                    error(e.into());
                    return;
                }
            };

            let group = PlayerGroup::random();
            info!("player: {client_id:?} spawned for group: {}", group.group);
        
            commands.entity(*entity)
            .insert((
                PlayerPresentation::random(),
                PlayerView,
                Culling::default(),
                group,
                TransformBundle::from_transform(
                    Transform::from_translation(CHARACTER_SPAWN_POSITION)
                ),
                CharacterControllerBundle::default(),
                Collider::capsule_y(CHARACTER_HALF_HIGHT, CHARACTER_RADIUS),
                net_trans_bundle,
                net_rot_bundle,
                EventSnapshots::<NetworkMovement2_5D>
                ::with_capacity(DEV_MAX_UPDATE_SNAPSHOT_SIZE),
                EventSnapshots::<NetworkFire>
                ::with_capacity(DEV_MAX_SNAPSHOT_SIZE)
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
    for (shooter, mut fire_snaps) in shooters.iter_mut() {
        for fire in fire_snaps.frontier_ref() {
            info!(
                "player: {:?} fired at {}",
                shooter.client_id(), 
                fire.timestamp() 
            );
    
            for (net_e, snaps) in query.iter() {
                let is_shooter = net_e.client_id() == shooter.client_id();
    
                let cache = snaps.cache_ref();
                let index = match cache.iter()
                .rposition(|s| 
                    s.timestamp() <= fire.timestamp()
                ) {
                    Some(idx) => idx,
                    None => {
                        if cfg!(debug_assertions) {
                            panic!(
                                "could not find timestamp smaller than {}",
                                fire.timestamp()
                            );
                        } else {
                            warn!(
                                "could not find timestamp smaller than {}, skipping",
                                fire.timestamp()
                            );
                            continue;
                        }
                    }
                };
    
                // get by found index
                let snap = cache.get(index).unwrap();
                info!(
                    "found latest snap: shooter: {}, index: {}, timestamp: {}, translation: {}",
                    is_shooter, 
                    index, 
                    snap.timestamp(), 
                    snap.component().0
                );
            }
        }

        fire_snaps.cache();
    }
}
