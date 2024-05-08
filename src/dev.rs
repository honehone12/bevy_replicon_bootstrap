pub mod config;
pub mod level;
pub mod event;
pub mod client;
pub mod server;

use bevy::prelude::*;
use bevy_replicon_renet::renet::transport::NetcodeTransportError;

pub fn handle_transport_error(mut errors: EventReader<NetcodeTransportError>) {
    for e in errors.read() {
        panic!("{e}")
    }
}

pub fn error(error: anyhow::Error) {
    panic!("{error}");
}
