use bevy::prelude::*;
use bevy_replicon::{
    client::confirm_history::ConfirmHistory, 
    core::replicon_tick::RepliconTick
};

#[derive(Resource, Default)]
pub struct LatestConfirmedTick(RepliconTick);

impl LatestConfirmedTick {
    #[inline]
    pub fn get(&self) -> RepliconTick {
        self.0
    }

    #[inline]
    pub fn try_set(&mut self, tick: RepliconTick) -> bool {
        if tick > self.0 {
            self.0 = tick;
            return true;
        } 

        false
    }
}

pub(crate) fn latest_confirmed_tick_system(
    query: Query<&ConfirmHistory, Changed<ConfirmHistory>>,
    mut latest_confirmed: ResMut<LatestConfirmedTick>
) {
    for confirm in query.iter() {
        let tick = confirm.last_tick();
        if latest_confirmed.try_set(tick) {
            debug!("updated lates confirmed tick: {tick:?}");
        }
    }
}