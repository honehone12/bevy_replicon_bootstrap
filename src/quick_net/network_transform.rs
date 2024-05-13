use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::core::{
    component_snapshot::ComponentSnapshots, 
    interpolation::LinearInterpolatable
};

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
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

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
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

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
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

#[derive(Bundle)]
pub struct NetworkTranslation2DWithSnapshots {
    translation: NetworkTranslation2D,
    snaps: ComponentSnapshots<NetworkTranslation2D>
}

impl NetworkTranslation2DWithSnapshots {
    #[inline]
    pub fn new(
        init: Vec3, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let translation = NetworkTranslation2D::from_3d(init);
        snaps.insert(translation, tick)?;
        
        Ok(Self{ 
            translation, 
            snaps 
        })
    }
}

#[derive(Bundle)]
pub struct NetworkTranslation3DWithSnapshots {
    translation: NetworkTranslation3D,
    snaps: ComponentSnapshots<NetworkTranslation3D>
}

impl NetworkTranslation3DWithSnapshots {
    #[inline]
    pub fn new(
        init: Vec3, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let translation = NetworkTranslation3D::new(init);
        snaps.insert(translation, tick)?;
        
        Ok(Self{ 
            translation, 
            snaps 
        })
    }
}

#[derive(Bundle)]
pub struct NetworkYawWithSnapshots {
    yaw: NetworkYaw,
    snaps: ComponentSnapshots<NetworkYaw>
}

impl NetworkYawWithSnapshots {
    #[inline]
    pub fn new(
        init: Quat, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let yaw = NetworkYaw::from_quat(init);
        snaps.insert(yaw, tick)?;
        
        Ok(Self{ 
            yaw, 
            snaps 
        })
    }
}
