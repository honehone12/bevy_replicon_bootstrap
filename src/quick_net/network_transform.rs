use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::core::interpolation::Interpolatable;


#[derive(Component, Serialize, Deserialize, Default, Clone)]
pub struct NetworkTranslation2D(pub Vec2);

impl Interpolatable for NetworkTranslation2D {
    fn interpolate(&self, rhs: &Self, s: f32) -> Self {
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
pub struct NetworkYaw(pub f32);

impl Interpolatable for NetworkYaw {
    fn interpolate(&self, rhs: &Self, t: f32) -> Self {
        Self(self.0.lerp(rhs.0, t))
    }
}

impl NetworkYaw {
    #[inline]
    pub fn from_3d(quat: Quat) -> Self {
        Self(quat.to_euler(EulerRot::YXZ).0)
    }

    #[inline]
    pub fn to_3d(&self) -> Quat {
        Quat::from_rotation_y(self.0.to_radians())
    }
}
