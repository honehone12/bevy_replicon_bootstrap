use bevy::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

pub trait NetworkEvent
: Event + Serialize + DeserializeOwned + Clone {
    fn index(&self) -> usize;
    fn timestamp(&self) -> f64;
    fn validate(&self) -> anyhow::Result<()>;
}
