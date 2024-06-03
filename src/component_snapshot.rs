use std::{collections::{vec_deque::Iter, VecDeque}, marker::PhantomData};
use bevy::{
    prelude::*,
    utils::SystemTime
};
use bevy_replicon::{
    
    client::confirm_history, prelude::*, server::server_tick::ServerTick 
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

    pub fn insert(&mut self, component: C, tick: u32) 
    -> anyhow::Result<()> {
        if self.max_size == 0 {
            bail!("zero size deque");
        }

        let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs_f64();

        if let Some(latest_snap) = self.latest_snapshot() {
            if tick < latest_snap.tick {
                bail!("tick: {tick} is older than lated snapshot: {}", latest_snap.tick);
            }

            debug_assert!(timestamp >= latest_snap.timestamp());
        }

        if self.deq.len() >= self.max_size {
            self.deq.pop_front();
        }

        
        self.deq.push_back(ComponentSnapshot::new(
            component, 
            timestamp, 
            tick
        ));
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
        self.deq.make_contiguous()
        .sort_by_key(|s| s.tick);
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
    server_tick: Res<ServerTick>
) { 
    let tick = server_tick.get();
    for (c, mut snaps) in query.iter_mut() {
        match snaps.insert(c.clone(), tick) {
            Ok(()) => debug!(
                "inserted to component snapshot at tick: {} len: {}", 
                tick, snaps.len()
            ),
            Err(e) => warn!("discarding: {e}") 
        }
        break;
    }
}

fn client_populate_component_snapshots<C: Component + Clone>(
    mut query: Query<( 
        &C, 
        &mut ComponentSnapshots<C>,
        &confirm_history::ConfirmHistory
    ), 
        Changed<C>
    >,
) {
    for (c, mut snaps, confirmed_tick) in query.iter_mut() {
        let tick = confirmed_tick.last_tick().get();
        match snaps.insert(c.clone(), tick) {
            Ok(()) => debug!(
                "inserted to component buffer at tick: {} now len: {}",
                tick, snaps.len()
            ),
            Err(e) => warn!("discarding: {e}")
        }
        break;
    }
}

pub struct ComponentSnapshotPlugin<C>(pub PhantomData<C>)
where C: Component + Serialize + DeserializeOwned + Clone;

impl<C> Plugin for ComponentSnapshotPlugin<C>
where C: Component + Serialize + DeserializeOwned + Clone {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.replicate::<C>()
            .add_systems(PostUpdate,
                server_populate_component_snapshots::<C>
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            app.replicate::<C>()
            .add_systems(PreUpdate, 
                client_populate_component_snapshots::<C>
                .after(ClientSet::Receive)
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
