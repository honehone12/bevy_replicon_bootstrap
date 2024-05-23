use bevy::prelude::*;

pub trait NetworkEvent: Event {
    fn index(&self) -> usize;
    fn timestamp(&self) -> f64;
}
