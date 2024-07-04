use bevy::input::mouse::MouseMotion;
use bevy_replicon::client::confirm_history::ConfirmHistory;
use super::{
    level::*, 
    * 
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
            client_setup_floor
        ))
        .add_systems(Update, (
            handle_transport_error,
            handle_player_spawned,
            handle_input, 
            handle_action,
            draw_cc_gizmos_system 
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
    mut movements: EventWriter<NetworkMovement2_5D>,
    mut fires: EventWriter<NetworkFire>,
    latest_confirmed: Res<LatestConfirmedTick>
) {
    if let Ok(transform) = query.get_single() {
        let tick = latest_confirmed.get()
        .get();

        for (a, event_id) in actions.read_with_id() {
            if a.has_movement() {
                let mut bits = 0;
                if a.has_jump {
                    bits |= 0x01;
                }

                let current_translation = transform.translation;
                let current_yaw = transform.rotation.to_euler(EulerRot::YXZ)
                .0
                .to_degrees();  

                movements.send(NetworkMovement2_5D{
                    current_translation,
                    current_yaw,
                    linear_axis: a.movement_vec,
                    rotation_axis: a.rotation_vec,
                    bits,
                    index: event_id.id,
                    tick
                });
            }

            if a.has_fire {
                fires.send(NetworkFire{
                    index: event_id.id,
                    tick
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
        &NetworkCharacterController, 
        &NetworkAngle,
        &ConfirmHistory
    ), 
        Added<NetworkEntity>
    >,
    client: Res<Client>
) {
    for (
        e, net_e, 
        presentation, 
        net_trans, 
        net_rot, 
        confirmed_tick
    ) in query.iter() {
        let tick = confirmed_tick.last_tick()
        .get();

        commands.entity(e)
        .insert((
            PbrBundle{
                mesh: meshes.add(Mesh::from(Capsule3d::new(
                    CHARACTER_RADIUS, 
                    CHARACTER_HALF_HIGHT * 2.0
                ))),
                material: materials.add(presentation.color),
                transform: Transform{
                    translation: net_trans.to_vec3(TranslationAxis::XZ),
                    rotation: net_rot.to_quat(RotationAxis::Y),
                    scale: Vec3::ONE
                },
                ..default()
            },
            ComponentSnapshots::with_init(
                *net_trans, 
                tick, 
                DEV_SMALL_CACHE_SIZE
            ).expect("sytem time looks earlier than unix epoch"),
            ComponentSnapshots::with_init(
                *net_rot, 
                tick, 
                DEV_SMALL_CACHE_SIZE
            ).expect("sytem time looks earlier than unix epoch")
        ));

        if net_e.client_id()
        .get() == client.id() {
            commands.entity(e)
            .insert((
                Owning,
                CharacterControllerBundle::new(
                    CHARACTER_HALF_HIGHT,
                    CHARACTER_RADIUS,
                    CHARACTER_OFFSET,
                    CHARACTER_MASS
                ),
                Jump::default(),
                EventSnapshots::<NetworkFire>::with_capacity(DEV_NO_CACHE),
                EventSnapshots::<NetworkMovement2_5D>::with_capacity(DEV_NO_CACHE)
            ));
        } else {
            commands.entity(e)
            .insert(CharacterControllerBundle::replica(
                CHARACTER_HALF_HIGHT,
                CHARACTER_RADIUS
            ));
        }

        info!("player: {:?} spawned at tick: {}", net_e.client_id(), tick);
    } 
}

fn draw_cc_gizmos_system(
    query: Query<&NetworkCharacterController>,
    mut gizmos: Gizmos
) {
    for cc in query.iter() {
        gizmos.sphere(
            cc.0, 
            Quat::IDENTITY, 
            CHARACTER_RADIUS,
            Color::GREEN 
        );
    }
}