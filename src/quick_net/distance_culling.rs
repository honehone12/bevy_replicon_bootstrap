use bevy::{
    ecs::entity::EntityHashMap, 
    prelude::*
};
use bevy_replicon::{
    core::replicon_tick::RepliconTick, 
    prelude::*
};
use crate::prelude::*;

#[derive(Default)]
pub struct Distance;

pub trait DistanceCalculatable {
    fn distance(&self, rhs: &Self) -> f32;
}

#[derive(Default, Clone, Copy)]
pub struct DistanceAt {
    pub tick: u32,
    pub distance: f32
}

#[derive(Resource, Default)]
pub struct DistanceMap(EntityHashMap<EntityHashMap<DistanceAt>>);

impl DistanceMap {
    pub fn insert(
        &mut self, 
        l: Entity, r: Entity, 
        distance_at: DistanceAt
    ) -> Option<DistanceAt> {
        if let Some(l_map) = self.0.get_mut(&l) {
            return l_map.insert(r, distance_at)
        }

        match self.0.get_mut(&r) {
            Some(r_map) => r_map.insert(l, distance_at),
            None => {
                self.0
                .entry(l)
                .or_insert(default())
                .insert(r, distance_at)
            }
        }
    }

    pub fn get(
        &self,
        l: &Entity, r: &Entity
    ) -> Option<&DistanceAt> {
        if let Some(l_map) = self.0.get(l) {
            if let Some(d) = l_map.get(r) {
                return Some(d)
            }
        }

        match self.0.get(r) {
            Some(r_map) => r_map.get(l),
            None => None
        }
    }
}

fn calculate_distance_system<C>(
    changed: Query<
        (Entity, &C), 
        (Or<(Changed<C>, Added<C>)>, With<Importance<Distance>>)
    >,
    query: Query<
        (Entity, &C), 
        With<Importance<Distance>>
    >,
    mut distance_map: ResMut<DistanceMap>,
    replicon_tick: Res<RepliconTick>
)
where C: Component + DistanceCalculatable {
    for (l_e, l_c) in changed.iter() {
        let tick = replicon_tick.get();

        for (r_e, r_c) in query.iter() {
            if l_e == r_e {
                continue;
            }

            if let Some(d) = distance_map.get(&l_e, &r_e) {
                if d.tick == tick {
                    continue;
                }
            }

            let distance = l_c.distance(&r_c);
            let distance_at = DistanceAt{
                tick,
                distance
            };
            
            distance_map.insert(l_e, r_e, distance_at);
            info!(
                "updated distance from: {:?} to: {:?} tick: {} distance: {}",
                l_e, r_e,
                tick, 
                distance
            );
        }
    }
}

pub trait DistanceCullingAppExt {
    fn use_distance_culling<C>(&mut self) -> &mut Self
    where C: Component + DistanceCalculatable;
}

impl DistanceCullingAppExt for App {
    fn use_distance_culling<C>(&mut self) -> &mut Self
    where C: Component + DistanceCalculatable {
        if self.world.contains_resource::<RepliconServer>() {
            self.insert_resource(DistanceMap::default())
            .add_systems(PostUpdate, 
                calculate_distance_system::<C>
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self
        } else {
            panic!("could not find replicon server nor client");
        }        
    }
}
