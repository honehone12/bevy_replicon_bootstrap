use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

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
    }
}