use serde::{Deserialize, Serialize};
use anyhow::bail;
use bevy::{
    prelude::*,
    utils::SystemTime
};
use bevy_replicon::{
    client::confirm_history, 
    server::server_tick::ServerTick 
};
use crate::core::LinearInterpolatable;

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
    frontier: Vec<ComponentSnapshot<C>>,
    cache: Vec<ComponentSnapshot<C>>,
    cache_size: usize
}

impl<C: Component> ComponentSnapshots<C> {
    #[inline]
    pub fn with_capacity(cache_size: usize) -> Self {
        Self{
            frontier: Vec::new(),
            cache: Vec::with_capacity(cache_size),
            cache_size
        }
    }

    #[inline]
    pub fn with_init(init: C, tick: u32, cache_size: usize) 
    -> anyhow::Result::<Self> {
        let mut snaps = Self::with_capacity(cache_size);
        match snaps.insert(init, tick) {
            Ok(()) => Ok(snaps),
            Err(e) => Err(e) 
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
    pub fn frontier_back(&self) -> Option<&ComponentSnapshot<C>> {
        let len = self.frontier_len();
        if len == 0 {
            return None;
        }

        self.frontier.get(len - 1)
    }

    #[inline]
    pub fn frontier_front(&self) -> Option<&ComponentSnapshot<C>> {
        self.frontier.get(0)
    }

    #[inline]
    pub fn frontier_ref(&self) -> &Vec<ComponentSnapshot<C>> {
        &self.frontier
    }

    #[inline]
    pub fn cache_ref(&self) -> &Vec<ComponentSnapshot<C>> {
        &self.cache
    }

    pub fn insert(&mut self, component: C, tick: u32) 
    -> anyhow::Result<()> {
        let frontier_len = self.frontier_len();
        
        if self.cache_size == 0
        && frontier_len >= 64 && frontier_len % 64 == 0 {
            warn!(
                "fronteier len: {} call cache() for clear frontier",
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

        let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs_f64();

        if let Some(frontier_snap) = self.frontier_front() {
            if tick < frontier_snap.tick {
                bail!(
                    "tick: {tick} is older than frontier snapshot: {}", 
                    frontier_snap.tick
                );
            }

            debug_assert!(timestamp >= frontier_snap.timestamp());
        }
        
        self.frontier.push(ComponentSnapshot::new(
            component, 
            timestamp, 
            tick
        ));
        Ok(())
    }

    #[inline]
    pub fn sort_frontier_by_timestamp(&mut self) {
        if self.frontier_len() == 0 {
            return;
        }

        self.frontier
        .sort_unstable_by(|l, r| 
            // timestamp is always stamped in insert()
            // that returns error on bad result
            l.timestamp()
            .partial_cmp(&r.timestamp())
            .expect("timestamp is Nan")
        );
    }

    pub fn cache_n(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let frontier_len = self.frontier_len();
        if frontier_len < n {
            return;
        }

        if self.cache_size == 0 {
            self.frontier.drain(..n);
            return;
        }

        if n > self.cache_size {
            self.cache.clear();
            let uncacheable = n - self.cache_size;
            self.frontier.drain(..uncacheable);
            let drain = self.frontier.drain(..self.cache_size);
            self.cache.append(&mut drain.collect());

            debug_assert!(self.frontier_len() == frontier_len - n);
            debug_assert!(self.cache_len() == self.cache_size);
            return;
        }

        if self.cache_len() + n > self.cache_size {
            self.cache.drain(..n);
        }

        let drain = self.frontier.drain(..n);
        self.cache.append(&mut drain.collect());

        debug_assert!(self.frontier_len() == frontier_len - n);
        debug_assert!(self.cache_len() <= self.cache_size);
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
            self.cache.clear();
            frontier_len = self.frontier_len();
        }

        if self.cache_len() + frontier_len > self.cache_size {
            self.cache.drain(..frontier_len);
        }

        let drain = self.frontier.drain(..);
        self.cache.append(&mut drain.collect());

        debug_assert!(self.frontier_len() == 0);
        debug_assert!(self.cache_len() <= self.cache_size);
    }
}

pub fn linear_interpolate_by_time<C: LinearInterpolatable>(
    snaps: &ComponentSnapshots<C>,
    network_tick_delta: f64
) -> anyhow::Result<Option<C>> {
    if network_tick_delta <= 0.0 {
        bail!("invalid network tick delta");
    }

    if snaps.frontier_len() < 2 {
        return Ok(None)
    }

    let mut iter = snaps.frontier_ref()
    .iter()
    .rev();
    // frontier is longer than or equal 2
    let latest = iter.next().unwrap();
    
    let now = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)?
    .as_secs_f64();
    let elapsed = now - latest.timestamp();
    if elapsed < 0.0 {
        bail!("latest snapshot is future");
    }
    
    // network tick delta time = 100%
    // elapsed = ?%
    // into 0.0 ~ 1.0

    // become 1.0 or over
      // if we don't return here this can be extrapolation.
      // but we are not sure should do or not 
    if elapsed >= network_tick_delta {
        return Ok(Some(
            latest.component()
            .clone()
        ));
    }
    
    let per = (elapsed / network_tick_delta) as f32;
    let second = iter.next().unwrap();

    let interpolated = second
    .component()
    .linear_interpolate(latest.component(), per);
    Ok(Some(interpolated))
}

pub(super) fn server_populate_component_snapshots<C: Component + Clone>(
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
                "inserted component snapshot: frontier len: {}, cache len: {}",
                snaps.frontier_len(),
                snaps.cache_len()
            ),
            Err(e) => warn!("discarding: {e}") 
        }
    }
}

pub(super) fn client_populate_component_snapshots<C: Component + Clone>(
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
                "inserted component snapshot frontier len: {}, cache len: {}",
                snaps.frontier_len(),
                snaps.cache_len()
            ),
            Err(e) => warn!("discarding: {e}")
        }
    }
}
