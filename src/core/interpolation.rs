use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct InterpolationConfig {
    pub network_tick_delta: f64
}

pub trait LinearInterpolatable: Component + Clone {
    fn linear_interpolate(&self, rhs: &Self, per: f32) -> Self;
}
