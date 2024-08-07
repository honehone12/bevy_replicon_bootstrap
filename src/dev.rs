pub mod config;
pub mod level;
pub mod game_client;
pub mod game_server;

use anyhow::bail;
use rand::prelude::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
//use bevy_replicon_renet::renet::transport::NetcodeTransportError;
use bevy_rapier3d::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;
use config::*;

pub struct GameCommonPlugin;

impl Plugin for GameCommonPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMovementParams{
            base_speed: BASE_SPEED,
            base_angular_speed: BASE_ANGULAR_SPEED,
        })
        .add_plugins((
            NetworkBootPlugin {
                transform_axis: TransformAxis{
                    translation: default(),
                    rotation: RotationAxis::Y
                },
                replication_config: ReplicationConfig{
                    translation_threshold: TRANSLATION_REPLICATION_THRESHOLD,
                    rotation_threashold: ROTATION_REPLICATION_THRESHOLD
                },
                interpolation_config: InterpolationConfig { 
                    network_tick_delta: DEV_NETWORK_TICK_DELTA64 
                },
                prediction_config: PredictionConfig { 
                    translation_threshold: TRANSLATION_ERROR_THRESHOLD, 
                    rotation_threshold: ROTATION_ERROR_THRESHOLD, 
                    force_replicate_error_count: PREDICTION_ERROR_COUNT_THRESHOLD 
                },
            },
            Rapier3DPlugin{
                delta_time: PHYSICS_FIXED_TICK_DELTA,
                substeps: PHYSICS_SUBSTEPS
            }
        ))
        .add_plugins((
            NetworkCharacterTranslationPlugin::<
                NetworkTranslation3D,
                NetworkMovement2_5D
            >::new(),
            NetworkCharacterRotationPlugin::<
                NetworkAngleDegrees,
                NetworkMovement2_5D
            >::new(),
            NetworkRotationPlugin::<NetworkEuler>::new(),

            NetworkLinearVelocityPlugin::<NetworkLinearVelocity3D>::new(),
            NetworkAngularVelocityPlugin::<NetworkAngularVelocity3D>::new(),
            
            ClientEventPlugin::<NetworkHit>::new(ChannelKind::Ordered),
            ClientEventPlugin::<NetworkMovement2_5D>::new(ChannelKind::Unreliable)
        ))
        .replicate::<PlayerPresentation>()
        .replicate::<Ball>()
        .add_systems(FixedUpdate,(
            ground_check_system,
            update_character_controller_system,
            apply_gravity_system
        ).chain(
        ).in_set(BootsetMain));
    }
}

#[derive(Component, Serialize, Deserialize)]
pub struct PlayerPresentation {
    pub color: Color
}

impl PlayerPresentation {
    #[inline]
    pub fn random() -> Self {
        Self{
            color: Color::srgb(
                random(), 
                random(), 
                random()
            )
        }
    }
}

#[derive(Component, Default)]
pub struct Jump {
    power: f32,
    grounded: bool
}

#[derive(Component, Default)]
pub struct PlayerGroup {
    pub group: u8
}

impl PlayerGroup {
    #[inline]
    pub fn random() -> Self {
        let group = if random() {
            1
        } else {
            0
        };
        Self { group }
    }
}

impl RelevantGroup for PlayerGroup {
    #[inline]
    fn is_relevant(&self, rhs: &Self) -> bool {
        self.group == rhs.group
    }
}

#[derive(Resource, Clone)]
pub struct PlayerMovementParams {
    pub base_speed: f32,
    pub base_angular_speed: f32,
}

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct NetworkHit {
    pub point: Vec3,
    pub client_id: u64,
    
    pub index: u64,
    pub tick: u32
}

impl NetworkEvent for NetworkHit {
    #[inline]
    fn index(&self) -> usize {
        self.index as usize
    }

    #[inline]
    fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    fn validate(&self) -> anyhow::Result<()> {
        if !self.point.is_finite() {
            bail!("failed to validate point");
        }

        Ok(())
    }
}

#[derive(Component, Serialize, Deserialize)]
pub enum Ball {
    ServerSimulation,
    ClientPrediction
}

fn ground_check_system(
    mut query: Query<(
        &Transform,
        &KinematicCharacterController,
        &mut Jump
    )>,
    rapier: Res<RapierContext>
) {
    for (transform, cc, mut jump) in query.iter_mut() {
        let height = CHARACTER_HALF_HIGHT * 2.0;
        let offset = match cc.offset {
            CharacterLength::Absolute(n) => n,
            CharacterLength::Relative(n) => n * height * 2.0
        };
        let error = 0.05;

        jump.grounded = rapier.cast_ray(
            transform.translation - height, 
            transform.down().into(), 
            offset + error, 
            false, 
            default()
        ).is_some();
    }
}

fn update_character_controller_system(
    mut query: Query<(
        &mut Transform,
        &mut KinematicCharacterController,
        &mut Jump,
        &mut EventCache<NetworkMovement2_5D>
    )>,
    params: Res<PlayerMovementParams>,
    time: Res<Time<Fixed>>
) {
    for (
        mut transform, 
        mut cc, 
        mut jump,
        mut movements
    ) in query.iter_mut() {
        if movements.frontier_len() == 0 {
            continue;
        }

        movements.sort_frontier_by_index();
        let delta_time = time.delta_seconds();

        for snap in movements.frontier_ref()
        .iter() {
            let movement = snap.event();

            if movement.rotation_axis != Vec2::ZERO {
                let mut angle_delta = movement.rotation_axis.x;
                angle_delta *= params.base_angular_speed * delta_time;
                trace!("angle delta: {angle_delta}");

                transform.rotate_y(-angle_delta.to_radians());
            }

            if movement.linear_axis != Vec2::ZERO {
                let axis = Vec3::new(
                    movement.linear_axis.x, 
                    0.0, 
                    -movement.linear_axis.y
                ).normalize();
                let dir = transform.rotation * axis;
                
                let translation_delta = dir * params.base_speed * delta_time;
                trace!("translation delta: {translation_delta}");
        
                match cc.translation {
                    Some(ref mut v) => *v += translation_delta,
                    None => cc.translation = Some(translation_delta)
                }
            }

            if movement.bits & 0x01 != 0 {
                if jump.grounded {
                    jump.power = JUMP_POWER;    
                }    
            }
        }

        movements.cache();
    }
}

fn apply_gravity_system(
    mut query: Query<(
        &mut KinematicCharacterController, 
        &mut Jump
    )>,
    time: Res<Time<Fixed>>
) {
    for (mut cc, mut jump) in query.iter_mut() {
        if jump.grounded && jump.power != JUMP_POWER {
            if jump.power != 0.0 {
                jump.power = 0.0;
            }
            
            continue;   
        }

        let delta_time = time.delta_seconds();
        let mass = cc.custom_mass.unwrap_or(1.0);
        let g = GRAVITY * mass * delta_time;
        let dy = jump.power * delta_time + g;
        jump.power += g;
        
        match cc.translation {
            Some(ref mut v) => v.y += dy,
            None => cc.translation = Some(Vec3::new(0.0, dy, 0.0))
        }
    }
}

// pub fn handle_netcode_transport_error(mut errors: EventReader<NetcodeTransportError>) {
//     for e in errors.read() {
//         panic!("transport error: {e}")
//     }
// }
