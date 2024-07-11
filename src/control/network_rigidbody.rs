use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::core::*;

#[derive(Component, Serialize, Deserialize)]
pub enum NetworkRigidBody {
    ServerSimulation,
    ClientPrediction,
    ClientPredictionWithTransformInfo,
}

#[derive(Component, Serialize, Deserialize, Default)]
pub struct NetworkLinearVelocity3D(pub Vec3);

impl NetworkLinearVelocity for NetworkLinearVelocity3D {
    fn from_vec3(vec: Vec3, _: TranslationAxis) -> Self {
        Self(vec)
    }

    fn to_vec3(&self, _: TranslationAxis) -> Vec3 {
        self.0
    }
}

#[derive(Component, Serialize, Deserialize, Default)]
pub struct NetworkAngularVelocity3D(pub Vec3);

impl NetworkAngularVelocity for NetworkAngularVelocity3D {
    fn from_vec3(vec: Vec3, _: RotationAxis) -> Self {
        Self(vec)
    }

    fn to_vec3(&self, _: RotationAxis) -> Vec3 {
        self.0
    }
}