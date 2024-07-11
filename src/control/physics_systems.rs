use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::prelude::*;

pub(crate) fn apply_rb_linear_velocity_system<L>(
    mut query: Query<(
        &Sleeping,
        &Velocity,
        &NetworkRigidBody,
        &mut L
    ), 
        With<RigidBody>
    >,
    axis: Res<TransformAxis>
)
where L: NetworkLinearVelocity {
    for (sleep, vel, net_rb, mut net_linvel) in query.iter_mut() {
        if matches!(net_rb, NetworkRigidBody::ServerSimulation) {
            continue;
        }

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
        &NetworkRigidBody,
        &mut A
    ), 
        With<RigidBody>
    >,
    axis: Res<TransformAxis>
)
where A: NetworkAngularVelocity {
    for (sleep, vel, net_rb, mut net_angvel) in query.iter_mut() {
        if matches!(net_rb, NetworkRigidBody::ServerSimulation) {
            continue;
        }

        if sleep.sleeping {
            continue;
        }

        *net_angvel = A::from_vec3(vel.angvel, axis.rotation);
    }
}

pub(crate) fn apply_netrb_linear_velocity_system<L>(
    mut query: Query<(
        &mut Velocity,
        &NetworkRigidBody,
        &L
    ), (
        Changed<L>,
        With<RigidBody>
    )>,
    axis: Res<TransformAxis>
)
where L: NetworkLinearVelocity {
    for (mut vel, net_rb, net_linvel) in query.iter_mut() {
        if matches!(net_rb, NetworkRigidBody::ServerSimulation) {
            continue;
        }

        vel.linvel = net_linvel.to_vec3(axis.translation);
    }
}

pub(crate) fn apply_netrb_angular_velocity_system<A>(
    mut query: Query<(
        &mut Velocity,
        &NetworkRigidBody,
        &A
    ), (
        Changed<A>,
        With<RigidBody>
    )>,
    axis: Res<TransformAxis>
)
where A: NetworkAngularVelocity {
    for (mut vel, net_rb, net_angvel) in query.iter_mut() {
        if matches!(net_rb, NetworkRigidBody::ServerSimulation) {
            continue;
        }

        vel.angvel = net_angvel.to_vec3(axis.rotation);    
    }
}
