//! Game project.

mod bot;
mod residuals;

// ANCHOR: imports
use crate::bot::Bot;
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    event::{ElementState, Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::{
        animation::spritesheet::SpriteSheetAnimation,
        dim2::{rectangle::Rectangle, rigidbody::RigidBody},
        node::Node,
        Scene,
    },
    script::{ScriptContext, ScriptTrait},
};
use std::path::Path;
// ANCHOR_END: imports

#[derive(Visit, Reflect, Debug, Default)]
pub struct Game {
    scene: Handle<Scene>,

    // ANCHOR: player_field
    player: Handle<Node>,
    // ANCHOR_END: player_field
}

// ANCHOR: register
impl Plugin for Game {
    fn register(&self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors.add::<Player>("Player");
        // ...
        // ANCHOR_END: register
        script_constructors.add::<Bot>("Bot");
    }

    fn init(&mut self, scene_path: Option<&str>, context: PluginContext) {
        context
            .async_scene_loader
            .request(scene_path.unwrap_or("data/scene.rgs"));
    }

    fn on_scene_loaded(
        &mut self,
        _path: &Path,
        scene: Handle<Scene>,
        _data: &[u8],
        context: &mut PluginContext,
    ) {
        if self.scene.is_some() {
            context.scenes.remove(self.scene);
        }

        self.scene = scene;
    }

    fn update(&mut self, context: &mut PluginContext) {
        if let Some(scene) = context.scenes.try_get_mut(self.scene) {
            scene.drawing_context.clear_lines();
            // scene.graph.physics2d.draw(&mut scene.drawing_context);
        }
    }
}

// ANCHOR: sprite_field
#[derive(Visit, Reflect, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "c5671d19-9f1a-4286-8486-add4ebaadaec")]
#[visit(optional)]
struct Player {
    sprite: Handle<Node>,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,

    // ANCHOR: animation_fields
    animations: Vec<SpriteSheetAnimation>,
    current_animation: u32,
    // ANCHOR_END: animation_fields
}

// ANCHOR: animation_fields_defaults_begin
impl Default for Player {
    fn default() -> Self {
        Self {
            // ANCHOR_END: animation_fields_defaults_begin
            sprite: Handle::NONE,
            move_left: false,
            move_right: false,
            move_up:false,
            move_down:false,
            // ANCHOR: animation_fields_defaults_end
            // ...
            animations: Default::default(),
            current_animation: 0,
        }
    }
}
// ANCHOR_END: animation_fields_defaults_end

impl ScriptTrait for Player {
    // ANCHOR: set_player_field
    fn on_start(&mut self, ctx: &mut ScriptContext) {
        ctx.plugins.get_mut::<Game>().player = ctx.handle;
    }
    // ANCHOR_END: set_player_field

    // ANCHOR: on_os_event
    // Called everytime when there is an event from OS (mouse click, key press, etc.)
    fn on_os_event(&mut self, event: &Event<()>, _context: &mut ScriptContext) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { event, .. } = event {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;

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
    }
    // ANCHOR_END: on_os_event

    // Called every frame at fixed rate of 60 FPS.
    // ANCHOR: on_update_begin
    fn on_update(&mut self, context: &mut ScriptContext) {
        // The script can be assigned to any scene node, but we assert that it will work only with
        // 2d rigid body nodes.
        if let Some(rigid_body) = context.scene.graph[context.handle].cast_mut::<RigidBody>() {
            
            // Determine the x and y speed based on the state of the keyboard input
            let x_speed = match (self.move_left, self.move_right) {
                (true, false) => 3.0, // If the player is moving left, set the x speed to 3.0
                (false, true) => -3.0, // If the player is moving right, set the x speed to -3.0
                _ => 0.0, // If the player is not moving left or right, set the x speed to 0.0
            };
            let y_speed = match (self.move_up, self.move_down) {
                (true, false) => 3.0, // If the player is moving up, set the y speed to 3.0
                (false, true) => -3.0, // If the player is moving down, set the y speed to -3.0
                _ => 0.0, // If the player is not moving up or down, set the y speed to 0.0
            };

            // Set the linear velocity of the rigid body based on the state of the player
            rigid_body.set_lin_vel(Vector2::new(x_speed, y_speed));
            // ...
            // ANCHOR_END: on_update_begin

            // ANCHOR: sprite_scaling
            // It is always a good practice to check whether the handles are valid, at this point we don't know
            // for sure what's the value of the `sprite` field. It can be unassigned and the following code won't
            // execute. A simple `context.scene.graph[self.sprite]` would just panicked in this case.
            if let Some(sprite) = context.scene.graph.try_get_mut(self.sprite) {
                // We want to change player orientation only if he's moving.
                if x_speed != 0.0 {
                    let local_transform = sprite.local_transform_mut();

                    let current_scale = **local_transform.scale();

                    local_transform.set_scale(Vector3::new(
                        // Just change X scaling to mirror player's sprite.
                        current_scale.x.copysign(-x_speed),
                        current_scale.y,
                        current_scale.z,
                    ));
                }

            }
            // ANCHOR_END: sprite_scaling

            // ANCHOR: animation_selection
            if x_speed != 0.0 {
                self.current_animation = 0;
            } else{
                if y_speed != 0.0 {
                self.current_animation = 0;

                } else{
                    self.current_animation = 1;
                }
            }
            // ANCHOR_END: animation_selection

            // ANCHOR: on_update_closing_bracket_2
        }
        // ANCHOR_END: on_update_closing_bracket_2

        // ANCHOR: applying_animation
        if let Some(current_animation) = self.animations.get_mut(self.current_animation as usize) {
            current_animation.update(context.dt);

            if let Some(sprite) = context
                .scene
                .graph
                .try_get_mut(self.sprite)
                .and_then(|n| n.cast_mut::<Rectangle>())
            {
                // Set new frame to the sprite.
                sprite
                    .material()
                    .data_ref()
                    .bind("diffuseTexture", current_animation.texture());
                sprite.set_uv_rect(
                    current_animation
                        .current_frame_uv_rect()
                        .unwrap_or_default(),
                );
            }
        }
        // ANCHOR_END: applying_animation

        // ANCHOR: on_update_closing_bracket_1
    }
    // ANCHOR_END: on_update_closing_bracket_1
}
