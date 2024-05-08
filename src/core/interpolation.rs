use bevy::prelude::*;

pub trait Interpolatable: Component {
    fn interpolate(&self, rhs: &Self, per: f32) -> Self;
}
