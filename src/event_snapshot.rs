use std::marker::PhantomData;
use bevy::{
    utils::SystemTime,
    prelude::*
};
use bevy_replicon::{
    client::confirm_history::ConfirmHistory,
    server::server_tick::ServerTick, 
    prelude::*, 
};
use anyhow::bail;
use super::{network_entity::NetworkEntity, network_event::NetworkEvent};

pub struct EventSnapshot<E: NetworkEvent> {
    event: E,
    received_timestamp: f64,
    tick: u32
}

impl<E: NetworkEvent> EventSnapshot<E> {
    #[inline]
    pub fn new(event: E, received_timestamp: f64, tick: u32) -> Self {
        Self{
            event,
            received_timestamp,
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
    pub fn received_timestamp(&self) -> f64 {
        self.received_timestamp
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
    frontier: Vec<EventSnapshot<E>>,
    frontier_index: usize,
    cache: Vec<EventSnapshot<E>>,
    cache_size: usize,
}

impl<E: NetworkEvent> EventSnapshots<E> {
    #[inline]
    pub fn with_cache_capacity(cache_size: usize) -> Self {
        Self { 
            frontier: Vec::new(),
            frontier_index: 0,
            cache: Vec::with_capacity(cache_size),
            cache_size
        }
    }

    #[inline]
    pub fn frontier_len(&self) -> usize {
        self.frontier.len()
    }

    #[inline]
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&EventSnapshot<E>> {
        if self.frontier_len() == 0 {
            return None;
        }
        self.frontier.get(self.frontier_len() - 1)
    }

    #[inline]
    pub fn frontier_snapshot(&self) -> Option<&EventSnapshot<E>> {
        self.frontier.get(0)
    }

    #[inline]
    pub fn frontier_index(&self) -> usize {
        self.frontier_index
    }

    #[inline]
    pub fn frontier_ref(&self) -> &Vec<EventSnapshot<E>> {
        &self.frontier
    }

    #[inline]
    pub fn cache_ref(&self) -> &Vec<EventSnapshot<E>> {
        &self.cache
    }

    pub fn insert(&mut self, event: E, tick: u32)
    -> anyhow::Result<()> {
        if self.cache_size > 0 
        && self.frontier_len() > self.cache_size {
            warn!("are you missing to call cache() ?");
        }

        let received_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs_f64();

        if let Some(latest_snap) = self.latest_snapshot() {
            if tick < latest_snap.tick {
                bail!(
                    "tick: {tick} is older than latest snapshot: {}", 
                    latest_snap.tick
                );
            }

            if event.timestamp() <= latest_snap.timestamp() {
                bail!(
                    "timestamp: {} is older than latest: {}",
                    event.timestamp(),
                    latest_snap.timestamp(),
                );
            }

            debug_assert!(received_timestamp >= latest_snap.received_timestamp());
        }

        if event.index() < self.frontier_index {
            bail!(
                "event index: {} is older than frontier: {}", 
                event.index(), self.frontier_index
            );
        } 

        self.frontier.push(EventSnapshot::new(
            event, 
            received_timestamp, 
            tick
        ));
        Ok(())
    }

    #[inline]
    pub fn sort_by_index(&mut self) {
        if self.frontier_len() == 0 {
            return;
        }

        self.frontier
        .sort_by_key(|s| s.index());
    }

    pub fn cache(&mut self) {
        let mut frontier_size = self.frontier_len();
        if frontier_size == 0 {
            return;
        } 
        
        if self.cache_size == 0 {
            self.frontier.clear();
            return;
        } 

        if frontier_size > self.cache_size {
            let uncacheable = self.cache_size - frontier_size;
            _ = self.frontier.drain(..uncacheable);
            frontier_size = self.frontier_len();
        }
        
        if self.cache_len() + frontier_size > self.cache_size {
            _ = self.cache.drain(..frontier_size);
        }

        // frontier is not empty
        let latest_idx = self.latest_snapshot()
        .unwrap()
        .index(); 
        let drain = self.frontier.drain(..);
        self.cache.append(&mut drain.collect());
        self.frontier_index = latest_idx + 1;

        debug_assert!(self.frontier_len() == 0);
        debug_assert!(self.cache_len() <= self.cache_size);
    }
}

fn server_populate_client_event_snapshots<E: NetworkEvent>(
    mut events: EventReader<FromClient<E>>,
    mut query: Query<(&NetworkEntity, &mut EventSnapshots<E>)>,
    server_tick: Res<ServerTick>
) {
    let tick = server_tick.get();
    for FromClient { client_id, event } in events.read() {
        if let Err(e) = event.validate() {
            warn!("discarding: {e}");
            continue;
        }

        for (net_e, mut snaps) in query.iter_mut() {
            if net_e.client_id() != *client_id {
                continue;
            }

            match snaps.insert(event.clone(), tick) {
                Ok(()) => (),
                Err(e) => warn!("discarding: {e}")
            }
        }
    }
}

fn client_populate_client_event_snapshots<E: NetworkEvent>(
    mut query: Query<(&mut EventSnapshots<E>, &ConfirmHistory)>,
    mut events: EventReader<E>,
) {
    for event in events.read() {
        if let Err(e) = event.validate() {
            warn!("discarding: {e}");
            continue;
        }

        for (mut snaps, confirmed_tick) in query.iter_mut() {
            let tick = confirmed_tick.last_tick().get();
            match snaps.insert(event.clone(), tick) {
                Ok(()) => (),
                Err(e) => warn!("discarding: {e}")
            }
        }
    }
}

pub struct NetworkEventSnapshotPlugin<E: NetworkEvent>{
    pub channel_kind: ChannelKind,
    pub phantom: PhantomData<E>
}

impl<E: NetworkEvent> Plugin for NetworkEventSnapshotPlugin<E> {
    fn build(&self, app: &mut App) {
        if app.world.contains_resource::<RepliconServer>() {
            app.add_client_event::<E>(self.channel_kind)
            .add_systems(PreUpdate, 
                server_populate_client_event_snapshots::<E>
                .after(ServerSet::Receive)    
            );
        } else if app.world.contains_resource::<RepliconClient>() {
            app.add_client_event::<E>(self.channel_kind)
            .add_systems(PostUpdate, 
                client_populate_client_event_snapshots::<E>
            );
        } else {
            panic!("could not find replicon server nor client");
        }
    }
}
