use std::net::{IpAddr, SocketAddr, UdpSocket};
use bevy::{
    prelude::*,
    utils::SystemTime
};
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    client::RepliconRenetClientPlugin,
    renet::{
        transport::{ClientAuthentication, ConnectToken, NetcodeClientTransport}, 
        ConnectionConfig, RenetClient
    }, 
    RenetChannelsExt
};
use crate::core::network_resource::*;

pub struct RenetClientBuilder {
    pub client_addr: IpAddr,
    pub server_addr: IpAddr,
    pub server_port: u16,
    pub timeout_seconds: i32,
    pub client_id: u64,
    pub protocol_id: u64,
    pub private_key: [u8; 32],
    pub user_data: [u8; 256],
    pub token_expire_seconds: u64,
}

impl RenetClientBuilder {
    pub fn build_replicon(&self)
    -> (impl PluginGroup, impl Plugin) {
        let replicon = RepliconPlugins.build()
        .disable::<ServerPlugin>();
        
        (replicon, RepliconRenetClientPlugin)
    }

    pub fn build_transport(&self, net_channels: &RepliconChannels)
    -> anyhow::Result<(Client, RenetClient, NetcodeClientTransport)> {
        let renet_client = RenetClient::new(ConnectionConfig{
            server_channels_config: net_channels.get_server_configs(),
            client_channels_config: net_channels.get_client_configs(),
            ..default()
        });

        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let socket = UdpSocket::bind((self.client_addr, 0))?;
        let connect_token = ConnectToken::generate(
            current_time,
            self.protocol_id,
            self.token_expire_seconds,
            self.client_id,
            self.timeout_seconds,
            vec![SocketAddr::new(self.server_addr, self.server_port)],
            Some(&self.user_data),
            &self.private_key
        )?;
        let auth = ClientAuthentication::Secure {connect_token};
        let netcode_transport = NetcodeClientTransport::new(current_time, auth, socket)?;
        
        Ok((Client::new(self.client_id), renet_client, netcode_transport))    
    }
}
