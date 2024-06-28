pub mod config;
pub mod level;
pub mod game_client;
pub mod game_server;

use anyhow::bail;
use rand::prelude::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;
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
            NetworkTranslationPlugin::<
                NetworkCharacterController,
                NetworkMovement2_5D
            >::new(),
            NetworkRotationPlugin::<
                NetworkAngle,
                NetworkMovement2_5D
            >::new(),

            ClientEventPlugin::<NetworkMovement2_5D>::new(ChannelKind::Unreliable),
            ClientEventPlugin::<NetworkFire>::new(ChannelKind::Ordered),
        ))
        .replicate::<PlayerPresentation>()
        .add_systems(FixedUpdate,
            update_character_controller_system
            .in_set(ClientBootSet::Update)
        )
        /*.add_systems(FixedUpdate,
            handle_character_controller_output
            .after(AFTER_PHYSICS_SET)
        )*/;
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
            color: Color::rgb(
                random(), 
                random(), 
                random()
            )
        }
    }
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
pub struct NetworkFire {
    pub index: usize,
    pub timestamp: f64
}

impl NetworkEvent for NetworkFire {
    #[inline]
    fn index(&self) -> usize {
        self.index
    }

    #[inline]
    fn timestamp(&self) -> f64 {
        self.timestamp
    }

    #[inline]
    fn validate(&self) -> anyhow::Result<()> {
        if !self.timestamp.is_finite() {
            bail!("failed to validate timestamp");
        }

        Ok(())
    }
}

pub fn update_character_controller_system(
    mut query: Query<(
        &mut Transform,
        &mut KinematicCharacterController,
        &mut EventSnapshots<NetworkMovement2_5D>
    )>,
    params: Res<PlayerMovementParams>,
    time: Res<Time<Fixed>>
) {
    for (mut transform, mut cc, mut movement_snaps) in query.iter_mut() {
        if movement_snaps.frontier_len() == 0 {
            continue;
        }

        movement_snaps.sort_frontier_by_index();

        for snap in movement_snaps.frontier_ref()
        .iter() {
            let mut d = match cc.translation {
                None => Vec3::ZERO,
                Some(v) => v
            };

            let movement = snap.event();

            if movement.rotation_axis != Vec2::ZERO {
                let mut angle = movement.rotation_axis.x;
                angle *= params.base_angular_speed * time.delta_seconds();

                transform.rotate_y(-angle.to_radians());
            }

            if movement.linear_axis != Vec2::ZERO {
                let axis = Vec3::new(
                    movement.linear_axis.x, 
                    0.0, 
                    -movement.linear_axis.y
                ).normalize();
                
                let dir = (transform.rotation * axis)
                .normalize();

                d += dir * (params.base_speed * time.delta_seconds());
                cc.translation = Some(d);
            }
        }

        movement_snaps.cache();
    }
}

pub fn handle_character_controller_output(
    query: Query<&KinematicCharacterControllerOutput>
) {
    for out in query.iter() {
        info!(
            "{} : {}",
            out.desired_translation,
            out.effective_translation
        );
    }
}

pub fn handle_transport_error(mut errors: EventReader<NetcodeTransportError>) {
    for e in errors.read() {
        panic!("{e}")
    }
}
