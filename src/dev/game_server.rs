use uuid::Uuid;
use bevy_replicon::server::server_tick::ServerTick;
use bevy_replicon_renet::renet::transport::NetcodeServerTransport;
use bevy_replicon_renet::renet::{ClientId as RenetClientId, RenetServer};
use rapier3d::geometry::{Capsule, Ray, RayCast};
use level::*;
use rapier3d::math::Isometry;
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
                culling_threshold: DISTANCE_CULLING_THREASHOLD
            },
            RelevantGroupPlugin::<PlayerGroup>::new()
        ))
        .add_systems(Startup, ( 
            server_setup_floor,
            server_setup_walls,
            setup_ball
        ).chain())
        .add_systems(Update, (
            handle_transport_error,
            handle_server_event,
            handle_player_entity_event
        ).chain())
        .add_systems(PostUpdate, 
            handle_hit
            .after(ServerBootSet::Cache)
            .before(ServerSet::Send)
        );
    }
}

fn setup_ball(mut commands: Commands) {
    let ball_1 = commands.spawn((
        Replicated,
        Culling::Disable,
        Ball::ServerSimulation,
        TransformBundle::from_transform(Transform{
            translation: BALL_POSITION_1,
            ..default()
        }),
        DynamicRigidBodyBundle::new(
            BALL_MASS, 
            Vec3::ZERO, 
            Vec3::ZERO
        ),
        NetworkTranslationBundle::<NetworkTranslation3D>::new(
            BALL_POSITION_1, 
            TranslationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        NetworkRotationBundle::<NetworkEuler>::new(
            Quat::IDENTITY, 
            RotationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        Collider::ball(BALL_RADIUS)
    ))
    .id();
    info!("ball 1: {ball_1:?} spawned");

    let ball_2 = commands.spawn((
        Replicated,
        Culling::Disable,
        Ball::ClientPrediction,
        TransformBundle::from_transform(Transform{
            translation: BALL_POSITION_2,
            ..default()
        }),
        DynamicRigidBodyBundle::new(
            BALL_MASS, 
            Vec3::ZERO, 
            Vec3::ZERO
        ),
        NetworkTranslationBundle::<NetworkTranslation3D>::new(
            BALL_POSITION_2, 
            TranslationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        NetworkRotationBundle::<NetworkEuler>::new(
            Quat::IDENTITY, 
            RotationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        NetworkLinearVelocity3D::default(),
        NetworkAngularVelocity3D::default(),
        Collider::ball(BALL_RADIUS)
    ))
    .id();
    info!("ball 2: {ball_2:?} spwaned");
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
                        continue;
                    }
                };

                let uuid = match Uuid::from_slice(&user_data[0..16]) {
                    Ok(u) => u,
                    Err(e) => {
                        warn!("malformatted uuid for client: {:?}: {e}", client_id);
                        renet_server.disconnect(renet_client_id);
                        continue;
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
            let group = PlayerGroup::default();//random();
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
                NetworCharacterkTranslationBundle::<NetworkTranslation3D>::new(
                    player_start.translation,
                    TranslationAxis::Default, 
                    tick, 
                    LARGE_CACHE_SIZE
                ).expect("sytem time looks earlier than unix epoch"),
                NetworkCharacterRotationBundle::<NetworkAngleDegrees>::new(
                    Quat::IDENTITY, 
                    RotationAxis::Z,
                    tick, 
                    LARGE_CACHE_SIZE
                ).expect("sytem time looks earlier than unix epoch"),
                EventCache::<NetworkMovement2_5D>::with_capacity(
                    MEDIUM_CACHE_SIZE
                ),
                EventCache::<NetworkHit>::with_capacity(
                    SMALL_CACHE_SIZE
                )
            ));
        }
    }
}

