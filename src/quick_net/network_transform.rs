use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;

use super::distance_culling::DistanceCalculatable;

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

impl NetworkTranslation2D {
    #[inline]
    pub fn from_3d(vec3: Vec3) -> Self {
        Self(Vec2::new(vec3.x, vec3.z))
    }
    
    #[inline]
    pub fn to_3d(&self) -> Vec3 {
        Vec3::new(self.0.x, 0.0, self.0.y)
    }
}

#[derive(Component, Serialize, Deserialize, Default, Clone, Copy)]
pub struct NetworkYaw(pub f32);

impl LinearInterpolatable for NetworkYaw {
    fn linear_interpolate(&self, rhs: &Self, t: f32) -> Self {
        Self(self.0.lerp(rhs.0, t))
    }
}

impl NetworkYaw {
    #[inline]
    pub fn from_quat(quat: Quat) -> Self {
        Self(quat.to_euler(EulerRot::YXZ).0)
    }

    #[inline]
    pub fn to_quat(&self) -> Quat {
        Quat::from_rotation_y(self.0.to_radians())
    }
}

#[derive(Bundle)]
pub struct NetworkTranslation2DWithSnapshots {
    pub translation: NetworkTranslation2D,
    pub snaps: ComponentSnapshots<NetworkTranslation2D>,
    pub prediction_error: PredioctionError<NetworkTranslation2D>
}

impl NetworkTranslation2DWithSnapshots {
    #[inline]
    pub fn new(
        init: Vec3, 
        tick: u32,
        max_size: usize
    ) -> anyhow::Result<Self> {
        let mut snaps = ComponentSnapshots::with_capacity(max_size);
        let translation = NetworkTranslation2D::from_3d(init);
        snaps.insert(translation, tick)?;
        
        Ok(Self{ 
            translation, 
            snaps ,
            prediction_error: default()
        })
    }
}

#[derive(Bundle)]
pub struct NetworkYawWithSnapshots {
    pub yaw: NetworkYaw,
    pub snaps: ComponentSnapshots<NetworkYaw>,
}

impl NetworkYawWithSnapshots {
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

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct NetworkMovement2D {
    pub current_translation: Vec2,
    pub axis: Vec2,
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

pub type NetworkTranslationUpdateFn<P> = fn(
    &mut NetworkTranslation2D,
    &NetworkMovement2D,
    &P,
    &Time<Fixed>
);

#[derive(Resource)]
pub struct NetworkTransformUpdateFns<P: Resource> {
    pub translation_update_fn: NetworkTranslationUpdateFn<P>
}

#[derive(Resource)]
pub struct NetworkTransformInterpolationConfig {
    pub network_tick_delta: f64
}

fn update_translation_2d_server_system<P: Resource>(
    mut query: Query<(
        &NetworkEntity,
        &mut NetworkTranslation2D,
        &ComponentSnapshots<NetworkTranslation2D>,
        &mut PredioctionError<NetworkTranslation2D>,
        &mut EventSnapshots<NetworkMovement2D>
    )>,
    params: Res<P>,
    update_fns: Res<NetworkTransformUpdateFns<P>>,
    fixed_time: Res<Time<Fixed>>,
    thresholds: Res<PredictionErrorThresholds>,
    mut force_replication: EventWriter<ToClients<ForceReplicate<NetworkTranslation2D>>>
) {
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
        let first = frontier.next().unwrap().event();
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
        let server_translation = snaps.get(index).unwrap().component();
        let client_translation = first.current_translation;

        let error = server_translation.0.distance_squared(client_translation);
        if error > thresholds.translation_error_threshold {
            prediction_error.error_count += 1;
            
            warn!(
                "translation error is over threashold, now prediction error count: {}", 
                prediction_error.error_count
            );

            if prediction_error.error_count > thresholds.prediction_error_count_threshold {
                warn!(
                    "prediction error count is over threashold"
                );

                force_replication.send(ToClients { 
                    mode: SendMode::Direct(net_e.client_id()), 
                    event: default()
                });

                prediction_error.error_count = 0;
            }
        } else {
            prediction_error.error_count = 0;
        }
        
        let mut translation = net_translation.clone();
        (update_fns.translation_update_fn)(
            &mut translation, 
            first, 
            &params, 
            &fixed_time
        );

        while let Some(snap) = frontier.next() {
            (update_fns.translation_update_fn)(
                &mut translation, 
                snap.event(), 
                &params, 
                &fixed_time
            );
        }

        net_translation.0 = translation.0;
    } 
}

fn update_translation_2d_client_system<P: Resource>(
    mut query: Query<&mut Transform, With<Owning>>,
    mut movements: EventReader<NetworkMovement2D>,
    params: Res<P>,
    update_fns: Res<NetworkTransformUpdateFns<P>>,
    fixed_time: Res<Time<Fixed>>
) {
    for movement in movements.read() {
        if let Ok(mut transform) = query.get_single_mut() {
            let mut translation = NetworkTranslation2D::from_3d(transform.translation);        
            (update_fns.translation_update_fn)(
                &mut translation, 
                movement, 
                &params, 
                &fixed_time
            );    
            transform.translation = translation.to_3d();
        }       
    }
}

fn apply_network_transform_client_system(
    mut query: Query<(
        &mut Transform,
        &NetworkTranslation2D,
        &ComponentSnapshots<NetworkTranslation2D>
    ), Without<Owning>>,
    config: Res<NetworkTransformInterpolationConfig>
) {
    for (mut transform, net_translation, translation_snaps) in query.iter_mut() {
        match linear_interpolate(
            net_translation, 
            translation_snaps, 
            config.network_tick_delta
        ) {
            Ok(t) => {
                transform.translation = t.to_3d();
            }
            Err(e) => {
                if cfg!(debug_assertions) {
                    panic!("error on transform interpolation: {e}");

                } else {
                    error!("error on transform interpolation: {e}");
                    transform.translation = net_translation.to_3d();
                }
            }
        };
    }
}

pub trait NetworkTransformAppExt {
    fn use_network_transform_2d<P: Resource>(
        &mut self,
        translation_update_fn :NetworkTranslationUpdateFn<P>,
        network_tick_delta: f64
    ) -> &mut Self;
}

impl NetworkTransformAppExt for App {
    fn use_network_transform_2d<P: Resource>(
        &mut self,
        translation_update_fn: NetworkTranslationUpdateFn<P>,
        network_tick_delta: f64
    ) -> &mut Self {
        if self.world.contains_resource::<RepliconServer>() {
            self.insert_resource(NetworkTransformUpdateFns{
                translation_update_fn
            })
            .add_systems(FixedUpdate, 
                update_translation_2d_server_system::<P>
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self.insert_resource(NetworkTransformUpdateFns{
                translation_update_fn
            })
            .insert_resource(NetworkTransformInterpolationConfig{
                network_tick_delta
            })
            .add_systems(FixedUpdate, (
                update_translation_2d_client_system::<P>,
                apply_network_transform_client_system
            ).chain())
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
