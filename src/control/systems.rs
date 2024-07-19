use bevy::prelude::*;
use crate::prelude::*;

pub(crate) fn cache_translation_system<T>(
    mut query: Query<
        &mut ComponentSnapshots<T>, 
        (With<Owning>, Changed<ComponentSnapshots<T>>)
    >
)
where T: NetworkTranslation {
    let Ok(mut snaps) = query.get_single_mut() else {
        return;
    };

    snaps.cache();
}

pub(crate) fn cache_rotation_system<R>(
    mut query: Query<
        &mut ComponentSnapshots<R>, 
        (With<Owning>, Changed<ComponentSnapshots<R>>)
    >
)
where R: NetworkRotation {
    let Ok(mut snaps) = query.get_single_mut() else {
        return;
    };

    snaps.cache();
}

pub(crate) fn apply_transform_translation_system<T>(
    mut query: Query<
        (&Transform, &mut T, &mut ComponentSnapshots<T>), 
        Changed<Transform>
    >,
    config: Res<ReplicationConfig>,
    axis: Res<TransformAxis>
)
where T: NetworkTranslation {
    for (transform, mut t, mut snaps) in query.iter_mut() {
        match snaps.latest_snapshot() {
            Some(s) => {
                if s.component()
                .to_vec3(axis.translation)
                .distance_squared(transform.translation) 
                <= config.translation_threshold_sq() {
                    snaps.cache();
                    continue;
                }
            }
            None => warn!("no snapshots found")
        }

        *t = T::from_vec3(transform.translation, axis.translation);
        snaps.cache();
        debug!("updated translation: {}", transform.translation);
    }
}

pub(crate) fn apply_transform_rotation_system<R>(
    mut query: Query<
        (&Transform, &mut R, &mut ComponentSnapshots<R>),
        Changed<Transform>
    >,
    config: Res<ReplicationConfig>,
    axis: Res<TransformAxis>
)
where R: NetworkRotation {
    for (transform, mut r, mut snaps) in query.iter_mut() {
        match snaps.latest_snapshot() {
            Some(s) => {
                if s.component()
                .to_quat(axis.rotation)
                .normalize()
                .angle_between(transform.rotation.normalize())
                .abs()
                <= config.rotation_threashold.to_radians() {
                    snaps.cache();
                    continue;
                }
            }
            None => warn!("no snapshots found")
        }

        *r = R::from_quat(transform.rotation, axis.rotation);
        snaps.cache();
        debug!("updated rotation: {}", transform.rotation);
    } 
}

pub(crate) fn apply_network_translation_system<T>(
    mut query: Query<(
        &mut Transform,
        &T,
        &mut ComponentSnapshots<T>,
    ), 
        Without<Owning>
    >,
    axis: Res<TransformAxis>,
    config: Res<InterpolationConfig>
) 
where T: NetworkTranslation {
    for (mut transform, net_trans, mut snaps) in query.iter_mut() {
        let (back_0, back_1) = match snaps.frontier_back_pair() {
            Some(back) => (back.0.component(), back.1.component()),
            None => {
                transform.translation = net_trans.to_vec3(axis.translation);
                continue;
            }
        };

        match snaps.elapsed_per_network_tick(config.network_tick_delta) {
            Ok(p) => {
                let interpolated = back_1.interpolate(
                    &back_0, 
                    p.min(1.0), 
                    axis.translation
                );
                transform.translation = interpolated;
            }
            Err(e) => {
                error!("error on translation interpolation: {e}");
                transform.translation = net_trans.to_vec3(axis.translation);
            }
        };

        const REQUIRED: usize = 2;
        let frontier_len = snaps.frontier_len();
        if frontier_len > REQUIRED {
            snaps.cache_n(frontier_len - REQUIRED);
        }
    }
}

