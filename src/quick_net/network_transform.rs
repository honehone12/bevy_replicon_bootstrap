use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::core::interpolation::LinearInterpolatable;

#[derive(Component, Serialize, Deserialize, Default, Clone)]
pub struct NetworkTranslation2D(pub Vec2);

impl LinearInterpolatable for NetworkTranslation2D {
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl NetworkTranslation2D {
    #[inline]
    pub fn from_3d(vec3: Vec3) -> Self {
        Self(Vec2::new(vec3.x, vec3.z))
    }
    
    #[inline]
    pub fn to_3d(&self) -> Vec3 {
        Vec3::new(self.0.x, 0.0, self.0.y)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone)]
pub struct NetworkTranslation3D(pub Vec3);

impl LinearInterpolatable for NetworkTranslation3D {
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl NetworkTranslation3D {
    #[inline]
    pub fn new(vec3: Vec3) -> Self {
        Self(vec3)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone)]
pub struct NetworkYaw(pub f32);

impl LinearInterpolatable for NetworkYaw {
    fn linear_interpolate(&self, rhs: &Self, t: f32) -> Self {
        Self(self.0.lerp(rhs.0, t))
    }
}

impl NetworkYaw {
    #[inline]
    pub fn from_quat(quat: Quat) -> Self {
        Self(quat.to_euler(EulerRot::YXZ).0)
    }

    #[inline]
    pub fn to_quat(&self) -> Quat {
        Quat::from_rotation_y(self.0.to_radians())
    }
}
