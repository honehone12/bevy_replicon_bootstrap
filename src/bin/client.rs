use std::net::{IpAddr, Ipv4Addr};

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_action::{quick_net::client::*, dev::config::*};

fn main() {
    let config = ClientConfig{
        client_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_addr: IpAddr::V4(Ipv4Addr::LOCALHOST),
        server_port: DEV_SERVER_LISTEN_PORT,
        timeout_seconds: DEV_CLIENT_TIME_OUT_SEC,
        client_id: get_dev_client_id(),
        protocol_id: get_dev_protocol_id(),
        private_key: get_dev_private_key(),
        // I think user data is sent after encryption, am I correct?.
        // https://github.com/mas-bandwidth/netcode/blob/main/STANDARD.md
        user_data: get_dev_user_data(),
        token_expire_seconds: DEV_TOKEN_EXPIRE_SEC,
    };
    
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
    .add_plugins(config.build_replicon());

    match config.build_transport(app.world.resource::<RepliconChannels>()) {
        Ok((client, renet, netcode)) => {
            app.insert_resource(client)
            .insert_resource(renet)
            .insert_resource(netcode)
            .run();
        },
        Err(e) => {
            panic!("{e}");
        }
    }
}
