use std::marker::PhantomData;
use bevy::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Component, Default)]
pub struct PredioctionError<C>
where C: Component + Serialize + DeserializeOwned {
    pub error_count: u32,
    phantom: PhantomData<C>
}

#[derive(Event, Serialize, Deserialize, Default)]
pub struct ForceReplicate<C: Component + Serialize + DeserializeOwned>(pub PhantomData<C>);