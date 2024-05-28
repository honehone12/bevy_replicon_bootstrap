use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::prelude::*;

#[derive(Resource)]
pub enum TranslationAxis {
    XYZ,
    XY,
    XZ
}

impl TranslationAxis {
    #[inline]
    pub fn pack(&self, vec: &Vec3) -> Vec3 {
        match self {
            TranslationAxis::XYZ => *vec,
            TranslationAxis::XY => Vec3::new(vec.x, vec.y, 0.0),
            TranslationAxis::XZ => Vec3::new(vec.x, vec.z, 0.0),
        }
    }
    
    #[inline]
    pub fn unpack(&self, vec: &Vec3) -> Vec3 {
        match self {
            TranslationAxis::XYZ => *vec,
            TranslationAxis::XY => Vec3::new(vec.x, vec.y, 0.0),
            TranslationAxis::XZ => Vec3::new(vec.x, 0.0, vec.y),
        }
    }
}

pub trait NetworkTranslation: Component {
    fn from_vec3(vec: Vec3) -> Self;
    fn to_vec3(&self) -> Vec3;
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation2D(pub Vec2);

impl LinearInterpolatable for NetworkTranslation2D {
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl DistanceCalculatable for NetworkTranslation2D {
    fn distance(&self, rhs: &Self) -> f32 {
        self.0.distance_squared(rhs.0)
    }   
}

impl NetworkTranslation for NetworkTranslation2D {
    fn from_vec3(vec3: Vec3) -> Self {
        Self(Vec2::new(vec3.x, vec3.y))
    }
    
    fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.0.x, self.0.y, 0.0)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkTranslation3D(pub Vec3);

impl LinearInterpolatable for NetworkTranslation3D {
    fn linear_interpolate(&self, rhs: &Self, s: f32) -> Self {
        Self(self.0.lerp(rhs.0, s))
    }
}

impl DistanceCalculatable for NetworkTranslation3D {
    fn distance(&self, rhs: &Self) -> f32 {
        self.0.distance_squared(rhs.0)
    }   
}

impl NetworkTranslation for NetworkTranslation3D {
    fn from_vec3(vec3: Vec3) -> Self {
        Self(vec3)
    }
    
    fn to_vec3(&self) -> Vec3 {
        self.0
    }
}

pub trait NetworkRotation: Component {
    fn from_quat(quat: Quat) -> Self;
    fn to_quat(&self) -> Quat;
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkAngle(pub f32);

impl LinearInterpolatable for NetworkAngle {
    fn linear_interpolate(&self, rhs: &Self, t: f32) -> Self {
        Self(self.0.lerp(rhs.0, t))
    }
}

impl NetworkRotation for NetworkAngle {
    fn from_quat(quat: Quat) -> Self {
        Self(quat.to_euler(EulerRot::YXZ).0)
    }

    fn to_quat(&self) -> Quat {
        Quat::from_rotation_y(self.0.to_radians())
    }
}

#[derive(Bundle)]
pub struct NetworkTranslation2DBundle {
    pub translation: NetworkTranslation2D,
    pub snaps: ComponentSnapshots<NetworkTranslation2D>,
    pub prediction_error: PredioctionError<NetworkTranslation2D>
}

impl NetworkTranslation2DBundle {
    #[inline]
    pub fn new(
        init: Vec3, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let translation = NetworkTranslation2D::from_vec3(init);
        snaps.insert(translation, tick)?;
        
        Ok(Self{ 
            translation, 
            snaps ,
            prediction_error: default()
        })
    }
}

#[derive(Bundle)]
pub struct NetworkAngleBundle {
    pub yaw: NetworkAngle,
    pub snaps: ComponentSnapshots<NetworkAngle>,
}

impl NetworkAngleBundle {
    #[inline]
    pub fn new(
        init: Quat, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let yaw = NetworkAngle::from_quat(init);
        snaps.insert(yaw, tick)?;
        
        Ok(Self{ 
            yaw, 
            snaps 
        })
    }
}

pub trait NetworkMovement: Event {
    fn current_translation(&self) -> Vec3;
}

#[derive(Event, Serialize, Deserialize, Clone, Default)]
pub struct NetworkMovement2D {
    pub current_translation: Vec2,
    pub linear_axis: Vec2,
    pub rotation_axis: Vec2,
    pub bits: u32,
    pub index: usize,
    pub timestamp: f64
}

impl NetworkEvent for NetworkMovement2D {
    fn index(&self) -> usize {
        self.index
    }
    
