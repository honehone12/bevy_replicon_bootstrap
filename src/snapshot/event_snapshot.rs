use anyhow::bail;
use bevy::{
    utils::SystemTime,
    prelude::*
};
use bevy_replicon::{
    client::confirm_history::ConfirmHistory,
    server::server_tick::ServerTick, 
    prelude::*, 
};
use crate::{
    Owning, 
    core::{NetworkEntity, NetworkEvent}
};

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
    pub fn sent_timestamp(&self) -> f64 {
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
    pub fn with_capacity(cache_size: usize) -> Self {
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
    pub fn frontier_back(&self) -> Option<&EventSnapshot<E>> {
        let len = self.frontier_len();
        if len == 0 {
            return None;
        }

        self.frontier.get(len - 1)
    }

    #[inline]
    pub fn frontier_front(&self) -> Option<&EventSnapshot<E>> {
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
        let frontier_len = self.frontier_len();

        if self.cache_size == 0
        && frontier_len >= 64 && frontier_len % 64 == 0 {
            warn!(
                "frontier len: {}, call cache() to clear fronter",
                frontier_len
            );
        }
        
        if self.cache_size > 0 
        && frontier_len > self.cache_size {
            warn!(
                "frontier len: {} over cache size, call cache() after frontier_ref()",
                frontier_len
            );
        }

        let received_timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs_f64();

        if let Some(frontier_snap) = self.frontier_front() {
            if tick < frontier_snap.tick {
                bail!(
                    "tick: {tick} is older than frontier snapshot: {}", 
                    frontier_snap.tick
                );
            }

            if event.timestamp() <= frontier_snap.sent_timestamp() {
                bail!(
                    "timestamp: {} is older than latest: {}",
                    event.timestamp(),
                    frontier_snap.sent_timestamp(),
                );
            }

            debug_assert!(received_timestamp >= frontier_snap.received_timestamp());
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
    pub fn sort_frontier_by_index(&mut self) {
        if self.frontier_len() == 0 {
            return;
        }

        self.frontier
        .sort_unstable_by_key(|s| s.index());
    }

    pub fn cache(&mut self) {
        let mut frontier_len = self.frontier_len();
        if frontier_len == 0 {
            return;
        } 
        
        if self.cache_size == 0 {
            self.frontier.clear();
            return;
        } 

        if frontier_len > self.cache_size {
            let uncacheable = frontier_len - self.cache_size;
            self.frontier.drain(..uncacheable);
            frontier_len = self.frontier_len();
        }
        
        if self.cache_len() + frontier_len > self.cache_size {
            self.cache.drain(..frontier_len);
        }

        // frontier is not empty
        let latest_idx = self.frontier_back()
        .unwrap()
        .index(); 
        let drain = self.frontier.drain(..);
        self.cache.append(&mut drain.collect());
        self.frontier_index = latest_idx + 1;

        debug_assert!(self.frontier_len() == 0);
        debug_assert!(self.cache_len() <= self.cache_size);
    }
}

pub(super) fn server_populate_client_event_snapshots<E: NetworkEvent>(
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
                Ok(()) => debug!(
                    "inserted event snapshot: frontier index: {} frontier len: {}, cache len: {}",
                    snaps.frontier_index(),
                    snaps.frontier_len(), 
                    snaps.cache_len()
                ),
                Err(e) => warn!("discarding: {e}")
            }
        }
    }
}

pub(super) fn client_populate_client_event_snapshots<E: NetworkEvent>(
    mut query: Query<(
        &mut EventSnapshots<E>, 
        &ConfirmHistory
    ),
        With<Owning>
    >,
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
                Ok(()) => debug!(
                    "inserted event snapshot: frontier index: {} frontier len: {}, cache len: {}",
                    snaps.frontier_index(),
                    snaps.frontier_len(), 
                    snaps.cache_len()
                ),
                Err(e) => warn!("discarding: {e}")
            }
        }
    }
}
