use std::collections::{VecDeque, vec_deque::Iter};
use bevy::prelude::*;
use bevy_replicon::{
    prelude::*, 
    core::replicon_tick::RepliconTick,
    client::ServerEntityTicks
};
use anyhow::bail;
use serde::{Serialize, de::DeserializeOwned};
use super::{network_entity::NetworkEntity, network_event::NetworkEvent};

pub struct EventSnapshot<E: NetworkEvent> {
    event: E,
    tick: u32
}

impl<E: NetworkEvent> EventSnapshot<E> {
    #[inline]
    pub fn new(event: E, tick: u32) -> Self {
        Self{
            event,
            tick
        }
    }

    #[inline]
    pub fn event(&self) -> &E {
        &self.event
    }

    #[inline]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.event.index()
    }

    #[inline]
    pub fn timestamp(&self) -> f64 {
        self.event.timestamp()
    } 
}

#[derive(Component)]
pub struct EventSnapshots<E: NetworkEvent> {
    deq: VecDeque<EventSnapshot<E>>,
    max_size: usize,
    frontier_index: usize
}

impl<E: NetworkEvent> EventSnapshots<E> {
    #[inline]
    pub fn with_capacity(max_size: usize) -> Self {
        Self { 
            deq: VecDeque::with_capacity(max_size), 
            frontier_index: 0,
            max_size 
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.deq.len()
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&EventSnapshot<E>> {
        self.deq.back()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&EventSnapshot<E>> {
        self.deq.get(index)
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, EventSnapshot<E>> {
        self.deq.iter()
    }

    #[inline]
    pub fn sort_with_index(&mut self) {
        self.deq.make_contiguous().sort_by_key(|s| s.index());
    }

    #[inline]
    pub fn pop_front(&mut self) {
        self.deq.pop_front();
    }

    #[inline]
    pub fn insert(&mut self, event: E, tick: u32)
    -> anyhow::Result<()> {
        if self.max_size == 0 {
            bail!("zero size deque");
        }

        if let Some(latest_snap) = self.latest_snapshot() {
            if tick < latest_snap.tick {
                bail!("tick: {tick} is older than latest snapshot: {}", latest_snap.tick);
            }
        }

        if event.index() < self.frontier_index {
            bail!(
                "event index: {} is older than frontier: {}", 
                event.index(), self.frontier_index
            );
        } 

        if self.deq.len() >= self.max_size {
            self.deq.pop_front();
        }

        self.deq.push_back(EventSnapshot::new(event, tick));
        Ok(())
    }

    #[inline]
    pub fn frontier(&mut self) -> Iter<'_, EventSnapshot<E>> {
        if let Some(begin) = self.deq.iter()
        .position(|e| e.index() >= self.frontier_index) {
            // buffer is not empty here
            self.frontier_index = self.deq.back().unwrap().index() + 1;
            self.deq.range(begin..)
        } else {
            self.deq.range(0..0)
        }
    }
}

fn server_populate_client_event_snapshots<E>(
    mut events: EventReader<FromClient<E>>,
    mut query: Query<(&NetworkEntity, &mut EventSnapshots<E>)>,
    replicon_tick: Res<RepliconTick>
) 
where E: NetworkEvent + Serialize + DeserializeOwned + Clone {
    let tick = replicon_tick.get();
    for FromClient { client_id, event } in events.read() {
        for (net_e, mut snaps) in query.iter_mut() {
            if net_e.client_id() != *client_id {
                continue;
            }

            match snaps.insert(event.clone(), tick) {
                Ok(()) => debug!(
                    "inserted event snapshot at tick: {} len: {}", 
                    tick, snaps.len()
                ),
                Err(e) => warn!("discarding: {e}")
            }
        }
    }
}

fn client_populate_client_event_snapshots<E>(
    mut query: Query<(Entity, &mut EventSnapshots<E>)>,
    mut events: EventReader<E>,
    server_ticks: Res<ServerEntityTicks>
)
where E: NetworkEvent + Serialize + DeserializeOwned + Clone {
    for event in events.read() {
        for (e, mut snaps) in query.iter_mut() {
            let tick = server_ticks.get(&e)
            .expect("server tick should be mapped").get();
            
            match snaps.insert(event.clone(), tick) {
                Ok(()) => debug!(
                    "inserted event snapshot at tick: {} len: {}", 
                    tick, snaps.len()
                ),
                Err(e) => warn!("discarding: {e}")
            }
        }
    }
}

pub trait NetworkEventSnapshotAppExt {
    fn use_client_event_snapshots<E>(
        &mut self,
        channel: impl Into<RepliconChannel>
    ) -> &mut Self
    where E: NetworkEvent + Serialize + DeserializeOwned + Clone;
}

impl NetworkEventSnapshotAppExt for App{
    fn use_client_event_snapshots<E>(
        &mut self,
        channel: impl Into<RepliconChannel>
    ) -> &mut Self
    where E: NetworkEvent + Serialize + DeserializeOwned + Clone {
        if self.world.contains_resource::<RepliconServer>() {
            self.add_systems(PreUpdate, 
                server_populate_client_event_snapshots::<E>
                .after(ServerSet::Receive)    
            );
        } else if self.world.contains_resource::<RepliconClient>() {
            self.add_systems(PostUpdate, 
                client_populate_client_event_snapshots::<E>
            );
        } else {
            panic!("could not find replicon server nor client");
        }
        self.add_client_event::<E>(channel)
    }
}
