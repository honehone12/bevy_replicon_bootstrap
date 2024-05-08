use std::time::SystemTime;
use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use rand::prelude::*;
use crate::{
    prelude::*,
    dev::{level::*, event::*, *}
};

pub struct GameClientPlugin;

impl Plugin for GameClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_transport_error)
        .add_systems(Startup, (
            setup_floor,
            setup_light,
            setup_fixed_camera
        ))
        .add_systems(Update, (
           handle_input, handle_action 
        ).chain());
    }
}

#[derive(Resource)]
pub struct PlayerMovementParams {
    pub base_speed: f32,
    pub prediction_error_threashold: f32
}

#[derive(Component, Serialize, Deserialize)]
pub struct PlayerPresentation {
    pub color: Color
}

impl PlayerPresentation {
    #[inline]
    pub fn random() -> Self {
        Self{
            color: Color::rgb(
                random(), 
                random(), 
                random()
            )
        }
    }
}

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
