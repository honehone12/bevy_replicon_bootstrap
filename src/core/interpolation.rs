use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct InterpolationConfig {
    pub network_tick_delta: f64
}
