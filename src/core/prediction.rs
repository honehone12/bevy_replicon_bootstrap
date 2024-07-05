use std::marker::PhantomData;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use crate::prelude::*;

#[derive(Resource, Clone)]
pub struct PredictionConfig {
    pub translation_threshold: f32,
    pub rotation_threshold: f32,
    pub force_replicate_error_count: u32
}

impl PredictionConfig {
    #[inline]
    pub fn translation_threshold_sq(&self) -> f32 {
        self.translation_threshold * self.translation_threshold
    }
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
pub struct ForceReplicateTranslation<T>(PhantomData<T>)
where T: NetworkTranslation;

pub type CorrectTranslation<T> = ToClients<ForceReplicateTranslation<T>>;

#[derive(Event, Serialize, Deserialize, Default)]
pub struct ForceReplicateRotation<R>(PhantomData<R>)
where R: NetworkRotation;

pub type CorrectRotation<R> = ToClients<ForceReplicateRotation<R>>;
