use fyrox::{
    core::{algebra::{Vector2, Vector3}, pool::Handle, visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*},
    event::{Event, WindowEvent, ElementState},
    keyboard::{PhysicalKey, KeyCode},
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
    scene::{
        animation::spritesheet::SpriteSheetAnimation,
        dim2::{rectangle::Rectangle, rigidbody::RigidBody},
        node::Node,
        Scene,
    },
};


#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5d471337-8b68-4052-922a-4fb86b5abf1e")]
#[visit(optional)]
pub struct Player {
    sprite: Handle<Node>,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
}


impl ScriptTrait for Player {
    fn on_init(&mut self, _context: &mut ScriptContext) {
        // Put initialization logic here.
    }

    fn on_start(&mut self, _context: &mut ScriptContext) {
        // There should be a logic that depends on other scripts in scene.
        // It is called right after **all** scripts were initialized.
    }

    fn on_deinit(&mut self, _context: &mut ScriptDeinitContext) {
        // Put de-initialization logic here.
    }

    fn on_os_event(&mut self, event: &Event<()>, context: &mut ScriptContext) {
        // Destructure the event object if the event is a WindowEvent
        if let Event::WindowEvent { event, .. } = event {

            // Destructure the WindowEvent if it is a KeyboardInput
            if let WindowEvent::KeyboardInput { event, .. } = event {

                // Check if the key is currently being pressed
                let pressed = event.state == ElementState::Pressed;

                // Check if the key being pressed/released is W, A, S, or D
                // Update state accordingly
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft)=> self.move_left = pressed,
                    PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight)=> self.move_right = pressed,
                    PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp)=> self.move_up = pressed,
                    PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown)=> self.move_down = pressed,
                    _ => {}
                }
            }
        }
    }

    fn on_update(&mut self, context: &mut ScriptContext) {
        // Grab the rigid body component from the entity
        if let Some(rigid_body) = context.scene.graph[context.handle].cast_mut::<RigidBody>() {
            
            // Determine the x and y speed based on the state of the keyboard input
            let x_speed = match (self.move_left, self.move_right) {
                (false, true) => 3.0, // If the player is moving left, set the x speed to 3.0
                (true, false) => -3.0, // If the player is moving right, set the x speed to -3.0
                _ => 0.0, // If the player is not moving left or right, set the x speed to 0.0
            };
            let y_speed = match (self.move_up, self.move_down) {
                (false, true) => 3.0, // If the player is moving up, set the y speed to 3.0
                (true, false) => -3.0, // If the player is moving down, set the y speed to -3.0
                _ => 0.0, // If the player is not moving up or down, set the y speed to 0.0
            };

            // Set the linear velocity of the rigid body based on the state of the player
            rigid_body.set_lin_vel(Vector2::new(x_speed, y_speed));
        }
    }
}

