use std::marker::PhantomData;
use bevy::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Resource)]
pub struct PredictionErrorThresholds {
    pub translation_error_threshold: f32,
    pub prediction_error_count_threshold: u32
}


#[derive(Component, Default)]
pub struct PredioctionError<C>
where C: Component + Serialize + DeserializeOwned {
    pub error_count: u32,
    phantom: PhantomData<C>
}

#[derive(Event, Serialize, Deserialize, Default)]
pub struct ForceReplicate<C: Component + Serialize + DeserializeOwned>(pub PhantomData<C>);
