use std::net::IpAddr;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_quinnet::{ChannelsConfigurationExt, RepliconQuinnetClientPlugin};
use bevy_quinnet::client::{
    connection::{
        ClientEndpointConfiguration, 
        ConnectionLocalId
    },
    QuinnetClient
};
pub use bevy_quinnet::client::certificate::CertificateVerificationMode;

pub struct QuinnetClientBuilder {
    pub server_addr: IpAddr,
    pub server_port: u16,
    pub client_addr: IpAddr,
    pub client_port: u16,
    pub cert_mode: CertificateVerificationMode
} 

impl QuinnetClientBuilder {
    pub fn build_plugin(&self)
    -> (impl PluginGroup, impl Plugin) {
        let replicon = RepliconPlugins.build()
        .disable::<ServerPlugin>();

        (replicon, RepliconQuinnetClientPlugin)
    }

    pub fn build_transport(self, world: &mut World) 
    -> anyhow::Result<ConnectionLocalId> {
        let endpoint_config = ClientEndpointConfiguration::from_ips(
            self.server_addr, 
            self.server_port, 
            self.client_addr, 
            self.client_port
        );

        let channel_config = world.resource::<RepliconChannels>()
        .get_client_configs();
        let conn_id = world.resource_mut::<QuinnetClient>()
        .open_connection(
            endpoint_config, 
            self.cert_mode, 
            channel_config    
        )?;

        Ok(conn_id)
    }
}
