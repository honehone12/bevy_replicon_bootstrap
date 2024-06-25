use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::prelude::*;

fn update_transform_server_system<T, R, E, P>(
    mut query: Query<(
        &NetworkEntity,
        &mut T, &mut ComponentSnapshots<T>, &mut PredioctionError<T>,
        &mut R, &mut ComponentSnapshots<R>, &mut PredioctionError<R>,
        &mut EventSnapshots<E>
    )>,
    params: Res<P>,
    update: Res<NetworkTransformUpdate<T, R, E, P>>,
    axis: Res<TransformAxis>,
    fixed_time: Res<Time<Fixed>>,
    thresholds: Res<PredictionErrorThreshold>,
    mut force_replication: EventWriter<ToClients<ForceReplicateTransform<T, R>>>
)
where 
T: NetworkTranslation,
R: NetworkRotation, 
E: NetworkMovement,
P: Resource {
    for (
        net_e,
        mut net_trans, mut trans_snaps, mut trans_pred_err,
        mut net_rot, mut rot_snaps, mut rot_pred_err, 
        mut movements
    ) in query.iter_mut() {
        trans_snaps.cache();
        rot_snaps.cache();
        
        if movements.frontier_len() == 0 {
            continue;
        }

        movements.sort_frontier_by_index();
        
        // frontier is not empty
        let first = movements.frontier_front()
        .unwrap()
        .event();
        let first_timestamp = first.timestamp();

        let trans_cache = trans_snaps.cache_ref();
        let trans_idx = match trans_cache.iter()
        .rposition(|s| 
            s.timestamp() <= first_timestamp
        ) {
            Some(idx) => idx,
            None => {
                if cfg!(debug_assertions) {
                    panic!("could not find snapshot for timestamp: {first_timestamp}");
                } else {
                    error!(
                        "could not find snapshot for timestamp: {}, skipping update",
                        first_timestamp
                    );
                    continue;
                }
            }
        };

        let rot_cache = rot_snaps.cache_ref();
        let rot_idx = match rot_cache.iter()
        .rposition(|s| 
            s.timestamp() <= first_timestamp
        ) {
            Some(idx) => idx,
            None => {
                if cfg!(debug_assertions) {
                    panic!("could not find snapshot for timestamp: {first_timestamp}");
                } else {
                    error!(
                        "could not find snapshot for timestamp: {}, skipping update",
                        first_timestamp
                    );
                    continue;
                }
            }
        };

        // get by found index
        let server_translation = trans_cache.get(trans_idx)
        .unwrap()
        .component()
        .to_vec3(axis.translation);
        let server_rotation = rot_cache.get(rot_idx)
        .unwrap()
        .component()
        .to_quat(axis.rotation);
        let client_translation = first.current_translation(axis.translation);
        let client_rotation = first.current_rotation(axis.rotation);
        if client_rotation.length_squared() == 0.0 {
            warn!("client rotation length is zero, skipping update");
            continue;
        }

        let trans_err = server_translation.distance_squared(client_translation);
        if trans_err > thresholds.translation_threshold {
            trans_pred_err.increment_count();
            if trans_pred_err.get_count() > thresholds.error_count_threshold {
                warn!("sending translation force replication for: {:?}", net_e.client_id());
                force_replication.send(ToClients{ 
                    mode: SendMode::Direct(net_e.client_id()), 
                    event: default()
                });

                trans_pred_err.reset_count();
            }
        } else {
            trans_pred_err.reset_count();
        }

        let rot_err = server_rotation.normalize()
        .angle_between(client_rotation.normalize())
        .to_degrees();
        if rot_err > thresholds.rotation_threshold {
            rot_pred_err.increment_count();
            if rot_pred_err.get_count() > thresholds.error_count_threshold {
                warn!("sending rotation force replication for: {:?}", net_e.client_id());
                force_replication.send(ToClients{
                    mode: SendMode::Direct(net_e.client_id()),
                    event: default()
                });

                rot_pred_err.reset_count();    
            }
        } else {
            rot_pred_err.reset_count();
        }
        
        let mut translation = net_trans.clone();
        let mut rotation = net_rot.clone();
        for snap in movements.frontier_ref()
        .iter() {
            (update.update())(
                &mut translation,
                &mut rotation, 
                &snap.event(),
                &params, 
                &fixed_time
            );
        }

        movements.cache();

        *net_rot = rotation;
        *net_trans = translation;
    } 
}

fn update_transform_client_system<T, R, E, P>(
    mut query: Query<(
        &mut Transform,
        &mut ComponentSnapshots<T>,
        &mut ComponentSnapshots<R>, 
        &mut EventSnapshots<E>
    ), 
        With<Owning>
    >,
    params: Res<P>,
    update: Res<NetworkTransformUpdate<T, R, E, P>>,
    axis: Res<TransformAxis>,
    fixed_time: Res<Time<Fixed>>
)
where 
T: NetworkTranslation,
R: NetworkRotation, 
E: NetworkMovement, 
P: Resource {
    if let Ok((
        mut transform,
        mut trans_snaps,
        mut rot_snaps, 
        mut movements
    )) = query.get_single_mut() {
        for movement in movements.frontier_ref()
        .iter() {
            let mut translation = T::from_vec3(transform.translation, axis.translation);        
            let mut rotation = R::from_quat(transform.rotation, axis.rotation);
            (update.update())(
                &mut translation,
                &mut rotation,
                movement.event(),
                &params,
                &fixed_time
            );
            transform.rotation = rotation.to_quat(axis.rotation);
            transform.translation = translation.to_vec3(axis.translation);
        }

        movements.cache();
        trans_snaps.cache();
        rot_snaps.cache();
    } 
}

