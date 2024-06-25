pub mod network_entity;
pub mod network_event;
pub mod player_entity;
pub mod interpolation;
pub mod prediction;

pub use network_entity::*;
pub use network_event::*;
pub use player_entity::*; 
pub use interpolation::*;
pub use prediction::*;

use serde::{Serialize, de::DeserializeOwned};
use bevy::prelude::*;

#[derive(Default, Clone, Copy)]
pub enum TranslationAxis {
    #[default]
    Default,
    XY,
    XZ
}

#[derive(Default, Clone, Copy)]
pub enum RotationAxis {
    #[default]
    Default,
    Y,
    Z
}

#[derive(Resource, Default)]
pub struct TransformAxis {
    pub translation: TranslationAxis,
    pub rotation: RotationAxis,
}

pub trait DistanceCalculatable: Component {
    fn distance(&self, rhs: &Self) -> f32;
}

pub trait NetworkTranslation
: Component + DistanceCalculatable + LinearInterpolatable
+ Serialize + DeserializeOwned + Clone + Default {
    fn from_vec3(vec: Vec3, axis: TranslationAxis) -> Self;
    fn to_vec3(&self, axis: TranslationAxis) -> Vec3;
}

pub trait NetworkRotation
: Component + LinearInterpolatable 
+ Serialize + DeserializeOwned + Clone + Default {
    fn from_quat(quat: Quat, axis: RotationAxis) -> Self;
    fn to_quat(&self, axis: RotationAxis) -> Quat;
}

pub trait NetworkMovement: NetworkEvent {
    fn current_translation(&self, axis: TranslationAxis) -> Vec3;
    fn current_rotation(&self, axis: RotationAxis) -> Quat;
}

pub type NetworkTransformUpdateFn<T, R, E, P> 
= fn(&mut T, &mut R, &E, &P, &Time<Fixed>);

#[derive(Resource)]
pub struct NetworkTransformUpdate<T, R, E, P>(
    NetworkTransformUpdateFn<T, R, E, P>
)
where 
T: NetworkTranslation, 
R: NetworkRotation, 
E: NetworkMovement, 
P: Resource;

impl<T, R, E, P> NetworkTransformUpdate<T, R, E, P>
where 
T: NetworkTranslation, 
R: NetworkRotation,
E: NetworkMovement, 
P: Resource {
    #[inline]
    pub fn new(update_fn: NetworkTransformUpdateFn<T, R, E, P>) 
    -> Self {
        Self(update_fn)
    }

    #[inline]
    pub fn update(&self) -> NetworkTransformUpdateFn<T, R, E, P> {
        self.0
    }
}
