use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Component, Serialize, Deserialize)]
pub enum NetworkRigidBody {
    ServerSimulation,
    ClientPrediction
}

#[derive(Component, Serialize, Deserialize, Default)]
pub struct NetworkRBLinearVelocity3D(Vec3);

#[derive(Component, Serialize, Deserialize, Default)]
pub struct NetworkRBAngularVelocity3D(pub Vec3);

#[derive(Component, Serialize, Deserialize, Default)]
pub struct NetworkRBTranslation3D(pub Vec3);

#[derive(Component, Serialize, Deserialize, Default)]
pub struct NetworkRBRotation3D(pub Vec3);
