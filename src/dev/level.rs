use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::core::PlayerStart;

pub const FLOOR_SIZE: Vec3 = Vec3::new(50.0, 1.0, 50.0);
pub const FLOOR_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);
pub const FLOOR_POSITION: Vec3 = Vec3::new(0.0, -0.5, 0.0);
pub const LIGHT_POSITION: Vec3 = Vec3::new(0.0, 50.0, 0.0);
pub const LIGHT_ROTATION_X: f32 = -std::f32::consts::PI / 4.0;
pub const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 70.0, 25.0);

pub fn client_setup_floor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    commands.spawn((
        PbrBundle{
            mesh: meshes.add(Mesh::from(Cuboid::from_size(FLOOR_SIZE))),
            material: materials.add(FLOOR_COLOR),
            transform: Transform::from_translation(FLOOR_POSITION),
            ..default()
        },
        Collider::cuboid(
            FLOOR_SIZE.x * 0.5, 
            FLOOR_SIZE.y * 0.5, 
            FLOOR_SIZE.z * 0.5
        )
    ));
}

pub fn server_setup_floor(mut commands: Commands) {
    commands.spawn((
        TransformBundle::from_transform(
            Transform::from_translation(FLOOR_POSITION)
        ),
        Collider::cuboid(
            FLOOR_SIZE.x * 0.5, 
            FLOOR_SIZE.y * 0.5, 
            FLOOR_SIZE.z * 0.5
        )
    ));
}

const WALL_HEIGHT: f32 = 10.0;
const WALL_LENGTH: f32 = 1.0;

pub fn client_setup_walls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    commands.spawn((
        PbrBundle{
            mesh: meshes.add(Mesh::from(Cuboid::from_size(Vec3::new(
                FLOOR_SIZE.x,
                WALL_HEIGHT,
                WALL_LENGTH
            )))),
            material: materials.add(FLOOR_COLOR),
            transform: Transform::from_translation(Vec3::new(
                0.0,
                WALL_HEIGHT * 0.5,
                FLOOR_SIZE.z * 0.5 - WALL_LENGTH * 0.5
            )),
            ..default()
        },
        Collider::cuboid(
            FLOOR_SIZE.x * 0.5, 
            WALL_HEIGHT * 0.5, 
            WALL_LENGTH * 0.5
        )
    ));

    commands.spawn((
        PbrBundle{
            mesh: meshes.add(Mesh::from(Cuboid::from_size(Vec3::new(
                FLOOR_SIZE.x,
                WALL_HEIGHT,
                WALL_LENGTH
            )))),
            material: materials.add(FLOOR_COLOR),
            transform: Transform::from_translation(Vec3::new(
                0.0,
                WALL_HEIGHT * 0.5,
                -(FLOOR_SIZE.z * 0.5 - WALL_LENGTH * 0.5)
            )),
            ..default()
        },
        Collider::cuboid(
            FLOOR_SIZE.x * 0.5, 
            WALL_HEIGHT * 0.5, 
            WALL_LENGTH * 0.5
        )
    ));

    commands.spawn((
        PbrBundle{
            mesh: meshes.add(Mesh::from(Cuboid::from_size(Vec3::new(
                WALL_LENGTH,
                WALL_HEIGHT,
                FLOOR_SIZE.z
            )))),
            material: materials.add(FLOOR_COLOR),
            transform: Transform::from_translation(Vec3::new(
                FLOOR_SIZE.x * 0.5 - WALL_LENGTH * 0.5,
                WALL_HEIGHT * 0.5,
                0.0
            )),
            ..default()
        },
        Collider::cuboid(
            WALL_LENGTH * 0.5, 
            WALL_HEIGHT * 0.5, 
            FLOOR_SIZE.z * 0.5
        )
    ));

    commands.spawn((
        PbrBundle{
            mesh: meshes.add(Mesh::from(Cuboid::from_size(Vec3::new(
                WALL_LENGTH,
                WALL_HEIGHT,
                FLOOR_SIZE.z
            )))),
            material: materials.add(FLOOR_COLOR),
            transform: Transform::from_translation(Vec3::new(
                -(FLOOR_SIZE.x * 0.5 - WALL_LENGTH * 0.5),
                WALL_HEIGHT * 0.5,
                0.0
            )),
            ..default()
        },
        Collider::cuboid(
            WALL_LENGTH * 0.5, 
            WALL_HEIGHT * 0.5, 
            FLOOR_SIZE.z * 0.5
        )
    ));
}

