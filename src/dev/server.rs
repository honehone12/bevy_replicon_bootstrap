use bevy::prelude::*;
use super::handle_transport_error;

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_transport_error);
    }
}

// fn handle_server_event_system(
//     mut commands: Commands,
//     mut events: EventReader<ServerEvent>,
//     mut palyer_entities: ResMut<PlayerEntityMap>,
//     netcode_server: Res<NetcodeServerTransport>, 
//     mut errors: EventWriter<NetstackError> 
// ) {
//     for e in events.read() {
//         match e {
//             ServerEvent::ClientConnected { client_id } => {
//                 let user_data = match netcode_server.user_data(
//                     RenetClientId::from_raw(client_id.get())
//                 ) {
//                     Some(u) => u,
//                     None => {
//                         errors.send(NetstackError(
//                             anyhow!("no user data for this client: {client_id:?}")
//                         ));
//                         return;
//                     }
//                 };

//                 let uuid = match Uuid::from_slice(&user_data[0..16]) {
//                     Ok(u) => u,
//                     Err(e) => {
//                         errors.send(NetstackError(e.into()));
//                         return;
//                     }
//                 };

//                 let entity = commands
//                     .spawn((
//                         ServerNetworkPlayerInfo::new(uuid),
//                         NetworkPlayer::new(*client_id)
//                     ))
//                     .id();
//                 match palyer_entities.try_insert(*client_id, entity) {
//                     Ok(()) => (),
//                     Err(e) => {
//                         errors.send(NetstackError(e));
//                     }
//                 }                
//                 info!("client: {client_id:?} id: {uuid} connected");
//             }
//             ServerEvent::ClientDisconnected { client_id, reason } => {
//                 match palyer_entities.get(client_id) {
//                     Some(e) => {
//                         commands.entity(*e).despawn();
//                         palyer_entities.remove(client_id);
//                     }
//                     None => ()
//                 }
//                 info!("client: {client_id:?} disconnected with reason: {reason}");
//             }
//         }
//     }
// }
