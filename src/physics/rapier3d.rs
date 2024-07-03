use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;

pub const BEFORE_PHYSICS_SET: PhysicsSet = PhysicsSet::SyncBackend;
pub const AFTER_PHYSICS_SET: PhysicsSet = PhysicsSet::Writeback;

pub struct Rapier3DPlugin {
    pub delta_time: f32,
    pub substeps: usize 
}

impl Plugin for Rapier3DPlugin {
    fn build(&self, app: &mut App) {
        let mut config = RapierConfiguration::from_world(&mut app.world);
        config.timestep_mode = TimestepMode::Fixed { 
            dt: self.delta_time, 
            substeps: self.substeps 
        };
        
        app.insert_resource(config)
        .add_plugins(
            RapierPhysicsPlugin::<()>::default()
            .in_fixed_schedule()
        );

        if app.world.contains_resource::<RepliconClient>() {
            app.add_plugins(RapierDebugRenderPlugin::default());
        }
    }
}

#[derive(Bundle)]
pub struct CharacterControllerBundle {
    pub character_controller: KinematicCharacterController,
    pub capsule: Collider,
    pub rb: RigidBody
}

impl CharacterControllerBundle {
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
            capsule: Collider::capsule_y(half_hight, radius),
            rb: RigidBody::KinematicPositionBased
        }
    }

    pub fn replica(half_hight: f32, radius: f32) -> impl Bundle {
        (
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(half_hight, radius),
        )
    }
}
