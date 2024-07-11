use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::prelude::*;

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation2D(pub Vec2);

impl NetworkTranslation for NetworkTranslation2D {
    #[inline]
    fn from_vec3(vec3: Vec3, axis: TranslationAxis) -> Self {
        match axis {
            TranslationAxis::Default 
            | TranslationAxis::XY => Self(Vec2::new(vec3.x, vec3.y)),
            TranslationAxis::XZ => Self(Vec2::new(vec3.x, vec3.z)),
        }
    }
    
    #[inline]
    fn to_vec3(&self, axis: TranslationAxis) -> Vec3 {
        match axis {
            TranslationAxis::Default
            | TranslationAxis::XY => Vec3::new(self.0.x, self.0.y, 0.0),
            TranslationAxis::XZ => Vec3::new(self.0.x, 0.0,  self.0.y),
        }
    }

    #[inline]
    fn interpolate(&self, rhs: &Self, s: f32, axis: TranslationAxis) 
    -> Vec3 {
        Self(self.0.lerp(rhs.0, s)).to_vec3(axis)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation3D(pub Vec3);

impl NetworkTranslation for NetworkTranslation3D {
    #[inline]
    fn from_vec3(vec3: Vec3, _: TranslationAxis) -> Self {
        Self(vec3)
    }
    
    #[inline]
    fn to_vec3(&self, _: TranslationAxis) -> Vec3 {
        self.0
    }

    #[inline]
    fn interpolate(&self, rhs: &Self, s: f32, _: TranslationAxis) 
    -> Vec3 {
        self.0.lerp(rhs.0, s)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkAngleDegrees(pub f32);

impl NetworkRotation for NetworkAngleDegrees {
    #[inline]
    fn from_quat(quat: Quat, axis: RotationAxis) -> Self {
        match axis {
            RotationAxis::Y => Self(quat.to_euler(EulerRot::YXZ).0.to_degrees()),
            RotationAxis::Z
            | RotationAxis::Default => Self(quat.to_euler(EulerRot::YXZ).2.to_degrees())
        }
    }

    #[inline]
    fn to_quat(&self, axis: RotationAxis) -> Quat {
        match axis {
            RotationAxis::Y => Quat::from_rotation_y(self.0.to_radians()),
            RotationAxis::Z
            | RotationAxis::Default => Quat::from_rotation_z(self.0.to_radians()),
        }
    }

    #[inline]
    fn interpolate(&self, rhs: &Self, t: f32, axis: RotationAxis) 
    -> Quat {
        let mut delta = (rhs.0 - self.0) % 360.0;
        if delta < 0.0 {
            delta += 360.0;
        }
        
        if delta > 180.0 {
            delta -= 360.0;
        }

        delta *= t;
        Self((self.0 + delta) % 360.0).to_quat(axis)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkEuler(pub Vec3);

impl NetworkRotation for NetworkEuler {
    #[inline]
    fn from_quat(quat: Quat, _: RotationAxis) -> Self {
        Self(quat.to_euler(EulerRot::XYZ).into())
    }

    #[inline]
    fn to_quat(&self, _: RotationAxis) -> Quat {
        Quat::from_euler(EulerRot::XYZ, 
            self.0.x, 
            self.0.y, 
            self.0.z
        )
    }

    #[inline]
    fn interpolate(&self, rhs: &Self, per: f32, _: RotationAxis) 
    -> Quat {
        Quat::from_euler(EulerRot::XYZ, 
            self.0.x, 
            self.0.y, 
            self.0.z
        )
        .slerp(
            Quat::from_euler(EulerRot::XYZ, 
                rhs.0.x, 
                rhs.0.y, 
                rhs.0.z
            ), 
            per
        )
    }
}
