use anyhow::bail;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::prelude::*;

#[derive(Default, Clone, Copy)]
pub enum TranslationAxis {
    #[default]
    Default,
    XY,
    XZ
}

#[derive(Default, Clone, Copy)]
pub enum RotationAxis {
    #[default]
    Default,
    Y,
    Z
}

#[derive(Resource, Default)]
pub struct TransformAxis {
    pub translation: TranslationAxis,
    pub rotation: RotationAxis,
}

pub trait NetworkTranslation
: Component + DistanceCalculatable + LinearInterpolatable
+ Serialize + DeserializeOwned + Clone + Default {
    fn from_vec3(vec: Vec3, axis: TranslationAxis) -> Self;
    fn to_vec3(&self, axis: TranslationAxis) -> Vec3;
}

pub trait NetworkRotation
: Component + LinearInterpolatable 
+ Serialize + DeserializeOwned + Clone + Default {
    fn from_quat(quat: Quat, axis: RotationAxis) -> Self;
    fn to_quat(&self, axis: RotationAxis) -> Quat;
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation2D(pub Vec2);

impl LinearInterpolatable for NetworkTranslation2D {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl DistanceCalculatable for NetworkTranslation2D {
    #[inline]
    fn distance(&self, rhs: &Self) -> f32 {
        self.0.distance_squared(rhs.0)
    }   
}

impl NetworkTranslation for NetworkTranslation2D {
    #[inline]
    fn from_vec3(vec3: Vec3, axis: TranslationAxis) -> Self {
        match axis {
            TranslationAxis::Default 
            | TranslationAxis::XY => Self(Vec2::new(vec3.x, vec3.y)),
            TranslationAxis::XZ => Self(Vec2::new(vec3.x, vec3.z)),
        }
    }
    
    #[inline]
    fn to_vec3(&self, axis: TranslationAxis) -> Vec3 {
        match axis {
            TranslationAxis::Default
            | TranslationAxis::XY => Vec3::new(self.0.x, self.0.y, 0.0),
            TranslationAxis::XZ => Vec3::new(self.0.x, 0.0,  self.0.y),
        }
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation3D(pub Vec3);

impl LinearInterpolatable for NetworkTranslation3D {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl DistanceCalculatable for NetworkTranslation3D {
    #[inline]
    fn distance(&self, rhs: &Self) -> f32 {
        self.0.distance_squared(rhs.0)
    }   
}

impl NetworkTranslation for NetworkTranslation3D {
    #[inline]
    fn from_vec3(vec3: Vec3, _: TranslationAxis) -> Self {
        Self(vec3)
    }
    
    #[inline]
    fn to_vec3(&self, _: TranslationAxis) -> Vec3 {
        self.0
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkAngle(pub f32);

impl LinearInterpolatable for NetworkAngle {
    #[inline]
    fn linear_interpolate(&self, rhs: &Self, t: f32) -> Self {
        let mut delta = (rhs.0 - self.0) % 360.0;
        if delta < 0.0 {
            delta += 360.0;
        }
        
        if delta > 180.0 {
            delta -= 360.0;
        }

        delta *= t;
        Self((self.0 + delta) % 360.0)
    }
}

impl NetworkRotation for NetworkAngle {
    #[inline]
    fn from_quat(quat: Quat, axis: RotationAxis) -> Self {
        match axis {
            RotationAxis::Y => Self(quat.to_euler(EulerRot::YXZ).0.to_degrees()),
            RotationAxis::Z
            | RotationAxis::Default => Self(quat.to_euler(EulerRot::YXZ).2.to_degrees())
        }
    }

    #[inline]
    fn to_quat(&self, axis: RotationAxis) -> Quat {
        match axis {
            RotationAxis::Y => Quat::from_rotation_y(self.0.to_radians()),
            RotationAxis::Z
            | RotationAxis::Default => Quat::from_rotation_z(self.0.to_radians()),
        }
    }
}

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

pub trait NetworkMovement: NetworkEvent {
    fn current_translation(&self, axis: TranslationAxis) -> Vec3;
    fn current_rotation(&self, axis: RotationAxis) -> Quat;
}

#[derive(Event, Serialize, Deserialize, Clone, Default)]
pub struct NetworkMovement2D {
    pub current_translation: Vec2,
    pub current_rotation: f32,
    pub linear_axis: Vec2,
    pub rotation_axis: Vec2,
    pub bits: u16,
    pub index: usize,
    pub timestamp: f64
}

impl NetworkEvent for NetworkMovement2D {
    #[inline]
    fn index(&self) -> usize {
        self.index
    }
    
    #[inline]
    fn timestamp(&self) -> f64 {
        self.timestamp
    }

    #[inline]
    fn validate(&self) -> anyhow::Result<()> {
        if !self.current_translation.is_finite() {
            bail!("failed to validate current translation");
        }
        if !self.current_rotation.is_finite() {
            bail!("failed to validate current rotation");
        }
        if !self.linear_axis.is_finite() {
            bail!("failed to validate linear axis");
        }
        if !self.rotation_axis.is_finite() {
            bail!("failed to validate rotation axis");
        }
        if !self.timestamp.is_finite() {
            bail!("failed to validate timestamp");
        }

        Ok(())
    }
}

impl NetworkMovement for NetworkMovement2D {
    #[inline]
    fn current_translation(&self, axis: TranslationAxis) -> Vec3 {
        match axis {
            TranslationAxis::Default
            | TranslationAxis::XY => Vec3::new(
                self.current_translation.x, 
                self.current_translation.y,
                0.0
            ),
            TranslationAxis::XZ => Vec3::new(
                self.current_translation.x,
                0.0,
                self.current_translation.y
            )
        }
    }

    #[inline]
    fn current_rotation(&self, axis: RotationAxis) -> Quat {
        match axis {
            RotationAxis::Y => Quat::from_rotation_y(self.current_rotation.to_radians()),
            RotationAxis::Z
            | RotationAxis::Default => Quat::from_rotation_z(
                self.current_rotation.to_radians()
            )
        }
    }
}

pub type NetworkTransformUpdateFn<T, R, E, P> 
= fn(&mut T, &mut R, &E, &P, &Time<Fixed>);

#[derive(Resource)]
pub struct NetworkTransformUpdate<T, R, E, P>(
    NetworkTransformUpdateFn<T, R, E, P>
)
where 
T: NetworkTranslation, 
R: NetworkRotation, 
E: NetworkMovement, 
P: Resource;

impl<T, R, E, P> NetworkTransformUpdate<T, R, E, P>
where 
T: NetworkTranslation, 
R: NetworkRotation,
E: NetworkMovement, 
P: Resource {
    #[inline]
    pub fn new(update_fn: NetworkTransformUpdateFn<T, R, E, P>) 
    -> Self {
        Self(update_fn)
    }

    #[inline]
    pub fn update(&self) -> NetworkTransformUpdateFn<T, R, E, P> {
        self.0
    }
}

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
            .insert_resource(NetworkTransformUpdate(self.update_fn))
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
            .insert_resource(NetworkTransformUpdate(self.update_fn))
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
