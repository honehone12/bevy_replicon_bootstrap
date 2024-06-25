use anyhow::bail;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::prelude::*;

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation2D(pub Vec2);

impl LinearInterpolatable for NetworkTranslation2D {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl DistanceCalculatable for NetworkTranslation2D {
    #[inline]
    fn distance(&self, rhs: &Self) -> f32 {
        self.0.distance_squared(rhs.0)
    }   
}

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
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation3D(pub Vec3);

impl LinearInterpolatable for NetworkTranslation3D {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl DistanceCalculatable for NetworkTranslation3D {
    #[inline]
    fn distance(&self, rhs: &Self) -> f32 {
        self.0.distance_squared(rhs.0)
    }   
}

impl NetworkTranslation for NetworkTranslation3D {
    #[inline]
    fn from_vec3(vec3: Vec3, _: TranslationAxis) -> Self {
        Self(vec3)
    }
    
    #[inline]
    fn to_vec3(&self, _: TranslationAxis) -> Vec3 {
        self.0
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkAngle(pub f32);

impl LinearInterpolatable for NetworkAngle {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, t: f32) -> Self {
        let mut delta = (rhs.0 - self.0) % 360.0;
        if delta < 0.0 {
            delta += 360.0;
        }
        
        if delta > 180.0 {
            delta -= 360.0;
        }

        delta *= t;
        Self((self.0 + delta) % 360.0)
    }
}

impl NetworkRotation for NetworkAngle {
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
}

#[derive(Bundle)]
pub struct NetworkTranslationBundle<T>
where T: NetworkTranslation + Serialize + DeserializeOwned + Default + Copy {
    pub translation: T,
    pub snaps: ComponentSnapshots<T>,
    pub prediction_error: PredioctionError<T>
}

impl<T> NetworkTranslationBundle<T>
where T: NetworkTranslation + Serialize + DeserializeOwned + Default + Copy {
    #[inline]
    pub fn new(
        init: Vec3,
        axis: TranslationAxis, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let translation = T::from_vec3(init, axis);
        snaps.insert(translation, tick)?;
        
        Ok(Self{ 
            translation, 
            snaps ,
            prediction_error: default()
        })
    }
}

#[derive(Bundle)]
pub struct NetworkRotationBundle<R>
where R: NetworkRotation + Serialize + DeserializeOwned + Default + Copy {
    pub rotation: R,
    pub snaps: ComponentSnapshots<R>,
    pub prediction_error: PredioctionError<R>
}

impl<R> NetworkRotationBundle<R>
where R: NetworkRotation + Serialize + DeserializeOwned + Default + Copy {
    #[inline]
    pub fn new(
        init: Quat, 
        axis: RotationAxis,
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let rotation = R::from_quat(init, axis);
        snaps.insert(rotation, tick)?;
        
        Ok(Self{ 
            rotation, 
            snaps,
            prediction_error: default() 
        })
    }
}

#[derive(Event, Serialize, Deserialize, Clone, Default)]
pub struct NetworkMovement2D {
    pub current_translation: Vec2,
    pub current_rotation: f32,
    pub linear_axis: Vec2,
    pub rotation_axis: Vec2,
    pub bits: u16,
    pub index: usize,
    pub timestamp: f64
}

impl NetworkEvent for NetworkMovement2D {
    #[inline]
    fn index(&self) -> usize {
        self.index
    }
    
    #[inline]
    fn timestamp(&self) -> f64 {
        self.timestamp
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
        if !self.timestamp.is_finite() {
            bail!("failed to validate timestamp");
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


