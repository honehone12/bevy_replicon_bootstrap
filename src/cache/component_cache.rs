use anyhow::bail;
use bevy::{
    prelude::*,
    utils::SystemTime
};
use bevy_replicon::{
    client::confirm_history, 
    server::server_tick::ServerTick 
};

#[derive(Clone)]
pub struct ComponentSnapshot<C: Component + Clone> {
    tick: u32,
    timestamp: f64,
    component: C,
}

impl<C: Component + Clone> ComponentSnapshot<C> {
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

#[derive(Component)]
pub struct ComponentCache<C: Component + Clone> {
    frontier: Vec<ComponentSnapshot<C>>,
    cache: Vec<ComponentSnapshot<C>>,
    cache_size: usize
}

impl<C: Component + Clone> ComponentCache<C> {
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
        let mut cache = Self::with_capacity(cache_size);
        match cache.insert(init, tick) {
            Ok(()) => Ok(cache),
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
    pub fn frontier_back_pair(&self) 
    -> Option<(&ComponentSnapshot<C>, &ComponentSnapshot<C>)> {
        let len = self.frontier_len(); 
        if len < 2 {
            return None;
        }

        // frontier is longer than or equal 2
        Some((
            self.frontier.get(len - 1)
            .unwrap(), 
            self.frontier.get(len - 2)
            .unwrap()
        ))
    }

    #[inline]
    pub fn elapsed(&self) -> anyhow::Result<f64> {
        if self.frontier_len() == 0 {
            bail!("frontier is empty");
        }
        
        // frontier is not empty
        let back = self.frontier_back()
        .unwrap();

        let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs_f64();
        let elapsed = now - back.timestamp();
        if elapsed < 0.0 {
            bail!("back is future");
        }
        
        Ok(elapsed)
    }

    #[inline]
    pub fn elapsed_per_network_tick(&self, network_tick_delta: f64)
    -> anyhow::Result<f32> {
        if network_tick_delta == 0.0 {
            bail!("invalid network tick delta");
        }
        
        let elapsed = self.elapsed()?;
        let per = (elapsed / network_tick_delta) as f32;
        Ok(per)
    }

    #[inline]
    pub fn find_at_tick(&self, tick: u32) -> Option<&ComponentSnapshot<C>> {
        self.cache.iter().rfind(|s| s.tick <= tick)
    }

    #[inline]
    pub fn frontier_front(&self) -> Option<&ComponentSnapshot<C>> {
        self.frontier.get(0)
    }

    #[inline]
    pub fn latest_snapshot(&self) -> Option<&ComponentSnapshot<C>> {
        let frontier_len = self.frontier.len();
        if frontier_len != 0 {
            return self.frontier.get(frontier_len - 1);
        }

        let cache_len = self.cache.len();
        if cache_len != 0 {
            return self.cache.get(cache_len - 1);
        }

        None
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

pub(super) fn server_populate_component_cache<C>(
    mut query: Query<
        (&C, &mut ComponentCache<C>), 
        Changed<C>
    >,
    server_tick: Res<ServerTick>
)
where C: Component + Clone { 
    let tick = server_tick.get();
    for (c, mut cache) in query.iter_mut() {
        match cache.insert(c.clone(), tick) {
            Ok(()) => trace!(
                "inserted component snapshot: frontier len: {}, cache len: {}",
                cache.frontier_len(),
                cache.cache_len()
            ),
            Err(e) => warn!("discarding component snapshot: {e}") 
        }
    }
}

pub(super) fn client_populate_component_cache<C>(
    mut query: Query<( 
        &C, 
        &mut ComponentCache<C>,
        &confirm_history::ConfirmHistory
    ), 
        Changed<C>
    >,
)
where C: Component + Clone {
    for (c, mut cache, confirmed_tick) in query.iter_mut() {
        // this as latest replication should be latest tick for this client
        // because this is changed at this tick
        let tick = confirmed_tick.last_tick().get();
        match cache.insert(c.clone(), tick) {
            Ok(()) => trace!(
                "inserted component snapshot frontier len: {}, cache len: {}",
                cache.frontier_len(),
                cache.cache_len()
            ),
            Err(e) => warn!("discarding component snapshot: {e}")
        }
    }
}
