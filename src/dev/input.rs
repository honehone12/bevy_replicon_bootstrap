use bevy::prelude::*;

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
pub struct ActionEvent {
    pub movement_vec: Vec2,
    pub is_fire: bool 
}

impl ActionEvent {
    #[inline]
    pub fn has_movement(&self) -> bool {
        self.movement_vec != Vec2::ZERO
    }
    
    #[inline]
    pub fn has_action(&self) -> bool {
        self.has_movement() || self.is_fire
    }
}

pub fn handle_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard_action_map: Res<KeyboardInputActionMap>,
    mouse_action_map: Res<MouseInputActionMap>,
    mut actions: EventWriter<ActionEvent> 
) {
    let mut action = ActionEvent::default();
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


