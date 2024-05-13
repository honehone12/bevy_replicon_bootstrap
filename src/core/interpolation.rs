use bevy::{
    prelude::*,
    utils::SystemTime
};
use super::component_snapshot::ComponentSnapshots;

pub trait LinearInterpolatable: Component {
    fn linear_interpolate(&self, rhs: &Self, per: f32) -> Self;
}

pub fn linear_interpolate<C>(
    current: &C,
    snaps: &ComponentSnapshots<C>,
    network_tick_delta: f64
) -> anyhow::Result<C>
where C: Component + LinearInterpolatable + Clone {
    let len = snaps.len();
    if len < 2 {
        return Ok(current.clone());
    }

    // deque is longer than 2
    let mut iter = snaps.iter().rev();
    let latest = iter.next().unwrap();
    
    let now = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)?
    .as_secs_f64();
    let elapsed = now - latest.timestamp();

    // network tick delta time = 100%
    // elapsed = ?%
    // into 0.0 ~ 1.0
    let per = (elapsed / network_tick_delta).clamp(0.0, 1.0) as f32;
    let second = iter.next().unwrap();

    let interpolated = second
    .component()
    .linear_interpolate(latest.component(), per);
    Ok(interpolated)
}
