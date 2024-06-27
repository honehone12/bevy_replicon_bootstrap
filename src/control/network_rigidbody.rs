use bevy::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Component, Serialize, Deserialize, Clone)]
pub enum NetworkRigidBody {
    ServerSimulation {
        translation: Vec3,
        euler: Vec3
    },
    ClientPrediction {
        translation: Vec3,
        euler: Vec3,
        linear_velocity: Vec3,
        angular_velocity: Vec3
    }
}

impl NetworkRigidBody {
    #[inline]
    pub fn default_server_simulation() -> Self {
        Self::ServerSimulation { 
            translation: Vec3::ZERO, 
            euler: Vec3::ZERO 
        }
    }

    #[inline]
    pub fn default_client_simulation() -> Self {
        Self::ClientPrediction { 
            translation: Vec3::ZERO, 
            euler: Vec3::ZERO, 
            linear_velocity: Vec3::ZERO, 
            angular_velocity: Vec3::ZERO 
        }
    }
}