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
    pub rigidbody: RigidBody,
    pub character_controller: KinematicCharacterController,
    pub controller_output: KinematicCharacterControllerOutput,
    pub velocity: Velocity
}

impl Default for CharacterControllerBundle {
    fn default() -> Self {
        Self { 
            rigidbody: RigidBody::KinematicVelocityBased, 
            character_controller: default(), 
            controller_output: default(), 
            velocity: default() 
        }
    }
}