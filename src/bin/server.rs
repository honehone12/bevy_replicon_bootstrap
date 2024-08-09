use std::{
    net::{IpAddr, Ipv4Addr}, 
    time::Duration
};
use bevy::{
    app::ScheduleRunnerPlugin, 
    log::LogPlugin, 
    prelude::*
};
use bevy_replicon_bootstrap::{
    prelude::*,
    dev::game_server::*, 
    dev::config::*
};

fn main() {
    let mut app = App::new();
    // let builder = RenetServerBuilder{
    //     network_tick_rate: DEV_NETWORK_TICK_RATE,
    //     listen_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
    //     listen_port: DEV_SERVER_LISTEN_PORT,
    //     protocol_id: get_dev_protocol_id(),
    //     private_key: get_dev_private_key(),
    //     max_clients: DEV_SERVER_MAX_CLIENTS,
    // };
    let builder = QuinnetServerBuilder{
        network_tick_rate: DEV_NETWORK_TICK_RATE,
        listen_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        listen_port: DEV_SERVER_LISTEN_PORT,
        cert_mode: CertificateRetrievalMode::GenerateSelfSigned { 
            server_hostname: "localhost".to_string() 
        }
        // cert_mode: CertificateRetrievalMode::LoadFromFile { 
        //     cert_file: "my_certificates/server.pub.pem".to_string(),
        //     key_file: "my_certificates/server.priv.pem".to_string() 
        // }
    };
    
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f32(DEV_SERVER_TICK_DELTA)
        )),
        LogPlugin{
            level: LOG_LEVEL,
            ..default()
        }
    ))
    .add_plugins(builder.build_plugin())
    .add_plugins(GameServerPlugin);

    match builder.build_transport(app.world_mut()) {
        Ok(_) => app.run(),
        Err(e) => panic!("{e}")
    };
}
