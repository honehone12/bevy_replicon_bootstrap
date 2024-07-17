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
        ).chain());
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
                EventSnapshots::<NetworkMovement2_5D>::with_capacity(
                    MEDIUM_CACHE_SIZE
                ),
                EventSnapshots::<NetworkHit>::with_capacity(
                    SMALL_CACHE_SIZE
                )
            ));
        }
    }
}

