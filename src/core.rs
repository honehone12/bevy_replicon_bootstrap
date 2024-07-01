pub mod network_entity;
pub mod network_event;
pub mod player_entity;
pub mod interpolation;
pub mod prediction;
pub mod boot_system_set;
pub mod player_start_line;

pub use network_entity::*;
pub use network_event::*;
pub use player_entity::*; 
pub use interpolation::*;
pub use prediction::*;
pub use boot_system_set::*;
pub use player_start_line::*;

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

#[derive(Resource, Clone, Default)]
pub struct TransformAxis {
    pub translation: TranslationAxis,
    pub rotation: RotationAxis,
}

pub trait NetworkTranslation
: Component + LinearInterpolatable
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
