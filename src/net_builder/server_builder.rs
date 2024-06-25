use std::net::{IpAddr, SocketAddr, UdpSocket};
use bevy::{
    app::PluginGroupBuilder, 
    prelude::*,
    utils::SystemTime
};
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{
        transport::{NetcodeServerTransport, ServerAuthentication}, 
        ConnectionConfig, RenetServer
    }, 
    RenetChannelsExt, RepliconRenetClientPlugin, RepliconRenetPlugins
};
use bevy_replicon_renet::renet::transport::ServerConfig as RenetServerConfig;

#[derive(Resource)]
pub struct Server;

pub struct ServerBuilder {
    pub network_tick_rate: u16,
    pub listen_addr: IpAddr,
    pub listen_port: u16,
    pub protocol_id: u64,
    pub private_key: [u8; 32],
    pub max_clients: usize
}

impl ServerBuilder {
    pub fn build_replicon(&self) -> (PluginGroupBuilder, PluginGroupBuilder) {
        let replicon = RepliconPlugins.build()
        .disable::<ClientPlugin>()
        .set(
            ServerPlugin{
                tick_policy: TickPolicy::MaxTickRate(self.network_tick_rate),
                visibility_policy: VisibilityPolicy::Whitelist,
                ..default()
            }
        );
        let replicon_renet = RepliconRenetPlugins.build()
        .disable::<RepliconRenetClientPlugin>(); 
        
        (replicon, replicon_renet)
    }

    pub fn build_transport(&self, net_channels: &RepliconChannels) 
    -> anyhow::Result<(Server, RenetServer, NetcodeServerTransport)> {
        let renet_server = RenetServer::new(ConnectionConfig{
            server_channels_config: net_channels.get_server_configs(),
            client_channels_config: net_channels.get_client_configs(),
            ..default()
        });

        let listen_addr = SocketAddr::new(
            self.listen_addr, 
            self.listen_port
        );
        let socket = UdpSocket::bind(listen_addr)?;
        let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?;
        let netcode_transport = NetcodeServerTransport::new(
            RenetServerConfig{
                current_time,
                max_clients: self.max_clients,
                protocol_id: self.protocol_id,
                authentication: ServerAuthentication::Secure{ 
                    private_key: self.private_key
                },
                public_addresses: vec![listen_addr]
            }, 
            socket
        )?;

        info!("server built at: {listen_addr}");
        Ok((Server, renet_server, netcode_transport))
    }
}
