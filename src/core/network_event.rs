use serde::{Serialize, de::DeserializeOwned};
use bevy::prelude::*;

pub trait NetworkEvent
: Event + Serialize + DeserializeOwned + Clone {
    fn index(&self) -> usize;
    fn tick(&self) -> u32;
    fn validate(&self) -> anyhow::Result<()>;
}
