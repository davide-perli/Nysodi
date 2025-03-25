
use fyrox::{
    core::{algebra::Vector2, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    event::{ElementState, Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    scene::dim2::rigidbody::RigidBody,
    script::{ScriptContext, ScriptDeinitContext, ScriptTrait},
};


#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "36e58f1e-541b-40a7-ab1f-c021ab46929b")]
#[visit(optional)]
pub struct Player {
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
}

impl ScriptTrait for Player {
    fn on_init(&mut self, context: &mut ScriptContext) {
        // Put initialization logic here.
    }

    fn on_start(&mut self, context: &mut ScriptContext) {
        // There should be a logic that depends on other scripts in scene.
        // It is called right after **all** scripts were initialized.
    }

    fn on_deinit(&mut self, context: &mut ScriptDeinitContext) {
        // Put de-initialization logic here.
    }

    fn on_os_event(&mut self, event: &Event<()>, _context: &mut ScriptContext) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { event, .. } = event {
                let pressed = event.state == ElementState::Pressed;
    
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                        self.move_left = pressed;
                    }
                    PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight) => {
                        self.move_right = pressed;
                    }
                    PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                        self.move_up = pressed;
                    }
                    PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                        self.move_down = pressed;
                    }
                    _ => {}
                }
            }
        }
    }
    

    fn on_update(&mut self, context: &mut ScriptContext) {
        if let Some(rigid_body) = context.scene.graph[context.handle].cast_mut::<RigidBody>() {
            
            let x_speed = match (self.move_left, self.move_right) {
                (true, false) => -3.0,
                (false, true) => 3.0,
                _ => 0.0,
            };
    
            let y_speed = match (self.move_up, self.move_down) {
                (false, true) => 3.0,
                (true, false) => -3.0,
                _ => 0.0,
            };
    
            rigid_body.set_lin_vel(Vector2::new(x_speed, y_speed));
        }
    }
    
}
    