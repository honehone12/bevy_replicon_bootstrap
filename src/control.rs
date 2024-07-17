pub mod network_transform;
pub mod network_velocity;
pub mod network_movement;
pub(crate) mod systems;
pub(crate) mod physics_systems;

pub use network_transform::*;
pub use network_velocity::*;
pub use network_movement::*;
pub(crate) use systems::*;
pub(crate) use physics_systems::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::prelude::*;

#[derive(Bundle)]
pub struct NetworCharacterkTranslationBundle<T>
where T: NetworkTranslation {
    pub translation: T,
    pub snaps: ComponentSnapshots<T>,
    pub prediction_error: PredioctionError<T>
}

impl<T> NetworCharacterkTranslationBundle<T>
where T: NetworkTranslation{
    #[inline]
    pub fn new(
        init: Vec3,
        axis: TranslationAxis, 
        tick: u32,
        cache_size: usize
    ) -> anyhow::Result<Self> {
        let translation = T::from_vec3(init, axis);
        let snaps = ComponentSnapshots::with_init(
            translation,
            tick,
            cache_size
        )?;
        
        Ok(Self{ 
            translation, 
            snaps ,
            prediction_error: default()
        })
    }
}

#[derive(Bundle)]
pub struct NetworkCharacterRotationBundle<R>
where R: NetworkRotation {
    pub rotation: R,
    pub snaps: ComponentSnapshots<R>,
    pub prediction_error: PredioctionError<R>
}

impl<R> NetworkCharacterRotationBundle<R>
where R: NetworkRotation {
    #[inline]
    pub fn new(
        init: Quat, 
        axis: RotationAxis,
        tick: u32,
        cache_size: usize
    ) -> anyhow::Result<Self> {
        let rotation = R::from_quat(init, axis);
        let snaps = ComponentSnapshots::with_init(
            rotation,
            tick,
            cache_size
        )?;
      
        Ok(Self{ 
            rotation, 
            snaps,
            prediction_error: default() 
        })
    }
}

#[derive(Bundle)]
pub struct NetworkTranslationBundle<T>
where T: NetworkTranslation {
    pub translation: T,
    pub snaps: ComponentSnapshots<T>  
}

impl<T> NetworkTranslationBundle<T>
where T: NetworkTranslation {
    #[inline]
    pub fn new(
        init: Vec3,
        axis: TranslationAxis,
        tick: u32,
        cache_size: usize
    ) -> anyhow::Result<Self> {
        let translation = T::from_vec3(init, axis);
        let snaps = ComponentSnapshots::with_init(
            translation, 
            tick, 
            cache_size
        )?;

        Ok(Self { 
            translation, 
            snaps
        })
    }
}

#[derive(Bundle)]
pub struct NetworkRotationBundle<R>
where R: NetworkRotation {
    pub rotation: R,
    pub snaps: ComponentSnapshots<R>
}

impl<R> NetworkRotationBundle<R>
where R: NetworkRotation {
    #[inline]
    pub fn new(
        init: Quat,
        axis: RotationAxis,
        tick: u32,
        cache_size: usize,
    ) -> anyhow::Result<Self> {
        let rotation = R::from_quat(init, axis);
        let snaps = ComponentSnapshots::with_init(
            rotation, 
            tick, 
            cache_size
        )?;

        Ok(Self { 
            rotation, 
            snaps
        })
    }
}

#[derive(Bundle)]
pub struct CharacterControllerBundle {
    pub character_controller: KinematicCharacterController,
    pub rigidbody: RigidBody,
    pub capsule: Collider
}

impl CharacterControllerBundle {
    #[inline]
    pub fn new(
        half_hight: f32, 
        radius: f32, 
        offset: f32, 
        mass: f32
    ) -> Self {
        Self{
            character_controller: KinematicCharacterController{
                custom_mass: Some(mass),
                offset: CharacterLength::Absolute(offset),
                up: Vec3::Y,
                slide: true,
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Relative(0.3),
                    min_width: CharacterLength::Relative(0.5),
                    include_dynamic_bodies: false,
                }),
                max_slope_climb_angle: 45.0f32.to_radians(),
                min_slope_slide_angle: 30.0f32.to_radians(),
                apply_impulse_to_dynamic_bodies: true,
                snap_to_ground: None,
                ..default()
            },
            rigidbody: RigidBody::KinematicPositionBased,
            capsule: Collider::capsule_y(half_hight, radius)
        }
    }

    #[inline]
    pub fn replica(half_hight: f32, radius: f32) -> impl Bundle {
        (
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(half_hight, radius),
        )
    }
}

#[derive(Bundle)]
pub struct DynamicRigidBodyBundle {
    pub rigidbody: RigidBody,
    pub velocity: Velocity,
    pub mass: AdditionalMassProperties,
    pub sleeping: Sleeping
}

impl DynamicRigidBodyBundle {
    #[inline]
    pub fn new(mass: f32, linear_velocity: Vec3, angular_velocity: Vec3) -> Self {
        Self{
            rigidbody: RigidBody::Dynamic,
            velocity: Velocity{
                linvel: linear_velocity,
                angvel: angular_velocity
            },
            mass: AdditionalMassProperties::Mass(mass),
            sleeping: Sleeping::default()
        }
    }

    #[inline]
    pub fn replica() -> RigidBody {
        RigidBody::KinematicPositionBased
    }
}

