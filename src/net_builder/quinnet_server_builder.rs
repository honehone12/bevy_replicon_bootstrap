use std::net::IpAddr;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_quinnet::{
    ChannelsConfigurationExt, 
    RepliconQuinnetServerPlugin
};
use bevy_quinnet::server::{
    certificate::ServerCertificate, 
    ServerEndpointConfiguration,
    QuinnetServer,
};
pub use bevy_quinnet::server::certificate::CertificateRetrievalMode;

pub struct QuinnetServerBuilder {
    pub network_tick_rate: u16,
    pub listen_addr: IpAddr,
    pub listen_port: u16,
    pub cert_mode: CertificateRetrievalMode
}

impl QuinnetServerBuilder {
    pub fn build_plugin(&self) 
    -> (impl PluginGroup, impl Plugin) {
        let replicon = RepliconPlugins.build()
        .disable::<ClientPlugin>()
        .set(
            ServerPlugin{
                tick_policy: TickPolicy::MaxTickRate(self.network_tick_rate),
                visibility_policy: VisibilityPolicy::Whitelist,
                ..default()
            }
        );
        
        (replicon, RepliconQuinnetServerPlugin)
    }

    pub fn build_transport(self, world: &mut World) 
    -> anyhow::Result<ServerCertificate> {
        let endpoint_config = ServerEndpointConfiguration::from_ip(
            self.listen_addr, 
            self.listen_port
        ); 
        let channel_config = world.resource::<RepliconChannels>()
        .get_server_configs();
        
        let server_cert = world.resource_mut::<QuinnetServer>()
        .start_endpoint(
            endpoint_config, 
            self.cert_mode, 
            channel_config
        )?;

        Ok(server_cert)
    }
}