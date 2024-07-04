use serde::{Serialize, Deserialize};
use anyhow::bail;
use bevy::prelude::*;
use crate::prelude::*;

#[derive(Event, Serialize, Deserialize, Clone, Default)]
pub struct NetworkMovement2D {
    pub current_translation: Vec2,
    pub current_rotation: f32,
    pub linear_axis: Vec2,
    pub rotation_axis: Vec2,
    pub bits: u16,
    pub index: usize,
    pub tick: u32
}

impl NetworkEvent for NetworkMovement2D {
    #[inline]
    fn index(&self) -> usize {
        self.index
    }

    #[inline]
    fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    fn validate(&self) -> anyhow::Result<()> {
        if !self.current_translation.is_finite() {
            bail!("failed to validate current translation");
        }
        if !self.current_rotation.is_finite() {
            bail!("failed to validate current rotation");
        }
        if !self.linear_axis.is_finite() {
            bail!("failed to validate linear axis");
        }
        if !self.rotation_axis.is_finite() {
            bail!("failed to validate rotation axis");
        }

        Ok(())
    }
}

impl NetworkMovement for NetworkMovement2D {
    #[inline]
    fn current_translation(&self, axis: TranslationAxis) -> Vec3 {
        match axis {
            TranslationAxis::Default
            | TranslationAxis::XY => Vec3::new(
                self.current_translation.x, 
                self.current_translation.y,
                0.0
            ),
            TranslationAxis::XZ => Vec3::new(
                self.current_translation.x,
                0.0,
                self.current_translation.y
            )
        }
    }

    #[inline]
    fn current_rotation(&self, axis: RotationAxis) -> Quat {
        match axis {
            RotationAxis::Y => Quat::from_rotation_y(self.current_rotation.to_radians()),
            RotationAxis::Z
            | RotationAxis::Default => Quat::from_rotation_z(
                self.current_rotation.to_radians()
            )
        }
    }
}

#[derive(Event, Serialize, Deserialize, Clone, Default)]
pub struct NetworkMovement2_5D {
    pub current_translation: Vec3,
    pub current_yaw: f32,
    pub linear_axis: Vec2,
    pub rotation_axis: Vec2,
    pub bits: u16,
    pub index: usize,
    pub tick: u32
}

impl NetworkEvent for NetworkMovement2_5D {
    #[inline]
    fn index(&self) -> usize {
        self.index
    }

    #[inline]
    fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    fn validate(&self) -> anyhow::Result<()> {
        if !self.current_translation.is_finite() {
            bail!("failed to validate current translation");
        }
        if !self.current_yaw.is_finite() {
            bail!("failed to validate current rotation");
        }
        if !self.linear_axis.is_finite() {
            bail!("failed to validate linear axis");
        }
        if !self.rotation_axis.is_finite() {
            bail!("failed to validate rotation axis");
        }

        Ok(())
    }
}

impl NetworkMovement for NetworkMovement2_5D {
    #[inline]
    fn current_translation(&self, _: TranslationAxis) -> Vec3 {
        self.current_translation
    }

    #[inline]
    fn current_rotation(&self, axis: RotationAxis) -> Quat {
        match axis {
            RotationAxis::Y
            | RotationAxis::Default => Quat::from_rotation_y(
                self.current_yaw.to_radians()
            ),
            RotationAxis::Z => Quat::from_rotation_z(self.current_yaw.to_radians())
        }
    }
}