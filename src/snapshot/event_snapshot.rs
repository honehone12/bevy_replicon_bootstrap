use anyhow::bail;
use bevy::{
    utils::SystemTime,
    prelude::*
};
use bevy_replicon::prelude::*;
use crate::{
    Owning, 
    core::{NetworkEntity, NetworkEvent}
};

#[derive(Clone)]
pub struct EventSnapshot<E: NetworkEvent> {
    event: E,
    timestamp: f64
}

impl<E: NetworkEvent> EventSnapshot<E> {
    #[inline]
    pub fn new(event: E, timestamp: f64) -> Self {
        Self{
            event,
            timestamp
        }
    }

    #[inline]
    pub fn event(&self) -> &E {
        &self.event
    }

    #[inline]
    pub fn sent_tick(&self) -> u32 {
        self.event.tick()
    }

    #[inline]
    pub fn received_timestamp(&self) -> f64 {
        self.timestamp
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.event.index()
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

    #[inline]
    pub(crate) fn insert_unchecked(&mut self, snap: EventSnapshot<E>) {
        self.frontier.push(snap);
    }

    pub fn insert(&mut self, event: E)
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
            if event.tick() < frontier_snap.sent_tick() {
                bail!(
                    "tick: {} is older than frontier snapshot: {}", 
                    event.tick(),
                    frontier_snap.sent_tick()
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
    mut query: Query<(&NetworkEntity, &mut EventSnapshots<E>)>,
    mut events: EventReader<FromClient<E>>
) {
    for FromClient { client_id, event } in events.read() {
        if let Err(e) = event.validate() {
            warn!("validation fail: {e}");
            continue;
        }

        for (net_e, mut snaps) in query.iter_mut() {
            if net_e.client_id() != *client_id {
                continue;
            }

            match snaps.insert(event.clone()) {
                Ok(()) => trace!(
                    "inserted event snapshot: frontier index: {} frontier len: {}, cache len: {}",
                    snaps.frontier_index(),
                    snaps.frontier_len(), 
                    snaps.cache_len()
                ),
                Err(e) => warn!("discarding event snapshot: {e}")
            }
        }
    }
}

pub(super) fn client_populate_client_event_snapshots<E: NetworkEvent>(
    mut query: Query<&mut EventSnapshots<E>, With<Owning>>,
    mut events: EventReader<E>,
) {
    for event in events.read() {
        if let Err(e) = event.validate() {
            warn!("validation fail: {e}");
            continue;
        }

        for mut snaps in query.iter_mut() {
            match snaps.insert(event.clone()) {
                Ok(()) => trace!(
                    "inserted event snapshot: frontier index: {} frontier len: {}, cache len: {}",
                    snaps.frontier_index(),
                    snaps.frontier_len(), 
                    snaps.cache_len()
                ),
                Err(e) => warn!("discarding event snapshot: {e}")
            }
        }
    }
}
