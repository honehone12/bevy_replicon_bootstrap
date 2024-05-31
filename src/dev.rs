pub mod config;
pub mod level;
pub mod game_client;
pub mod game_server;

use std::marker::PhantomData;
use anyhow::bail;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::prelude::*;
use config::*;

pub struct GameCommonPlugin;

impl Plugin for GameCommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RepliconActionPlugin)
        .add_plugins(NetworkTransformPlugin{
            translation_axis: TranslationAxis::XZ, 
            rotation_axis: RotationAxis::Y,
            update_fn: update_transform,
            params: PlayerMovementParams{
                base_speed: BASE_SPEED,
                base_angular_speed: BASE_ANGULAR_SPEED
            },
            network_tick_delta: DEV_NETWORK_TICK_DELTA64,
            translation_error_threshold: TRANSLATION_ERROR_THRESHOLD,
            rotation_error_threshold: ROTATION_ERROR_THRESHOLD,
            error_count_threshold: PREDICTION_ERROR_COUNT_THRESHOLD
        })
        .use_component_snapshot::<NetworkTranslation2D>()
        .use_component_snapshot::<NetworkAngle>()
        .use_client_event_snapshot::<NetworkMovement2D>(ChannelKind::Unreliable)
        .add_client_event::<NetworkFire>(ChannelKind::Ordered)
        .replicate::<PlayerPresentation>();
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

#[derive(Component, Default, Eq, PartialEq, Hash, Clone, Copy)]
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
            bail!("failed to validate timestamp")
        }

        Ok(())
    }
}

fn update_transform(
    translation: &mut NetworkTranslation2D,
    rotation: &mut NetworkAngle,
    movement: &NetworkMovement2D,
    params: &PlayerMovementParams,
    time: &Time<Fixed>
) {
    if movement.rotation_axis != Vec2::ZERO {
        let mut angle = movement.rotation_axis.x;
        angle *= params.base_angular_speed * time.delta_seconds();
        rotation.0 = (rotation.0 - angle) % 360.0;
        if rotation.0 < 0.0 {
            rotation.0 += 360.0;
        }
    }

    if movement.linear_axis != Vec2::ZERO {
        let axis = Vec3::new(
            movement.linear_axis.x, 
            0.0, 
            -movement.linear_axis.y
        ).normalize();
        
        let dir = (rotation.to_quat(RotationAxis::Y) * axis)
        .xz()
        .normalize();
        translation.0 += dir * (params.base_speed * time.delta_seconds());
    }
}

pub fn handle_transport_error(mut errors: EventReader<NetcodeTransportError>) {
    for e in errors.read() {
        panic!("{e}")
    }
}

pub fn error(error: anyhow::Error) {
    panic!("{error}");
}
