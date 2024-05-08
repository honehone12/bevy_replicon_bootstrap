use bevy::prelude::*;

pub trait Interpolatable: Component {
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}
