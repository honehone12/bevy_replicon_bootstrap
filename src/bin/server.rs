use std::{net::{IpAddr, Ipv4Addr}, time::Duration};
use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use bevy_replicon_action::{quick_net::server::*, dev::config::*};
use bevy_replicon::prelude::*;

fn main() {
    let config = ServerConfig{
        network_tick_rate: DEV_NETWORK_TICK_RATE,
        listen_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        listen_port: DEV_SERVER_LISTEN_PORT,
        protocol_id: get_dev_protocol_id(),
        private_key: get_dev_private_key(),
        max_clients: DEV_SERVER_MAX_CLIENTS,
    };
    
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f32(DEV_SERVER_TICK_DELTA)
        )),
        LogPlugin::default()
    ))
    .add_plugins(config.build_replicon());

    match config.setup_server(app.world.resource::<RepliconChannels>()) {
        Ok((server, renet, netcode)) => {
            app.insert_resource(server)
            .insert_resource(renet)
            .insert_resource(netcode)
            .run();
        }
        Err(e) => {
            panic!("{e}");
        }
    }
    
}