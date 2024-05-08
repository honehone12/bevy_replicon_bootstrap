use std::collections::{VecDeque, vec_deque::Iter};
use bevy::prelude::*;
use anyhow::bail;
use super::network_event::NetworkEvent;

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
