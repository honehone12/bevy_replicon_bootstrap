pub mod network_transform;
pub mod network_rigidbody;
pub mod network_character_controller;
pub mod network_movement;

pub use network_transform::*;
pub use network_rigidbody::*;
pub use network_character_controller::*;
pub use network_movement::*;

use serde::{Serialize, de::DeserializeOwned};
use bevy::prelude::*;
use crate::prelude::*;

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

pub(crate) fn cache_translation_system<T>(
    mut query: Query<
        &mut ComponentSnapshots<T>, 
        (With<Owning>, Changed<ComponentSnapshots<T>>)
    >
)
where T: NetworkTranslation {
    for mut snaps in query.iter_mut() {
        snaps.cache();
    }
}

pub(crate) fn cache_rotation_system<R>(
    mut query: Query<
        &mut ComponentSnapshots<R>, 
        (With<Owning>, Changed<ComponentSnapshots<R>>)
    >
)
where R: NetworkRotation {
    for mut snaps in query.iter_mut() {
        snaps.cache();
    }
}

pub(crate) fn apply_transform_translation_system<T>(
    mut query: Query<
        (&Transform, &mut T), 
        Changed<Transform>
    >,
    axis: Res<TransformAxis>
)
where T: NetworkTranslation {
    for (transform, mut t) in query.iter_mut() {
        *t = T::from_vec3(transform.translation, axis.translation);
        debug!("updated translation: {}", transform.translation);
    }
}

pub(crate) fn apply_transform_rotation_system<R>(
    mut query: Query<
        (&Transform, &mut R),
        Changed<Transform>
    >,
    axis: Res<TransformAxis>
)
where R: NetworkRotation {
    for (transform, mut r) in query.iter_mut() {
        *r = R::from_quat(transform.rotation, axis.rotation);
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
    for (mut transform, net_trans, mut trans_snaps) in query.iter_mut() {
        const REQUIRED: usize = 2;
        
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
    for (mut transform, net_rot, mut rot_snaps) in query.iter_mut() {
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
    thresholds: Res<PredictionConfig>,
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
        let frontier = frontier_snap.event();

        let trans_cache = trans_snaps.cache_ref();
        let trans_idx = match trans_cache.iter()
        .rposition(|s| 
            s.tick() <= frontier_tick
        ) {
            Some(idx) => idx,
            None => {
                if cfg!(debug_assertions) {
                    panic!(
                        "could not find snapshot for tick: {frontier_tick}"
                    );
                } else {
                    error!(
                        "could not find snapshot for tick: {frontier_tick}, skipping"
                    );
                    continue;
                }
            }
        };

        // get by found index
        let found_snap = trans_cache.get(trans_idx)
        .unwrap();
        let server_translation = found_snap.component()
        .to_vec3(axis.translation);
        let client_translation = frontier.current_translation(axis.translation);
        debug!(
            "found snap at: {} for event's tick: {}",
            found_snap.tick(),
            frontier_tick
        );

        let trans_err = server_translation.distance_squared(client_translation);
        if trans_err > thresholds.translation_threshold {
            trans_pred_err.increment_count();
            if trans_pred_err.get_count() > thresholds.force_replicate_error_count {
                warn!("sending translation force replication for: {:?}", net_e.client_id());
                trans_force_repl.send(ToClients{ 
                    mode: SendMode::Direct(net_e.client_id()), 
                    event: default()
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
    thresholds: Res<PredictionConfig>,
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
        let frontier = frontier_snap.event();

        let rot_cache = rot_snaps.cache_ref();
        let rot_idx = match rot_cache.iter()
        .rposition(|s| 
            s.tick() <= frontier_tick
        ) {
            Some(idx) => idx,
            None => {
                if cfg!(debug_assertions) {
                    panic!(
                        "could not find snapshot for tick: {frontier_tick}"
                    );
                } else {
                    error!(
                        "could not find snapshot for tick: {frontier_tick}, skipping"
                    );
                    continue;
                }
            }
        };

        // get by found index
        let found_snap = rot_cache.get(rot_idx)
        .unwrap();
        let server_rotation = found_snap.component()
        .to_quat(axis.rotation);
        let client_rotation = frontier.current_rotation(axis.rotation);
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
        if rot_err > thresholds.rotation_threshold {
            rot_pred_err.increment_count();
            if rot_pred_err.get_count() > thresholds.force_replicate_error_count {
                warn!("sending rotation force replication for: {:?}", net_e.client_id());
                rot_force_repl.send(ToClients{
                    mode: SendMode::Direct(net_e.client_id()),
                    event: default()
                });

                rot_pred_err.reset_count();    
            }
        } else {
            rot_pred_err.reset_count();
        }
    } 
}

pub(crate) fn handle_correct_translation<T>(
    mut query: Query<(&mut Transform, &T), With<Owning>>,
    mut force_replication: EventReader<ForceReplicateTranslation<T>>,
    axis: Res<TransformAxis>
)
where T: NetworkTranslation {
    for _ in force_replication.read() {
        if let Ok((mut transform, net_translation)) = query.get_single_mut() {
            transform.translation = net_translation.to_vec3(axis.translation);
            warn!("force replicated translation");
        }
    }
}

pub(crate) fn handle_correct_rotation<R>(
    mut query: Query<(&mut Transform, &R), With<Owning>>,
    mut force_replication: EventReader<ForceReplicateRotation<R>>,
    axis: Res<TransformAxis>
)
where R: NetworkRotation {
    for _ in force_replication.read() {
        if let Ok((mut transform, net_rotation)) = query.get_single_mut() {
            transform.rotation = net_rotation.to_quat(axis.rotation);
            warn!("force replicated rotation");
        }
    }
}
