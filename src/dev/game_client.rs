use bevy::{
    prelude::*,
    utils::SystemTime
};
use bevy_replicon::{
    client::confirmed::Confirmed, 
    prelude::*
};
use crate::{
    dev::{
        config::DEV_MAX_SNAPSHOT_SIZE,
        level::*, 
        *
    }, 
    prelude::*,
};

#[derive(Resource)]
pub struct KeyboardInputActionMap {
    pub movement_up: KeyCode,
    pub movement_left: KeyCode,
    pub movement_down: KeyCode,
    pub movement_right: KeyCode
}

#[derive(Resource)]
pub struct MouseInputActionMap {
    pub fire: MouseButton
}

pub struct GameClientPlugin;

impl Plugin for GameClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GameCommonPlugin)
        .insert_resource(KeyboardInputActionMap{
            movement_up: KeyCode::KeyW,
            movement_left: KeyCode::KeyA,
            movement_down: KeyCode::KeyS,
            movement_right: KeyCode::KeyD,
        })
        .insert_resource(MouseInputActionMap{
            fire: MouseButton::Left
        })
        .add_event::<Action>()
        .add_client_event::<NetworkMovement2D>(ChannelKind::Unreliable)
        .add_systems(Startup, (
            setup_light,
            setup_fixed_camera,
            setup_floor
        ))
        .add_systems(Update, (
            handle_transport_error,
            handle_player_spawned,
            handle_input, 
            handle_action 
        ).chain());
    }
}

#[derive(Event, Default)]
pub struct Action {
    pub movement_vec: Vec2,
    pub is_fire: bool 
}

impl Action {
    #[inline]
    pub fn has_movement(&self) -> bool {
        self.movement_vec != Vec2::ZERO
    }
    
    #[inline]
    pub fn has_action(&self) -> bool {
        self.has_movement() || self.is_fire
    }
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard_action_map: Res<KeyboardInputActionMap>,
    mouse_action_map: Res<MouseInputActionMap>,
    mut actions: EventWriter<Action> 
) {
    let mut action = Action::default();
    if keyboard.pressed(keyboard_action_map.movement_up) {
        action.movement_vec.y += 1.0
    } 
    if keyboard.pressed(keyboard_action_map.movement_down) {
        action.movement_vec.y -= 1.0
    }
    if keyboard.pressed(keyboard_action_map.movement_right) {
        action.movement_vec.x += 1.0
    }
    if keyboard.pressed(keyboard_action_map.movement_left) {
        action.movement_vec.x -= 1.0
    }

    if mouse.just_pressed(mouse_action_map.fire) {
        action.is_fire = true;
    }

    if action.has_action() {
        actions.send(action);
    }
} 

fn handle_action(
    query: Query<&Transform, With<Owning>>,
    mut actions: EventReader<Action>,
    mut movements: EventWriter<NetworkMovement2D>,
    mut fires: EventWriter<NetworkFire>
) {
    if let Ok(transform) = query.get_single() {
        for (a, event_id) in actions.read_with_id() {
            let timestamp = match SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH) {
                Ok(d) => d.as_secs_f64(),
                Err(e) => {
                    error(e.into());
                    return;
                }
            };

            if a.has_movement() {
                movements.send(NetworkMovement2D{
                    current_translation: Vec2::new(
                        transform.translation.x,
                        transform.translation.z
                    ),
                    axis: a.movement_vec,
                    index: event_id.id,
                    timestamp
                });
            }
            if a.is_fire {
                fires.send(NetworkFire{
                    index: event_id.id,
                    timestamp
                });
            }
        }
    }
}

fn handle_player_spawned(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(
        Entity,
        &NetworkEntity, 
        &PlayerPresentation, 
        &NetworkTranslation2D, 
        &NetworkYaw,
        &Confirmed
    ), 
        Added<NetworkEntity>
    >,
    client: Res<Client>,
) {
    for (e, net_e, presentation, net_t2d, net_yaw, confirmed_tick) in query.iter() {
        let tick = confirmed_tick.last_tick().get();
        let mut translation_snaps = ComponentSnapshots::with_capacity(DEV_MAX_SNAPSHOT_SIZE);
        match translation_snaps.insert(*net_t2d, tick) {
            Ok(()) => (),
            Err(e) => {
                error(e.into());
                return;
            }
        }
        let mut yaw_snaps = ComponentSnapshots::with_capacity(DEV_MAX_SNAPSHOT_SIZE); 
        match yaw_snaps.insert(*net_yaw, tick) {
            Ok(()) => (),
            Err(e) => {
                error(e.into());
                return;
            }
        }

        commands.entity(e)
        .insert((
            PbrBundle{
                mesh: meshes.add(Mesh::from(Capsule3d::default())),
                material: materials.add(presentation.color),
                transform: Transform{
                    translation: net_t2d.to_3d(),
                    rotation: net_yaw.to_quat(),
                    scale: Vec3::ONE
                },
                ..default()
            },
            translation_snaps,
            yaw_snaps
        ));

        if net_e.client_id().get() == client.id() {
            commands.entity(e).insert(Owning);
        }

        info!("player: {:?} spawned at tick: {}", net_e.client_id(), tick);
    } 
}