fn apply_network_transform_client_system<T, R>(
    mut query: Query<(
        &mut Transform,
        &T, &mut ComponentSnapshots<T>,
        &R, &mut ComponentSnapshots<R>,
    ), Without<Owning>>,
    axis: Res<TransformAxis>,
    config: Res<InterpolationConfig>
)
where 
T: NetworkTranslation + LinearInterpolatable,
R: NetworkRotation + LinearInterpolatable {
    for (
        mut transform, 
        net_trans, mut trans_snaps,
        net_rot, mut rot_snaps
    ) in query.iter_mut() {
        const REQUIRED: usize = 2;

        rot_snaps.sort_frontier_by_timestamp();
        let rot = match linear_interpolate_by_time(
            &rot_snaps, 
            config.network_tick_delta
        ) {
            Ok(r_op) => {
                match r_op {
                    Some(r) => r.to_quat(axis.rotation),
                    None => transform.rotation
                }
            }
            Err(e) => {
                if cfg!(debug_assertions) {
                    panic!("error on rotation interpolation: {e}");
                } else {
                    error!("error on rotation interpolation: {e}");
                    net_rot.to_quat(axis.rotation)
                }
            }
        };
        let rot_len = rot_snaps.frontier_len();
        if rot_len > REQUIRED {
            rot_snaps.cache_n(rot_len - REQUIRED);
        }
        transform.rotation = rot;
        
        trans_snaps.sort_frontier_by_timestamp();
        let trans = match linear_interpolate_by_time(
            &trans_snaps, 
            config.network_tick_delta
        ) {
            Ok(t_op) => {
                match t_op {
                    Some(t) => t.to_vec3(axis.translation),
                    None => transform.translation
                }
            }
            Err(e) => {
                if cfg!(debug_assertions) {
                    panic!("error on translation interpolation: {e}");
                } else {
                    error!("error on translation interpolation: {e}");
                    trans_snaps.cache();
                    net_trans.to_vec3(axis.translation)
                }
            }
        };
        let trans_len = trans_snaps.frontier_len();
        if trans_len > REQUIRED {
            trans_snaps.cache_n(trans_len - REQUIRED);
        }
        transform.translation = trans;
    }
}

fn handle_force_replication<T, R>(
    mut query: Query<(
        &mut Transform,
        &T, &R 
    ),
        With<Owning>
    >,
    mut force_replication: EventReader<ForceReplicateTransform<T, R>>,
    axis: Res<TransformAxis>
)
where 
T: NetworkTranslation,
R: NetworkRotation {
    for _ in force_replication.read() {
        if let Ok((
            mut transform, 
            net_translation, 
            net_rotation
        )) = query.get_single_mut() {
            transform.rotation = net_rotation.to_quat(axis.rotation);
            transform.translation = net_translation.to_vec3(axis.translation);
            warn!("force replicated");
        }
    }
}

pub struct NetworkTransformPlugin<T, R, E, P>
where
T: NetworkTranslation,
R: NetworkRotation,
E: NetworkMovement,
P: Resource + Clone {
    pub translation_axis: TranslationAxis,
    pub rotation_axis: RotationAxis, 
    pub update_fn: NetworkTransformUpdateFn<T, R, E, P>,
    pub params: P,
    pub network_tick_delta: f64,
    pub translation_error_threshold: f32,
    pub rotation_error_threshold: f32,
    pub error_count_threshold: u32,
}

impl<T, R, E, P> Plugin for NetworkTransformPlugin<T, R, E, P>
where
T: NetworkTranslation,
R: NetworkRotation,
E: NetworkMovement,
P: Resource + Clone  {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.insert_resource(TransformAxis{
                translation: self.translation_axis,
                rotation: self.rotation_axis
            })
            .insert_resource(NetworkTransformUpdate::new(self.update_fn))
            .insert_resource(self.params.clone())
            .insert_resource(PredictionErrorThreshold{
                translation_threshold: self.translation_error_threshold,
                rotation_threshold: self.rotation_error_threshold,
                error_count_threshold: self.error_count_threshold
            })
            .add_server_event::<ForceReplicateTransform<T, R>>(ChannelKind::Ordered)
            .add_systems(
                FixedUpdate, 
                update_transform_server_system::<T, R, E, P>
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            app.insert_resource(TransformAxis{
                translation: self.translation_axis,
                rotation: self.rotation_axis
            })
            .insert_resource(NetworkTransformUpdate::new(self.update_fn))
            .insert_resource(self.params.clone())
            .insert_resource(InterpolationConfig{
                network_tick_delta: self.network_tick_delta
            })
            .add_server_event::<ForceReplicateTransform<T, R>>(ChannelKind::Ordered)
            .add_systems(PreUpdate, 
                handle_force_replication::<T, R>
                .after(ClientSet::Receive)
            )
            .add_systems(FixedUpdate, (
                update_transform_client_system::<T, R, E, P>,
                apply_network_transform_client_system::<T, R>
            ).chain());
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}