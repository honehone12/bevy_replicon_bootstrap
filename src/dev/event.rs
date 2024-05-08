use bevy::prelude::*;
use crate::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct NetworkMovement2D {
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

#[derive(Event, Serialize, Deserialize, Clone)]
pub struct NetworkFire {
    pub index: usize,
    pub timestamp: f64
}

impl NetworkEvent for NetworkFire {
    fn index(&self) -> usize {
        self.index
    }

    fn timestamp(&self) -> f64 {
        self.timestamp
    }
}
