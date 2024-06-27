use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ServerBootSet {
    UnboxEvent,
    PlayerEntityEvent,
    CorrectReplication,
    Update,
    ApplyLocalChange,
    Cache
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum ClientBootSet {
    UnboxReplication,
    ApplyReplication,
    Update,
    CacheLocalChange
}
