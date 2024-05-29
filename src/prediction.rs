use std::marker::PhantomData;
use bevy::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{NetworkRotation, NetworkTranslation};

#[derive(Resource)]
pub struct PredictionErrorThreshold {
    pub translation_threshold: f32,
    pub rotation_threshold: f32,
    pub error_count_threshold: u32
}

#[derive(Component, Default)]
pub struct PredioctionError<C>
where C: Component + Serialize + DeserializeOwned {
    error_count: u32,
    phantom: PhantomData<C>
}

impl<C: Component + Serialize + DeserializeOwned> PredioctionError<C> {
    #[inline]
    pub fn get_count(&self) -> u32 {
        self.error_count
    }

    #[inline]
    pub fn increment_count(&mut self) {
        self.error_count = self.error_count.saturating_add(1);
    }

    #[inline]
    pub fn reset_count(&mut self) {
        self.error_count = 0;
    }
}

#[derive(Event, Serialize, Deserialize, Default)]
pub struct ForceReplicate<C>(PhantomData<C>)
where C: Component + Serialize + DeserializeOwned;

#[derive(Event, Serialize, Deserialize, Default)]
pub struct ForceReplicateTransform<T, R>(
    PhantomData<T>,
    PhantomData<R>
)
where T: NetworkTranslation, R: NetworkRotation;