    fn timestamp(&self) -> f64 {
        self.timestamp
    }
}

impl NetworkMovement for NetworkMovement2D {
    fn current_translation(&self) -> Vec3 {
        Vec3::new( 
            self.current_translation.x, 
            self.current_translation.y, 
            0.0
        )
    }
}

pub type NetworkTranslationUpdateFn<T, E, P> = fn(
    &mut T,
    &E,
    &P,
    &Time<Fixed>
);

pub type NetworkRotationUpdateFn<R, E, P> = fn(
    &mut R,
    &E,
    &P,
    &Time<Fixed>
);

#[derive(Resource)]
pub struct NetworkTransformUpdateRegistry<T, R, E, P>
where 
T: NetworkTranslation, 
R: NetworkRotation, 
E: NetworkMovement, 
P: Resource {
    translation_update_fn: NetworkTranslationUpdateFn<T, E, P>,
    rotation_update_fn: NetworkRotationUpdateFn<R, E, P>
}

impl<T, R, E, P> NetworkTransformUpdateRegistry<T, R, E, P>
where 
T: NetworkTranslation, 
R: NetworkRotation,
E: NetworkMovement, 
P: Resource {
    #[inline]
    pub fn new(
        translation_update_fn: NetworkTranslationUpdateFn<T, E, P>,
        rotation_update_fn: NetworkRotationUpdateFn<R, E, P>
    ) -> Self {
        Self{
            translation_update_fn,
            rotation_update_fn
        }
    }

    #[inline]
    pub fn update_translation(&self) -> NetworkTranslationUpdateFn<T, E, P> {
        self.translation_update_fn
    }

    #[inline]
    pub fn update_rotation(&self) -> NetworkRotationUpdateFn<R, E, P> {
        self.rotation_update_fn
    }
}

#[derive(Resource)]
pub struct NetworkTransformInterpolationConfig {
    pub network_tick_delta: f64
}

fn update_transform_server_system<T, R, E, P>(
    mut query: Query<(
        &NetworkEntity,
        &mut T, &ComponentSnapshots<T>, &mut PredioctionError<T>,
        &mut R, &ComponentSnapshots<R>, &mut PredioctionError<R>,
        &mut EventSnapshots<E>
    )>,
    params: Res<P>,
    registry: Res<NetworkTransformUpdateRegistry<T, R, E, P>>,
    fixed_time: Res<Time<Fixed>>,
    thresholds: Res<PredictionErrorThresholdConfig>,
    mut force_replication: EventWriter<ToClients<ForceReplicateTransform<T, R>>>
)
where 
T: NetworkTranslation + Serialize + DeserializeOwned + Clone + Default,
R: NetworkRotation + Serialize + DeserializeOwned + Clone + Default, 
E: NetworkEvent + NetworkMovement,
P: Resource {
    for (
        net_e,
        mut net_trans, trans_snaps, mut trans_pred_err,
        mut net_rot, rot_snaps, mut rot_pred_err, 
        mut movements
    ) in query.iter_mut() {  
        movements.sort_with_index();
        let mut frontier = movements.frontier();
        if frontier.len() == 0 {
            continue;
        }

        // frontier is not empty
        let first = frontier.next()
        .unwrap()
        .event();
        let index = match trans_snaps.iter().rposition(
            |s| s.timestamp() <= first.timestamp()
        ) {
            Some(idx) => idx,
            None => {
                if cfg!(debug_assertions) {
                    panic!(
                        "could not find timestamp smaller than: {}, insert one at initialization", 
                        first.timestamp()
                    );
                } else {
                    warn!(
                        "could not find timestamp smaller than: {}, this will cause fuge jump", 
                        first.timestamp()
                    );
                    continue;
                }
            }
        };

        // get by found index
        let server_translation = trans_snaps.get(index)
        .unwrap()
        .component()
        .to_vec3();
        let client_translation = first.current_translation();

        let error = server_translation.distance_squared(client_translation);
        if error > thresholds.translation_error_threshold {
            trans_pred_err.increment_count();
            
            let error_count = trans_pred_err.get_count();
            warn!(
                "translation error is over threashold, now prediction error count: {}", 
                error_count
            );

            if error_count > thresholds.prediction_error_count_threshold {
                warn!(
                    "prediction error count is over threashold"
                );

                force_replication.send(ToClients { 
                    mode: SendMode::Direct(net_e.client_id()), 
                    event: default()
                });

                trans_pred_err.reset_count();
            }
        } else {
            trans_pred_err.reset_count();
        }
        
        let mut translation = net_trans.clone();
        (registry.update_translation())(
            &mut translation, 
            &first,
            &params, 
            &fixed_time
        );

        while let Some(snap) = frontier.next() {
            let e = snap.event();
            (registry.update_translation())(
                &mut translation, 
                &e,
                &params, 
                &fixed_time
            );
        }

        *net_trans = translation;
    } 
}

fn update_transform_client_system<T, R, E, P>(
    mut query: Query<&mut Transform, With<Owning>>,
    mut movements: EventReader<E>,
    params: Res<P>,
    registry: Res<NetworkTransformUpdateRegistry<T, R, E, P>>,
    axis: Res<TranslationAxis>,
    fixed_time: Res<Time<Fixed>>
)
where 
T: NetworkTranslation,
R: NetworkRotation, 
E: NetworkEvent + NetworkMovement, 
P: Resource {
    for movement in movements.read() {
        if let Ok(mut transform) = query.get_single_mut() {
            let mut translation = T::from_vec3(axis.pack(&transform.translation));        
            (registry.update_translation())(
                &mut translation, 
                &movement,
                &params, 
                &fixed_time
            );    
            transform.translation = axis.unpack(&translation.to_vec3());
        }       
    }
}

fn apply_network_transform_client_system<T, R>(
    mut query: Query<(
        &mut Transform,
        &T,
        &ComponentSnapshots<T>
    ), Without<Owning>>,
    axis: Res<TranslationAxis>,
    config: Res<NetworkTransformInterpolationConfig>
)
where 
T: NetworkTranslation + LinearInterpolatable + Clone,
R: NetworkRotation + LinearInterpolatable + Clone {
    for (mut transform, net_translation, translation_snaps) in query.iter_mut() {
        match linear_interpolate(
            net_translation, 
            translation_snaps, 
            config.network_tick_delta
        ) {
            Ok(t) => {
                transform.translation = axis.unpack(&t.to_vec3());
            }
            Err(e) => {
                if cfg!(debug_assertions) {
                    panic!("error on transform interpolation: {e}");

                } else {
                    error!("error on transform interpolation: {e}");
                    transform.translation = axis.unpack(&net_translation.to_vec3());
                }
            }
        };
    }
}

fn handle_force_replication<T, R>(
    mut query: Query<(
        &mut Transform,
        &T 
    ),
        With<Owning>
    >,
    mut force_replication: EventReader<ForceReplicateTransform<T, R>>,
    axis: Res<TranslationAxis>
)
where 
T: NetworkTranslation + Serialize + DeserializeOwned,
R: NetworkRotation + Serialize + DeserializeOwned {
    for _ in force_replication.read() {
        if let Ok((mut transform, net_translation)) = query.get_single_mut() {
            warn!(
                "force replication: before: {}, after: {}",
                transform.translation,
                axis.unpack(&net_translation.to_vec3())
            );
            transform.translation = axis.unpack(&net_translation.to_vec3());
        }
    }
}

pub trait NetworkTransformAppExt {
    fn use_network_transform<T, R, E, P>(
        &mut self,
        axis: TranslationAxis,
        transform_update_fns: NetworkTransformUpdateRegistry<T, R, E, P>,
        params: P,
        interpolation_config: NetworkTransformInterpolationConfig,
        prediction_config: PredictionErrorThresholdConfig
    ) -> &mut Self
    where
    T: NetworkTranslation + LinearInterpolatable + Serialize + DeserializeOwned + Clone + Default,
    R: NetworkRotation + LinearInterpolatable + Serialize + DeserializeOwned + Clone + Default,
    E: NetworkMovement + NetworkEvent + Serialize + DeserializeOwned,
    P: Resource;
}

impl NetworkTransformAppExt for App {
    fn use_network_transform<T, R, E, P>(
        &mut self,
        axis: TranslationAxis,
        transform_update_fns: NetworkTransformUpdateRegistry<T, R, E, P>,
        params: P,
        interpolation_config: NetworkTransformInterpolationConfig,
        prediction_config: PredictionErrorThresholdConfig
    ) -> &mut Self
    where
    T: NetworkTranslation + LinearInterpolatable +Serialize + DeserializeOwned + Clone + Default,
    R: NetworkRotation + LinearInterpolatable + Serialize + DeserializeOwned + Clone + Default,
    E: NetworkMovement + NetworkEvent + Serialize + DeserializeOwned,
    P: Resource {
        if self.world.contains_resource::<RepliconServer>() {
            self.insert_resource(transform_update_fns)
            .insert_resource(params)
            .insert_resource(prediction_config)
            .add_server_event::<ForceReplicateTransform<T, R>>(ChannelKind::Ordered)
            .add_systems(
                FixedUpdate, 
                update_transform_server_system::<T, R, E, P>
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self.insert_resource(axis)
            .insert_resource(transform_update_fns)
            .insert_resource(params)
            .insert_resource(interpolation_config)
            .add_server_event::<ForceReplicateTransform<T, R>>(ChannelKind::Ordered)
            .add_systems(PreUpdate, 
                handle_force_replication::<T, R>
                .after(ClientSet::Receive)
            )
            .add_systems(FixedUpdate, (
                update_transform_client_system::<T, R, E, P>,
                apply_network_transform_client_system::<T, R>
            ).chain())
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
