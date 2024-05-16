use std::marker::PhantomData;
use bevy::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Component, Default)]
pub struct PredioctionError<C>
where C: Component + Serialize + DeserializeOwned {
    pub error_count: u32,
    phantom: PhantomData<C>
}