pub(crate) fn apply_network_rotation_system<R>(
    mut query: Query<(
        &mut Transform,
        &R,
        &mut ComponentSnapshots<R>,
    ), 
        Without<Owning>
    >,
    axis: Res<TransformAxis>,
    config: Res<InterpolationConfig>
)
where R: NetworkRotation {
    for (mut transform, net_rot, mut snaps) in query.iter_mut() {
        let (back_0, back_1) = match snaps.frontier_back_pair() {
            Some(back) => (back.0.component(), back.1.component()),
            None => {
                transform.rotation = net_rot.to_quat(axis.rotation);
                continue;
            }
        };

        match snaps.elapsed_per_network_tick(config.network_tick_delta) {
            Ok(p) => {
                let interpolated = back_1.interpolate(
                    &back_0, 
                    p.min(1.0), 
                    axis.rotation
                );
                transform.rotation = interpolated;
            }
            Err(e) => {
                error!("error on rotation interpolation: {e}");
                transform.rotation = net_rot.to_quat(axis.rotation);
            }
        };

        const REQUIRED: usize = 2;
        let frontier_len = snaps.frontier_len();
        if frontier_len > REQUIRED {
            snaps.cache_n(frontier_len - REQUIRED);
        }
    }
}

pub(crate) fn correct_translation_error_system<T, E>(
    mut query: Query<(
        &NetworkEntity,
        &mut ComponentSnapshots<T>, 
        &mut PredioctionError<T>,
        &mut EventSnapshots<E>
    )>,
    axis: Res<TransformAxis>,
    config: Res<PredictionConfig>,
    mut trans_force_repl: EventWriter<CorrectTranslation<T>>
)
where 
T: NetworkTranslation, 
E: NetworkMovement {
    for (net_e,
        mut trans_snaps, 
        mut trans_pred_err,
        mut movements
    ) in query.iter_mut() {
        trans_snaps.cache();
        
        if movements.frontier_len() == 0 {
            continue;
        }

        movements.sort_frontier_by_index();
        
        // frontier is not empty
        let frontier_snap = movements.frontier_front()
        .unwrap();
        let frontier_tick = frontier_snap.sent_tick();

        let found_snap = match trans_snaps.find_at_tick(frontier_tick) {
            Some(s) => s,
            None => {
                if cfg!(debug_assertions) {
                    panic!("could not find snapshot for tick: {frontier_tick}");
                } else {
                    error!("could not find snapshot for tick, skipping");
                    // should we sent force replication here ??
                    continue;
                }
            }
        };

        let server_translation = found_snap.component()
        .to_vec3(axis.translation);
        let client_translation = frontier_snap.event()
        .current_translation(axis.translation);
        debug!(
            "found snap at: {} for event's tick: {}",
            found_snap.tick(),
            frontier_tick
        );

        let trans_err = server_translation.distance_squared(client_translation);
        if trans_err > config.translation_threshold_sq() {
            trans_pred_err.increment_count();
            if trans_pred_err.get_count() > config.force_replicate_error_count {
                // frontier is not empty
                let last_idx = movements.frontier_back()
                .unwrap()
                .index();

                warn!(
                    "sending translation force replication for: {:?}: index: {}", 
                    net_e.client_id(),
                    last_idx
                );

                trans_force_repl.send(CorrectTranslation { 
                    mode: SendMode::Direct(net_e.client_id()), 
                    event: ForceReplicateTranslation::new(last_idx)
                });

                trans_pred_err.reset_count();
            }
        } else {
            trans_pred_err.reset_count();
        }
    } 
}

pub(crate) fn correct_rotation_error_system<R, E>(
    mut query: Query<(
        &NetworkEntity,
        &mut ComponentSnapshots<R>, 
        &mut PredioctionError<R>,
        &mut EventSnapshots<E>
    )>,
    axis: Res<TransformAxis>,
    config: Res<PredictionConfig>,
    mut rot_force_repl: EventWriter<CorrectRotation<R>>
)
where 
R: NetworkRotation, 
E: NetworkMovement {
    for (
        net_e,
        mut rot_snaps, 
        mut rot_pred_err, 
        mut movements
    ) in query.iter_mut() {
        rot_snaps.cache();
        
        if movements.frontier_len() == 0 {
            continue;
        }

        movements.sort_frontier_by_index();
        
        // frontier is not empty
        let frontier_snap = movements.frontier_front()
        .unwrap();
        let frontier_tick = frontier_snap.sent_tick();

        let found_snap = match rot_snaps.find_at_tick(frontier_tick) {
            Some(s) => s,
            None => {
                if cfg!(debug_assertions) {
                    panic!("could not find snapshot for tick: {frontier_tick}");
                } else {
                    error!("could not find snapshot for tick, skipping");
                    // should we sent force replication here ??
                    continue;
                }
            }
        };

        let server_rotation = found_snap.component()
        .to_quat(axis.rotation);
        let client_rotation = frontier_snap.event()
        .current_rotation(axis.rotation);
        if client_rotation.length_squared() == 0.0 {
            warn!("client rotation length is zero, skipping update");
            continue;
        }
        debug!(
            "found snap at: {} for event's tick: {}",
            found_snap.tick(),
            frontier_tick
        );

        let rot_err = server_rotation.normalize()
        .angle_between(client_rotation.normalize())
        .to_degrees();
        if rot_err > config.rotation_threshold {
            rot_pred_err.increment_count();
            if rot_pred_err.get_count() > config.force_replicate_error_count {
                // frontier is not empty
                let last_idx = movements.frontier_back()
                .unwrap()
                .index();
                
                warn!(
                    "sending rotation force replication for: {:?}: indec: {}", 
                    net_e.client_id(),
                    last_idx
                );

                rot_force_repl.send(ToClients{
                    mode: SendMode::Direct(net_e.client_id()),
                    event: ForceReplicateRotation::new(last_idx)
                });

                rot_pred_err.reset_count();    
            }
        } else {
            rot_pred_err.reset_count();
        }
    } 
}

