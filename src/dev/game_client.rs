use bevy::{
    prelude::*, 
    utils::SystemTime,
    input::mouse::MouseMotion 
};
use bevy_replicon::client::confirm_history::ConfirmHistory;
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
    pub movement_right: KeyCode,
    pub jump: KeyCode
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
            jump: KeyCode::Space
        })
        .insert_resource(MouseInputActionMap{
            fire: MouseButton::Left
        })
        .add_event::<Action>()
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
    pub rotation_vec: Vec2,
    pub has_jump: bool,
    pub has_fire: bool 
}

impl Action {
    #[inline]
    pub fn has_movement(&self) -> bool {
        self.movement_vec != Vec2::ZERO 
        || self.rotation_vec != Vec2::ZERO
        || self.has_jump
    }
    
    #[inline]
    pub fn has_action(&self) -> bool {
        self.has_movement() || self.has_fire 
    }
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
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

    if keyboard.just_pressed(keyboard_action_map.jump) {
        action.has_jump = true;
    }

    if mouse_button.just_pressed(mouse_action_map.fire) {
        action.has_fire = true;
    }

    for e in mouse_motion.read() {
        action.rotation_vec += e.delta;
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
                let mut bits = 0;
                if a.has_jump {
                    bits |= 0x01;
                }

                let current_translation = transform.translation.xz();
                let current_rotation = transform.rotation.to_euler(EulerRot::YXZ)
                .0
                .to_degrees();  

                movements.send(NetworkMovement2D{
                    current_translation,
                    current_rotation,
                    linear_axis: a.movement_vec,
                    rotation_axis: a.rotation_vec,
                    bits,
                    index: event_id.id,
                    timestamp,
                });
            }

            if a.has_fire {
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
        &NetworkAngle,
        &ConfirmHistory
    ), 
        Added<NetworkEntity>
    >
) {
    for (e, net_e, presentation, net_trans, net_rot, confirmed_tick) in query.iter() {
        let tick = confirmed_tick.last_tick().get();
        
        let mut trans_snaps = ComponentSnapshots
        ::with_capacity(DEV_MAX_SNAPSHOT_SIZE);
        match trans_snaps.insert(*net_trans, tick) {
            Ok(()) => (),
            Err(e) => {
                error(e.into());
                return;
            }
        }
        
        let mut rot_snaps = ComponentSnapshots
        ::with_capacity(DEV_MAX_SNAPSHOT_SIZE); 
        match rot_snaps.insert(*net_rot, tick) {
            Ok(()) => (),
            Err(e) => {
                error(e.into());
                return;
            }
        }

        let movement_snaps = EventSnapshots::<NetworkMovement2D>
        ::with_cache_capacity(DEV_MAX_SNAPSHOT_SIZE);

        let fire_snaps = EventSnapshots::<NetworkFire>
        ::with_cache_capacity(DEV_MAX_SNAPSHOT_SIZE);

        commands.entity(e)
        .insert((
            PbrBundle{
                mesh: meshes.add(Mesh::from(Cuboid::default())),
                material: materials.add(presentation.color),
                transform: Transform{
                    translation: net_trans.to_vec3(TranslationAxis::XZ),
                    rotation: net_rot.to_quat(RotationAxis::Y),
                    scale: Vec3::ONE
                },
                ..default()
            },
            trans_snaps,
            rot_snaps,
            movement_snaps,
            fire_snaps
        ));

        info!("player: {:?} spawned at tick: {}", net_e.client_id(), tick);
    } 
}
