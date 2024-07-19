use bevy::{
    utils::HashMap,
    prelude::*
};

pub type EEMap<T> = HashMap<(Entity, Entity), T>;

#[derive(Resource, Default)]
pub struct EntityPairMap<T>(EEMap<T>);

impl<T> EntityPairMap<T> {
    #[inline]
    pub fn insert(
        &mut self,
        key_l: Entity, key_r: Entity,
        v: T
    ) -> Option<T> {
        let key = if key_l >= key_r {
            (key_l, key_r)
        } else {
            (key_r, key_l)
        };

        self.0.insert(key, v)
    }

    #[inline]
    pub fn get(
        &self,
        key_l: Entity, key_r: Entity
    ) -> Option<&T> {
        let key = if key_l >= key_r {
            (key_l, key_r)
        } else {
            (key_r, key_l)
        };

        self.0.get(&key)
    }

    #[inline]
    pub fn remove(&mut self, key: Entity) {
        self.0.retain(|k, _| k.0 != key && k.1 != key);
    }
}
