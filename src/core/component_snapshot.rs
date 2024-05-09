use std::{collections::{VecDeque, vec_deque::Iter}, time::SystemTime};
use bevy::prelude::*;
use bevy_replicon::{
    prelude::*,
    client::ServerEntityTicks,
    core::replicon_tick::RepliconTick
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use anyhow::bail;

#[derive(Deserialize, Serialize)]
pub struct ComponentSnapshot<C: Component> {
    tick: u32,
    timestamp: f64,
    component: C,
}

impl<C: Component> ComponentSnapshot<C> {
    #[inline]
    pub fn new(component: C, timestamp: f64, tick: u32) -> Self {
        Self{ 
            tick,
            timestamp, 
            component 
        }
    }

    #[inline]
    pub fn tick(&self) -> u32 {
        self.tick
    }

    #[inline]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    #[inline]
    pub fn component(&self) -> &C {
        &self.component
    }
}

#[derive(Component, Deserialize, Serialize)]
pub struct ComponentSnapshots<C: Component> {
    deq: VecDeque<ComponentSnapshot<C>>,
    max_size: usize
}

impl<C: Component> ComponentSnapshots<C> {
    #[inline]
    pub fn with_capacity(max_size: usize) -> Self {
        Self{
            deq: VecDeque::with_capacity(max_size),
            max_size
        }
    }

    #[inline]
    pub fn insert(&mut self, component: C, tick: u32) 
    -> anyhow::Result<()> {
        if self.max_size == 0 {
            bail!("zero size deque");
        }

        if let Some(latest_snap) = self.latest_snapshot() {
            if tick < latest_snap.tick {
                bail!("tick: {tick} is older than lated snapshot: {}", latest_snap.tick);
            }
        }

        if self.deq.len() >= self.max_size {
            self.deq.pop_front();
        }

        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let timestamp = now.as_secs_f64();
        self.deq.push_back(ComponentSnapshot::new(component, timestamp, tick));
        Ok(())
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&ComponentSnapshot<C>> {
        self.deq.back()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&ComponentSnapshot<C>> {
        self.deq.get(index)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.deq.len()
    }

    #[inline]
    pub fn sort_with_tick(&mut self) {
        self.deq.make_contiguous().sort_by_key(|s| s.tick);
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_, ComponentSnapshot<C>> {
        self.deq.iter()
    }
}

fn server_populate_component_snapshots<C: Component + Clone>(
    mut query: Query<
        (&C, &mut ComponentSnapshots<C>), 
        Changed<C>
    >,
    replicon_tick: Res<RepliconTick>
) { 
    let tick = replicon_tick.get();
    for (c, mut snaps) in query.iter_mut() {
        match snaps.insert(c.clone(), tick) {
            Ok(()) => debug!(
                "inserted to component snapshot at tick: {} len: {}", 
                tick, snaps.len()
            ),
            Err(e) => warn!("discarding: {e}") 
        }
    }
}

fn client_populate_component_snapshots<C: Component + Clone>(
    mut query: Query<
        (Entity , &C, &mut ComponentSnapshots<C>), 
        Changed<C>
    >,
    server_tick: Res<ServerEntityTicks>,
) {
    for (e, c, mut snaps) in query.iter_mut() {
        let tick = server_tick.get(&e)
        .expect("server tick sholed be mapped").get();
        
        match snaps.insert(c.clone(), tick) {
            Ok(()) => debug!(
                "inserted to component buffer at tick: {} now len: {}",
                tick, snaps.len()
            ),
            Err(e) => warn!("discarding: {e}")
        }
    }
}

pub trait ComponentSnapshotAppExt {
    fn use_component_snapshots<C>(&mut self) -> &mut Self
    where C: Component + Serialize + DeserializeOwned + Clone;
}

impl ComponentSnapshotAppExt for App {
    fn use_component_snapshots<C>(&mut self) -> &mut Self
    where C: Component + Serialize + DeserializeOwned + Clone {
        if self.world.contains_resource::<RepliconServer>() {
            self.add_systems(PostUpdate,
                server_populate_component_snapshots::<C>
            )
        } else if self.world.contains_resource::<RepliconClient>() {
            self.add_systems(PreUpdate, 
                client_populate_component_snapshots::<C>
                .after(ClientSet::Receive)
            )
        } else {
            panic!("could not found replicon server nor client");
        }
    }
}