pub(crate) fn handle_correct_translation<T, E>(
    mut query: Query<(
        &mut Transform, 
        &T,
        &mut EventSnapshots<E>
    ), 
        With<Owning>
    >,
    mut force_replication: EventReader<ForceReplicateTranslation<T>>,
    axis: Res<TransformAxis>
)
where 
T: NetworkTranslation,
E: NetworkMovement {
    let Ok((
        mut transform, 
        net_trans,
        mut movements
    )) = query.get_single_mut() else {
        return;
    };
    
    let mut sort = false;

    for e in force_replication.read() {
        warn!(
            "force replicate translation: index: {}",
            e.last_index
        );

        let next_idx = e.last_index() + 1;
        
        if movements.frontier_len() > 0 {
            // frontier is not empty
            let frontier_next = movements.frontier_front()
            .unwrap()
            .index();
            debug!("frontier next index: {frontier_next}");
            if frontier_next  <= next_idx {
                transform.translation = net_trans.to_vec3(axis.translation);
                continue;
            }
        }
        
        if movements.cache_len() == 0 {
            transform.translation = net_trans.to_vec3(axis.translation);
            continue;
        }

        let mut resend = vec![];
        for m in movements.cache_ref()
        .iter()
        .rev() {
            if m.index() < next_idx {
                break;
            }

            resend.push(m.clone());
        }

        if resend.len() > 0 {
            sort = true;
            debug!("{} events were resent", resend.len());
        }

        for m in resend {
            movements.insert_unchecked(m);
        }

        transform.translation = net_trans.to_vec3(axis.translation);    
    }

    if sort {
        movements.sort_frontier_by_index();
    }
}

pub(crate) fn handle_correct_rotation<R, E>(
    mut query: Query<(
        &mut Transform, 
        &R,
        &mut EventSnapshots<E>
    ), 
        With<Owning>
    >,
    mut force_replication: EventReader<ForceReplicateRotation<R>>,
    axis: Res<TransformAxis>
)
where 
R: NetworkRotation,
E: NetworkMovement {
    let Ok((
        mut transform, 
        net_rot,
        mut movements
    )) = query.get_single_mut() else {
        return;
    };

    let mut sort = false;

    for e in force_replication.read() {
        warn!(
            "force replicate rotation: index: {}",
            e.last_index
        );

        let next_idx = e.last_index() + 1;
        
        if movements.frontier_len() > 0 {
            // frontier is not empty
            let frontier_next = movements.frontier_front()
            .unwrap()
            .index();
            debug!("frontier next index: {frontier_next}");
            if frontier_next  <= next_idx {
                transform.rotation = net_rot.to_quat(axis.rotation);
                continue;
            }
        } 
        
        if movements.cache_len() == 0 {
            transform.rotation = net_rot.to_quat(axis.rotation);
            continue;
        }

        let mut resend = vec![];
        for m in movements.cache_ref()
        .iter()
        .rev() {
            if m.index() < next_idx {
                break;
            }

            resend.push(m.clone());
        }

        if resend.len() > 0 {
            sort = true;
            debug!("{} events were resent", resend.len());
        }

        for m in resend {
            movements.insert_unchecked(m);
        }
        
        transform.rotation = net_rot.to_quat(axis.rotation);
    }

    if sort {
        movements.sort_frontier_by_index();
    }
}
