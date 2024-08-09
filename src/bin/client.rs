use std::net::{IpAddr, Ipv4Addr};
use bevy::{log::LogPlugin, prelude::*};
//use bevy_quinnet::client::certificate::TrustOnFirstUseConfig;
use bevy_replicon_bootstrap::{
    prelude::*,
    dev::game_client::*,
    dev::config::*
};

fn main() {
    let mut app = App::new();
    // let builder = RenetClientBuilder{
    //     client_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
    //     server_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
    //     server_port: DEV_SERVER_LISTEN_PORT,
    //     timeout_seconds: DEV_CLIENT_TIME_OUT_SEC,
    //     client_id: get_dev_client_id(),
    //     protocol_id: get_dev_protocol_id(),
    //     private_key: get_dev_private_key(),
    //     user_data: get_dev_user_data(),
    //     token_expire_seconds: DEV_TOKEN_EXPIRE_SEC,
    // };
    let builder = QuinnetClientBuilder{
        server_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_port: DEV_SERVER_LISTEN_PORT,
        client_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        client_port: 0,
        cert_mode: CertificateVerificationMode::SkipVerification
        // cert_mode: CertificateVerificationMode::TrustOnFirstUse(
        //     TrustOnFirstUseConfig::default()
        // )
    };
    
    app.add_plugins(
        DefaultPlugins.set(LogPlugin{
            level: LOG_LEVEL,
            ..default()
        })
    )
    .add_plugins(builder.build_plugin())
    .add_plugins(GameClientPlugin);

    match builder.build_transport(app.world_mut()) {
        Ok(_) => app.run(),
        Err(e) => panic!("{e}")
    };
}
