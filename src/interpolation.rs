use bevy::{
    prelude::*,
    utils::SystemTime
};
use super::component_snapshot::ComponentSnapshots;

#[derive(Resource)]
pub struct InterpolationConfig {
    pub network_tick_delta: f64
}

pub trait LinearInterpolatable: Component {
    fn linear_interpolate(&self, rhs: &Self, per: f32) -> Self;
}

pub(crate) fn linear_interpolate<C>(
    snaps: &ComponentSnapshots<C>,
    network_tick_delta: f64
) -> anyhow::Result<Option<C>>
where C: Component + LinearInterpolatable + Clone {
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
    
    // network tick delta time = 100%
    // elapsed = ?%
    // into 0.0 ~ 1.0

    // become 1.0
    if elapsed >= network_tick_delta {
        return Ok(Some(
            latest.component()
            .clone()
        ));
    }
    
    let per = (elapsed / network_tick_delta).clamp(0.0, 1.0) as f32;
    let second = iter.next().unwrap();

    let interpolated = second
    .component()
    .linear_interpolate(latest.component(), per);
    Ok(Some(interpolated))
}
