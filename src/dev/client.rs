use std::time::SystemTime;
use bevy::prelude::*;
use bevy_replicon::client::ServerEntityTicks;
use crate::{
    prelude::*,
    dev::{
        *, level::*, event::*, 
        config::DEV_MAX_BUFFER_SIZE
    }
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
        app.insert_resource(KeyboardInputActionMap{
            movement_up: KeyCode::KeyW,
            movement_left: KeyCode::KeyA,
            movement_down: KeyCode::KeyS,
            movement_right: KeyCode::KeyD,
        })
        .insert_resource(MouseInputActionMap{
            fire: MouseButton::Left
        })
        .add_event::<Action>()
        .add_systems(Startup, (
            setup_floor,
            setup_light,
            setup_fixed_camera
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
    query: Query<&Owning>,
    mut actions: EventReader<Action>,
    mut movements: EventWriter<NetworkMovement2D>,
    mut fires: EventWriter<NetworkFire>
) {
    if let Ok(_) = query.get_single() {
        for (a, event_id) in actions.read_with_id() {
            let now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                Ok(d) => d,
                Err(e) => {
                    error(e.into());
                    return;
                }
            };
            let timestamp = now.as_secs_f64();

            if a.has_movement() {
                movements.send(NetworkMovement2D{
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
        &NetworkYaw
    ), 
        Added<NetworkEntity>
    >,
    client: Res<Client>,
    server_ticks: Res<ServerEntityTicks>
) {
    for (e, net_e, presentation, net_t2d, net_yaw) in query.iter() {
        let tick = server_ticks.get(&e)
        .expect("server tick should be mapped").get();
        
        let mut translation_snaps = ComponentSnapshots::with_capacity(DEV_MAX_BUFFER_SIZE);
        translation_snaps.insert(*net_t2d, tick)
        .expect("check system time of the computer");
        let mut yaw_snaps = ComponentSnapshots::with_capacity(DEV_MAX_BUFFER_SIZE); 
        yaw_snaps.insert(*net_yaw, tick)
        .expect("check system time of the computer");
        
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
            yaw_snaps,
        ));

        if net_e.client_id().get() == client.id() {
            commands.entity(e).insert(Owning);
        }

        info!("player: {:?} spawned at tick: {}", net_e.client_id(), tick);
    } 
}
