pub mod network_entity;
pub mod network_event;
pub mod network_resource;
pub mod player_entity;
pub mod interpolation;
pub mod prediction;
pub mod boot_system_set;
pub mod player_start_line;
pub mod latest_confirmed_tick;

pub use network_entity::*;
pub use network_event::*;
pub use network_resource::*;
pub use player_entity::*; 
pub use interpolation::*;
pub use prediction::*;
pub use boot_system_set::*;
pub use player_start_line::*;
pub use latest_confirmed_tick::*;

use serde::{de::DeserializeOwned, Serialize};
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

#[derive(Resource, Clone, Default)]
pub struct TransformAxis {
    pub translation: TranslationAxis,
    pub rotation: RotationAxis,
}

#[derive(Resource, Clone)]
pub struct ReplicationConfig {
    pub translation_threshold: f32,
    pub rotation_threashold: f32
}

impl ReplicationConfig {
    #[inline]
    pub fn translation_threshold_sq(&self) -> f32 {
        self.translation_threshold * self.translation_threshold
    }
}

pub trait NetworkTranslation: Component
+ Serialize + DeserializeOwned + Clone + Copy + Default {
    fn from_vec3(vec: Vec3, axis: TranslationAxis) -> Self;
    fn to_vec3(&self, axis: TranslationAxis) -> Vec3;
    fn interpolate(&self, rhs: &Self, per: f32, axis: TranslationAxis) 
    -> Vec3;
}

pub trait NetworkRotation: Component
+ Serialize + DeserializeOwned + Clone + Copy + Default {
    fn from_quat(quat: Quat, axis: RotationAxis) -> Self;
    fn to_quat(&self, axis: RotationAxis) -> Quat;
    fn interpolate(&self, rhs: &Self, per: f32, axis: RotationAxis) 
    -> Quat;
}

pub trait NetworkLinearVelocity: Component
+ Serialize + DeserializeOwned + Default {
    fn from_vec3(vec: Vec3, axis: TranslationAxis) -> Self;
    fn to_vec3(&self, axis: TranslationAxis) -> Vec3;
}

pub trait NetworkAngularVelocity: Component
+ Serialize + DeserializeOwned + Default {
    fn from_vec3(vec: Vec3, axis: RotationAxis) -> Self;
    fn to_vec3(&self, axis: RotationAxis) -> Vec3;
}

pub trait NetworkMovement: NetworkEvent {
    fn current_translation(&self, axis: TranslationAxis) -> Vec3;
    fn current_rotation(&self, axis: RotationAxis) -> Quat;
}
