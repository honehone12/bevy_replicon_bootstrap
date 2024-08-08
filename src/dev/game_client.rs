use bevy::input::mouse::MouseMotion;
use bevy_replicon::client::confirm_history::ConfirmHistory;
use bevy::color::palettes::basic as color_palettes;
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
        .insert_resource(EntityPlayerMap::default())
        .add_event::<Action>()
        .add_event::<Fire>()
        .add_systems(Startup, (
            setup_light,
            setup_fixed_camera,
            client_setup_floor,
            client_setup_walls,
            client_setup_obstacles
        ))
        .add_systems(Update, (
            //handle_renetcode_error,
            handle_player_spawned,
            handle_input, 
            handle_action,
            handle_fire,
            draw_gizmos_system
        ).chain());
    }
}

#[derive(Event, Default)]
struct Action {
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

#[derive(Event)]
struct Fire;

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
    latest_confirmed: Res<LatestConfirmedTick>,
    mut fires: EventWriter<Fire>
) {
    let Ok(transform) = query.get_single() else {
        return;
    };

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
                current_angle: current_yaw,
                linear_axis: a.movement_vec,
                rotation_axis: a.rotation_vec,
                bits,
                index: event_id.id as u64,
                tick
            });
        }

        if a.has_fire {
            fires.send(Fire);
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
        &NetworkTranslation3D, 
        &NetworkAngleDegrees,
        &ConfirmHistory
    ), 
        Added<NetworkEntity>
    >,
    mut entity_player_map: ResMut<EntityPlayerMap>,
    axis: Res<TransformAxis>,
    replicon_client: Res<RepliconClient>
) {
    for (
        e, net_e, 
        presentation, 
        net_trans, net_rot, 
        confirmed_tick
    ) in query.iter() {
        let this_client_id = match replicon_client.status() {
            RepliconClientStatus::Connected { client_id: Some(id) } => id,
            _ => {
                error!("client status is not connected");
                continue;
            }
        };

        let tick = confirmed_tick.last_tick()
        .get();

        let entity = commands.entity(e)
        .insert((
            PbrBundle{
                mesh: meshes.add(Mesh::from(Capsule3d::new(
                    CHARACTER_RADIUS, 
                    CHARACTER_HALF_HIGHT * 2.0
                ))),
                material: materials.add(presentation.color),
                transform: Transform{
                    translation: net_trans.to_vec3(axis.translation),
                    rotation: net_rot.to_quat(axis.rotation),
                    ..default()
                },
                ..default()
            },
            ComponentCache::with_init(
                *net_trans, 
                tick, 
                SMALL_CACHE_SIZE
            ).expect("sytem time looks earlier than unix epoch"),
            ComponentCache::with_init(
                *net_rot, 
                tick, 
                SMALL_CACHE_SIZE
            ).expect("sytem time looks earlier than unix epoch")
        ))
        .id();

        let client_id = net_e.client_id();
        if this_client_id == client_id {
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
                EventCache::<NetworkMovement2_5D>::with_capacity(MEDIUM_CACHE_SIZE)
            ));
            info!("this is the owner of the spawned entity");
        } else {
            commands.entity(e)
            .insert(CharacterControllerBundle::replica(
                CHARACTER_HALF_HIGHT,
                CHARACTER_RADIUS
            ));
        }
        
        entity_player_map.try_insert(entity, client_id)
        .expect("same entity is already mapped");

        info!("player: {:?} spawned at tick: {}", client_id, tick);
    } 
}

fn handle_fire(
    query: Query<(Entity, &Transform), With<Owning>>,
    rapier: Res<RapierContext>,
    entity_player_map: Res<EntityPlayerMap>,
    latest_confirmed: Res<LatestConfirmedTick>,
    mut fires: EventReader<Fire>,
    mut hits: EventWriter<NetworkHit>
) {
    for (_, id) in fires.read_with_id() {
        let Ok((shooter_e, transform)) = query.get_single() else {
            continue;
        };
    
        let Some((e, intersection)) = rapier.cast_ray_and_get_normal(
            transform.translation, 
            transform.forward()
            .into(),
            FIRE_RANGE, 
            false, 
            QueryFilter::exclude_fixed()
            .exclude_sensors()
            .exclude_collider(shooter_e)
        ) else {
            continue;
        };
    
        let client_id = match entity_player_map.get(&e) {
            Some(id) => id.get(),
            None => continue
        };
    
        let tick = latest_confirmed
        .get()
        .get();
    
        info!(
            "requesting hit: client: {} point: {} at tick: {}", 
            client_id,
            intersection.point,
            tick
        );

        hits.send(NetworkHit{
            point: intersection.point,
            client_id,
            index: id.id as u64,
            tick
        });
    }
}

fn draw_gizmos_system(
    owning: Query<&Transform, With<Owning>>,
    query: Query<&NetworkTranslation3D>,
    mut gizmos: Gizmos
) {
    if let Ok(transform) = owning.get_single() {
        gizmos.ray(
            transform.translation, 
            transform.forward() * FIRE_RANGE, 
            color_palettes::AQUA
        );
    }

    const  RADIUS: f32 = 1.0;

    for net_trans in query.iter() {
        gizmos.sphere(
            net_trans.0, 
            Quat::IDENTITY, 
            RADIUS,
            color_palettes::GREEN 
        );
    }
}
