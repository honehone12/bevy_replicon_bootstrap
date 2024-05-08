pub mod dev;
pub mod quick_net;
pub mod core;

pub mod prelude {
    pub use crate::{
        core::{
            component_snapshot::*, 
            event_snapshot::*,
            interpolation::*,
            network_event::*,
            player_entity_map::*,
            owner::*
        },
        quick_net::{
            client::*,
            server::*,
            network_transform::*,
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn unimplemented_test() {
        unimplemented!("tests are not ready");
    }
}
