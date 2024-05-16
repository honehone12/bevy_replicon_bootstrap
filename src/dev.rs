pub mod config;
pub mod level;
pub mod event;
pub mod client;
pub mod server;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::prelude::*;
use config::*;
use event::NetworkMovement2D;

pub struct GameCommonPlugin;

impl Plugin for GameCommonPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMovementParams{
            base_speed: BASE_SPEED,
            translation_error_threashold: TRANSLATION_ERROR_THREASHOLD,
            prediction_error_count_threashold: PREDICTION_ERROR_COUNT_THREASHOLD
        })
        .replicate::<PlayerPresentation>();
    }
}

#[derive(Resource)]
pub struct PlayerMovementParams {
    pub base_speed: f32,
    pub translation_error_threashold: f32,
    pub prediction_error_count_threashold: u32
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

pub fn move_2d(
    translation: &mut NetworkTranslation2D,
    movement: &NetworkMovement2D,
    params: &PlayerMovementParams,
    time: &Time<Fixed>
) {
    let mut dir = movement.axis.normalize();
    dir.y *= -1.0;
    translation.0 += dir * (params.base_speed * time.delta_seconds())
}

pub fn handle_transport_error(mut errors: EventReader<NetcodeTransportError>) {
    for e in errors.read() {
        panic!("{e}")
    }
}

pub fn error(error: anyhow::Error) {
    panic!("{error}");
}
