pub mod config;
pub mod level;
pub mod game_client;
pub mod game_server;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::{prelude::*, quick_lib::distance_culling::DistanceCullingAppExt};
use config::{BASE_SPEED, DEV_NETWORK_TICK_DELTA64};

pub struct GameCommonPlugin;

impl Plugin for GameCommonPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMovementParams{
            base_speed: BASE_SPEED
        })
        .add_plugins(RepliconActionPlugin)
        .use_network_transform_2d(move_2d, DEV_NETWORK_TICK_DELTA64)
        .use_replicated_component_snapshot::<NetworkTranslation2D>()
        .use_replicated_component_snapshot::<NetworkYaw>()
        .use_distance_culling::<NetworkTranslation2D>()
        .add_client_event::<NetworkFire>(ChannelKind::Ordered)
        .add_server_event::<ForceReplicate<NetworkTranslation2D>>(ChannelKind::Ordered)
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

#[derive(Resource)]
pub struct PlayerMovementParams {
    pub base_speed: f32
}

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct NetworkFire {
    pub index: usize,
    pub timestamp: f64
}

impl NetworkEvent for NetworkFire {
    fn index(&self) -> usize {
        self.index
    }

    fn timestamp(&self) -> f64 {
        self.timestamp
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
