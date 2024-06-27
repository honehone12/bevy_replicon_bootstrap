use serde::{Serialize, Deserialize};
use bevy::prelude::*;
use crate::core::{NetworkTranslation, LinearInterpolatable};

#[derive(Component, Serialize, Deserialize, Clone, Copy, Default)]
pub struct NetworkCharacterController(pub Vec3);

impl NetworkTranslation for NetworkCharacterController {
    #[inline]
    fn from_vec3(vec: Vec3, _: crate::TranslationAxis) -> Self {
        Self(vec)
    }

    #[inline]
    fn to_vec3(&self, _: crate::TranslationAxis) -> Vec3 {
        self.0
    }
}

impl LinearInterpolatable for NetworkCharacterController {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, per: f32) -> Self {
        Self(self.0.lerp(rhs.0, per))
    }
}