pub fn server_setup_walls(mut commands: Commands) {
    commands.spawn((
        TransformBundle::from_transform(
            Transform::from_translation(Vec3::new(
                0.0,
                WALL_HEIGHT * 0.5,
                FLOOR_SIZE.z * 0.5 - WALL_LENGTH * 0.5
            ))
        ),
        Collider::cuboid(
            FLOOR_SIZE.x * 0.5, 
            WALL_HEIGHT * 0.5, 
            WALL_LENGTH * 0.5
        )
    ));

    commands.spawn((
        TransformBundle::from_transform(
            Transform::from_translation(Vec3::new(
                0.0,
                WALL_HEIGHT * 0.5,
                -(FLOOR_SIZE.z * 0.5 - WALL_LENGTH * 0.5)
            ))
        ),
        Collider::cuboid(
            FLOOR_SIZE.x * 0.5, 
            WALL_HEIGHT * 0.5, 
            WALL_LENGTH * 0.5
        )
    ));

    commands.spawn((
        TransformBundle::from_transform(
            Transform::from_translation(Vec3::new(
                FLOOR_SIZE.x * 0.5 - WALL_LENGTH * 0.5,
                WALL_HEIGHT * 0.5,
                0.0
            ))
        ),
        Collider::cuboid(
            WALL_LENGTH * 0.5, 
            WALL_HEIGHT * 0.5, 
            FLOOR_SIZE.z * 0.5
        )
    ));

    commands.spawn((
        TransformBundle::from_transform(
            Transform::from_translation(Vec3::new(
                -(FLOOR_SIZE.x * 0.5 - WALL_LENGTH * 0.5),
                WALL_HEIGHT * 0.5,
                0.0
            ))
        ),
        Collider::cuboid(
            WALL_LENGTH * 0.5, 
            WALL_HEIGHT * 0.5, 
            FLOOR_SIZE.z * 0.5
        )
    ));
}

pub fn setup_light(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle{
        directional_light: DirectionalLight{
            shadows_enabled: true,
            ..default()
        },
        transform: Transform{
            translation: LIGHT_POSITION,
            rotation: Quat::from_rotation_x(LIGHT_ROTATION_X),
            ..default()
        },
        ..default()
    });
}

pub fn setup_fixed_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle{
        transform: Transform::from_translation(CAMERA_POSITION)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}



pub const SPAWN_POSITION_0: Vec3 = Vec3::new(-10.0, 1.5, -10.0);
pub const SPAWN_POSITION_1: Vec3 = Vec3::new(10.0, 1.5, -10.0);
pub const SPAWN_POSITION_2: Vec3 = Vec3::new(10.0, 1.5, 10.0);
pub const SPAWN_POSITION_3: Vec3 = Vec3::new(-10.0, 1.5, 10.0);

pub const PLAYER_START_0: PlayerStart = PlayerStart{
    translation: SPAWN_POSITION_0,
    rotation: Quat::IDENTITY
};
pub const PLAYER_START_1: PlayerStart = PlayerStart{
    translation: SPAWN_POSITION_1,
    rotation: Quat::IDENTITY
};
pub const PLAYER_START_2: PlayerStart = PlayerStart{
    translation: SPAWN_POSITION_2,
    rotation: Quat::IDENTITY
};
pub const PLAYER_START_3: PlayerStart = PlayerStart{
    translation: SPAWN_POSITION_3,
    rotation: Quat::IDENTITY
};
