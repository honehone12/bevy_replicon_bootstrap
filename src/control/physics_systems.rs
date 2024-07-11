use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::prelude::*;

pub(crate) fn apply_rb_linear_velocity_system<L>(
    mut query: Query<(
        &Sleeping,
        &Velocity,
        &mut L
    ), 
        With<RigidBody>
    >,
    axis: Res<TransformAxis>
)
where L: NetworkLinearVelocity {
    for (sleep, vel, mut net_linvel) in query.iter_mut() {
        if sleep.sleeping {
            continue;
        }

        *net_linvel = L::from_vec3(vel.linvel, axis.translation);
    }
}

pub(crate) fn apply_rb_angular_velocity_system<A>(
    mut query: Query<(
        &Sleeping,
        &Velocity,
        &mut A
    ), 
        With<RigidBody>
    >,
    axis: Res<TransformAxis>
)
where A: NetworkAngularVelocity {
    for (sleep, vel, mut net_angvel) in query.iter_mut() {
        if sleep.sleeping {
            continue;
        }

        *net_angvel = A::from_vec3(vel.angvel, axis.rotation);
    }
}

pub(crate) fn apply_netrb_linear_velocity_system<L>(
    mut query: Query<
        (&mut Velocity, &L), 
        (Changed<L>, With<RigidBody>)
    >,
    axis: Res<TransformAxis>
)
where L: NetworkLinearVelocity {
    for (mut vel, net_linvel) in query.iter_mut() {
        vel.linvel = net_linvel.to_vec3(axis.translation);
    }
}

pub(crate) fn apply_netrb_angular_velocity_system<A>(
    mut query: Query<
        (&mut Velocity, &A), 
        (Changed<A>, With<RigidBody>)
    >,
    axis: Res<TransformAxis>
)
where A: NetworkAngularVelocity {
    for (mut vel, net_angvel) in query.iter_mut() {
        vel.angvel = net_angvel.to_vec3(axis.rotation);    
    }
}