fn handle_hit(
    mut shooter: Query<(
        &NetworkEntity,
        &mut EventCache<NetworkHit>,
        &ComponentCache<NetworkTranslation3D>,
        &ComponentCache<NetworkAngleDegrees>
    )>,
    query: Query<(
        &NetworkEntity,
        &ComponentCache<NetworkTranslation3D>
    )>,
    rapier: Res<RapierContext>,
    axis: Res<TransformAxis>,
    player_entity_map: Res<PlayerEntityMap>
) {
    let mut verified_hits = vec![];

    for (
        shooter_net_e,
        mut hit_cache, 
        shooter_trans_cache, 
        shooter_rot_cache
    ) in shooter.iter_mut() {
        for hit_snap in hit_cache.frontier_ref()
        .iter() {
            let origin = match shooter_trans_cache.latest_snapshot() {
                Some(s) => s.component()
                .to_vec3(axis.translation),
                None=> {
                    warn!("could not find latest translation");
                    continue;
                }
            };
            let dir = match shooter_rot_cache.latest_snapshot() {
                Some(s) => s.component()
                .to_quat(axis.rotation) * CHARACTER_FORWARD,
                None => {
                    warn!("could not find latest rotation");
                    continue;
                }
            };

            debug!("origin: {origin} dir: {dir}");

            let hit = hit_snap.event();
            let hit_toi_sq = (origin - hit.point).length_squared(); 
            if hit_toi_sq > FIRE_RANGE * FIRE_RANGE {
                warn!("toi is out of fire range, discarfing");
                continue;
            }

            let hit_client_id = ClientId::new(hit.client_id);
            let Some(hit_entity) = player_entity_map.get(&hit_client_id) else {
                warn!("player entity map does not have key: {hit_client_id:?}");
                continue;
            };

            // here checks only static obstacles
            // for further check, create parallel physics world or
            // just iterate over query
            if let Some((e, toi)) = rapier.cast_ray(
                origin, 
                dir, 
                FIRE_RANGE, 
                false, 
                QueryFilter::only_fixed()
                .exclude_sensors()
            ) {
                if e != *hit_entity && hit_toi_sq > toi * toi {
                    warn!("hit should be obstracted, discarding");
                    continue;
                }
            }
            
            let Ok((hit_net_e, hit_trans_cache)) = query.get(*hit_entity) else {
                warn!("query does not inclide entity: {hit_entity:?}");
                continue;
            };
            
            if hit_net_e.client_id() != hit_client_id {
                if cfg!(debug_assertions) {
                    panic!(
                        "player entity map is broken, received: {:?} found: {:?}",
                        hit_client_id,
                        hit_net_e.client_id()
                    );
                } else {
                    error!("player entity map is broken, skipping");
                    continue;
                }
            }

            // here checks only 2 possible positions
            // for more accurate check, just interpolate some steps
            let mut hit_translations = vec![];
            let tick = hit_snap.sent_tick();
            match hit_trans_cache.find_at_tick(tick - 1) {
                Some(s) => hit_translations.push(
                    s.component()
                    .to_vec3(axis.translation)
                ),
                None => warn!("could not find hit translation at tick: {tick} - 1")
            };
            match hit_trans_cache.find_at_tick(tick) {
                Some(s) => hit_translations.push(
                    s.component()
                    .to_vec3(axis.translation)
                ),
                None => warn!("could not find hit translation at tick: {tick}")
            };
            
            if hit_translations.len() == 0 {
                warn!("no translations for check");
                continue;
            }

            let capsule = Capsule::new_y(
                CHARACTER_HALF_HIGHT, 
                CHARACTER_RADIUS
            );
            for &t in hit_translations.iter() {
                let m = Isometry::from_parts(t.into(), Quat::IDENTITY.into());
                debug!("verifying hit for translation: {t}");  
                
                // sphere cast should be better for moving character
                if capsule.cast_ray(
                    &m, 
                    &Ray::new(origin.into(), dir.into()), 
                    FIRE_RANGE, 
                    false
                )
                .is_none() {
                    warn!("no cast hit, discarding");
                    continue;
                };

                debug!("translation: {t} is verified hit");
                verified_hits.push((
                    (hit_snap.received_timestamp() * 1000.0) as u64,
                    (shooter_net_e.client_id(), hit_client_id)
                ));
                break;
            };
        }

        hit_cache.cache();
    }    

    if verified_hits.len() > 1 {
        verified_hits.sort_unstable_by_key(|h| h.0);
    }
    
    for verified in verified_hits.iter() {
        info!(
            "verified hit: from: {:?} to: {:?}",
            verified.1.0,
            verified.1.1
        );
    }
}
