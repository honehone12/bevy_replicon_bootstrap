use bevy::prelude::*;

pub trait LinearInterpolatable: Component {
    fn linear_interpolate(&self, rhs: &Self, per: f32) -> Self;
}
