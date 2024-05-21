use std::marker::PhantomData;
use bevy::prelude::*;

#[derive(Component, Default)]
pub struct Importance<T>(PhantomData<T>);
