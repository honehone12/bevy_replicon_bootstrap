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

pub trait NetworkRotation: Component {
    fn from_quat(quat: Quat) -> Self;
    fn to_quat(&self) -> Quat;
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkYaw(pub f32);

impl LinearInterpolatable for NetworkYaw {
    fn linear_interpolate(&self, rhs: &Self, t: f32) -> Self {
        Self(self.0.lerp(rhs.0, t))
    }
}

impl NetworkRotation for NetworkYaw {
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
pub struct NetworkYawBundle {
    pub yaw: NetworkYaw,
    pub snaps: ComponentSnapshots<NetworkYaw>,
}

impl NetworkYawBundle {
    #[inline]
    pub fn new(
        init: Quat, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let yaw = NetworkYaw::from_quat(init);
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
        Vec3 { 
            x: self.current_translation.x, 
            y: self.current_translation.y, 
            z: 0.0 
        }
    }
}

pub type NetworkTranslationUpdateFn<C, E, P> = fn(
    &mut C,
    &E,
    &P,
    &Time<Fixed>
);

#[derive(Resource)]
pub struct NetworkTransformUpdateFns<C, E, P>
where C: NetworkTranslation, E: Event, P: Resource {
    translation_update_fn: NetworkTranslationUpdateFn<C, E, P>
}

impl<C, E, P> NetworkTransformUpdateFns<C, E, P>
where C: NetworkTranslation, E: Event, P: Resource {
    #[inline]
    pub fn new(translation_update_fn: NetworkTranslationUpdateFn<C, E, P>)
    -> Self {
        Self { 
            translation_update_fn 
        }
    }

    #[inline]
    pub fn translation_update_fn(&self) -> NetworkTranslationUpdateFn<C, E, P> {
        self.translation_update_fn
    }
}

#[derive(Resource)]
pub struct NetworkTransformInterpolationConfig {
    pub network_tick_delta: f64
}

fn update_translation_server_system<C, E, P>(
    mut query: Query<(
        &NetworkEntity,
        &mut C,
        &ComponentSnapshots<C>,
        &mut PredioctionError<C>,
        &mut EventSnapshots<E>
    )>,
    params: Res<P>,
    update_fns: Res<NetworkTransformUpdateFns<C, E, P>>,
    fixed_time: Res<Time<Fixed>>,
    thresholds: Res<PredictionErrorThresholdConfig>,
    mut force_replication: EventWriter<ToClients<ForceReplicate<C>>>
)
where 
C: NetworkTranslation + Serialize + DeserializeOwned + Clone + Default, 
E: NetworkEvent + NetworkMovement,
P: Resource {
    for (
        net_e,
        mut net_translation, 
        snaps, 
        mut prediction_error, 
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
        let index = match snaps.iter().rposition(
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
        let server_translation = snaps.get(index)
        .unwrap()
        .component()
        .to_vec3();
        let client_translation = first.current_translation();

        let error = server_translation.distance_squared(client_translation);
        if error > thresholds.translation_error_threshold {
            prediction_error.increment_count();
            
            let error_count = prediction_error.get_count();
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

                prediction_error.reset_count();
            }
        } else {
            prediction_error.reset_count();
        }
        
        let mut translation = net_translation.clone();
        (update_fns.translation_update_fn())(
            &mut translation, 
            &first,
            &params, 
            &fixed_time
        );

        while let Some(snap) = frontier.next() {
            let e = snap.event();
            (update_fns.translation_update_fn())(
                &mut translation, 
                &e,
                &params, 
                &fixed_time
            );
        }

        *net_translation = translation;
    } 
}

fn update_translation_client_system<C, E, P>(
    mut query: Query<&mut Transform, With<Owning>>,
    mut movements: EventReader<E>,
    params: Res<P>,
    update_fns: Res<NetworkTransformUpdateFns<C, E, P>>,
    axis: Res<TranslationAxis>,
    fixed_time: Res<Time<Fixed>>
)
where C: NetworkTranslation, E: NetworkEvent, P: Resource {
    for movement in movements.read() {
        if let Ok(mut transform) = query.get_single_mut() {
            let mut translation = C::from_vec3(axis.pack(&transform.translation));        
            (update_fns.translation_update_fn)(
                &mut translation, 
                &movement,
                &params, 
                &fixed_time
            );    
            transform.translation = axis.unpack(&translation.to_vec3());
        }       
    }
}

fn apply_network_transform_client_system<C>(
    mut query: Query<(
        &mut Transform,
        &C,
        &ComponentSnapshots<C>
    ), Without<Owning>>,
    axis: Res<TranslationAxis>,
    config: Res<NetworkTransformInterpolationConfig>
)
where C: NetworkTranslation + LinearInterpolatable + Clone {
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

fn handle_force_replication<C>(
    mut query: Query<(
        &mut Transform,
        &C 
    ),
        With<Owning>
    >,
    mut force_replication: EventReader<ForceReplicate<C>>,
    axis: Res<TranslationAxis>
)
where C: NetworkTranslation + Serialize + DeserializeOwned {
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
    fn use_network_transform_2d<P: Resource>(
        &mut self,
        axis: TranslationAxis,
        transform_update_fns: NetworkTransformUpdateFns<
            NetworkTranslation2D,
            NetworkMovement2D, 
            P
        >,
        params: P,
        interpolation_config: NetworkTransformInterpolationConfig,
        prediction_config: PredictionErrorThresholdConfig
    ) -> &mut Self;
}

impl NetworkTransformAppExt for App {
    fn use_network_transform_2d<P: Resource>(
        &mut self,
        axis: TranslationAxis,
        transform_update_fns: NetworkTransformUpdateFns<
            NetworkTranslation2D,
            NetworkMovement2D, 
            P
        >,
        params: P,
        interpolation_config: NetworkTransformInterpolationConfig,
        prediction_config: PredictionErrorThresholdConfig
    ) -> &mut Self {
        if self.world.contains_resource::<RepliconServer>() {
            self.insert_resource(transform_update_fns)
            .insert_resource(params)
            .insert_resource(prediction_config)
            .add_server_event::<ForceReplicate<NetworkTranslation2D>>(ChannelKind::Ordered)
            .add_systems(FixedUpdate, 
                update_translation_server_system::<
                    NetworkTranslation2D,
                    NetworkMovement2D, 
                    P
                >
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self.insert_resource(axis)
            .insert_resource(transform_update_fns)
            .insert_resource(params)
            .insert_resource(interpolation_config)
            .add_server_event::<ForceReplicate<NetworkTranslation2D>>(ChannelKind::Ordered)
            .add_systems(PreUpdate, 
                handle_force_replication::<NetworkTranslation2D>
                .after(ClientSet::Receive)
            )
            .add_systems(FixedUpdate, (
                update_translation_client_system::<
                    NetworkTranslation2D,
                    NetworkMovement2D, 
                    P
                >,
                apply_network_transform_client_system::<NetworkTranslation2D>
            ).chain())
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
