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
    resource::texture::Texture,
    scene::transform::TransformBuilder,
    scene::dim2::rectangle::RectangleBuilder,
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

    // heart_texture: Handle<Texture>,
    // heart_sprites: Vec<Handle<Node>>,
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

fn bounding_boxes_intersect(a: &Node, b: &Node) -> bool {
    // Get global positions (2D)
    let a_pos = a.global_position().xy();
    let b_pos = b.global_position().xy();

    // Get bounding box half extents (width/2, height/2)
    // You need to define how to get size; here we assume you have a method or fixed size
    let a_size = get_node_size(a);
    let b_size = get_node_size(b);

    // AABB collision check
    (a_pos.x - b_pos.x).abs() < (a_size.x + b_size.x) / 2.0 &&
    (a_pos.y - b_pos.y).abs() < (a_size.y + b_size.y) / 2.0
}

// Helper function to get size of node's bounding box
fn get_node_size(node: &Node) -> Vector2<f32> {
    // Example: fixed size for all sprites, or you can extract from sprite/rectangle component
    Vector2::new(1.0, 1.0) // Replace with actual size logic
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
    game_over: bool,

    // ANCHOR: animation_fields
    animations: Vec<SpriteSheetAnimation>,
    current_animation: u32,
    // ANCHOR_END: animation_fields

    // after dying, the player will be respawned at this position and has a total of how many lives I decide upon
    health: f32,
    max_health: f32,
    health_fill_handle: Handle<Node>,  

    initial_position: Vector2<f32>,
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
            game_over: false,
            // ANCHOR: animation_fields_defaults_end
            // ...
            animations: Default::default(),
            current_animation: 0,

            max_health: 100.0,
            health: 100.0,
            health_fill_handle : Handle::NONE,
            initial_position: Vector2::new(0.0, 0.0), // Default to (0, 0)
        }
    }
}
// ANCHOR_END: animation_fields_defaults_end

impl Player {
    // ANCHOR: health_bar
    fn update_health_bar(&mut self, context: &mut ScriptContext) {
        // Safety: Only update if handles are valid
        if self.health_fill_handle.is_some() {
            let health_ratio = self.health / self.max_health;
            let full_width = 100.0; // Set to your bar's full width

            println!("Updating health bar: health_ratio = {}", health_ratio);

            if let Some(health_fill_rect) = context.scene.graph.try_get_mut(self.health_fill_handle).and_then(|n| n.cast_mut::<Rectangle>()) {
                let mut local_transform = health_fill_rect.local_transform_mut();
                local_transform.set_scale(Vector3::new(health_ratio, local_transform.scale().y, local_transform.scale().z));
                println!("New scale: {:?}", local_transform.scale());
            }

            // Optionally, position the bar on the left so sit's not centered
            if let Some(health_fill_rect) = context.scene.graph.try_get_mut(self.health_fill_handle).and_then(|n| n.cast_mut::<Rectangle>()) {
                let mut local_transform = health_fill_rect.local_transform_mut();
                local_transform.set_position(Vector3::new((full_width - health_ratio * full_width) / 200.0, local_transform.scale().y, local_transform.scale().z));
                println!("New position: {:?}", local_transform.position());
            }
        }
    }
    // ANCHOR_END: health_bar
}

impl ScriptTrait for Player { // Only for defining default values of fields and default methods/ Player's behaviour
    // ANCHOR: set_player_field
    fn on_start(&mut self, ctx: &mut ScriptContext) {
        ctx.plugins.get_mut::<Game>().player = ctx.handle;

        self.max_health = 100.0;
        self.health = self.max_health;

        // Debugging: Check if handles are set
        println!("Health Fill Handle: {:?}", self.health_fill_handle);

        // the handles for the health bar are set in the editor
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
                        PhysicalKey::Code(KeyCode::Space) if pressed => {
                            // Reduce health by 20 when space is pressed
                            self.health = (self.health - 20.0).max(0.0); // Ensure health doesn't go below 0
                        },
                        PhysicalKey::Code(KeyCode::KeyR) if pressed && self.game_over => {
                            // Reset health to max when R is pressed
                            self.health = self.max_health;
                            self.game_over = false; // Reset game over state
                            println!("Game Restarted! Health reset to {}", self.health);
                        },
                        PhysicalKey::Code(KeyCode::Escape) if pressed && self.game_over => {
                            // Exit the game when Escape is pressed
                            println!("Exiting game...");
                            std::process::exit(0);
                        },
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
        let player_handle = context.plugins.get::<Game>().player;


        if self.game_over {
            return; // Stop updating if the game is over
        }
    

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

        // ANCHOR: health_bar
        println!("Player health: {}", self.health);
        self.update_health_bar(context);

        if self.health <= 0.0 {
            self.game_over = true;
            println!("Game Over! Press R to Restart or Esc to Exit.");
            return;
        }

        // ANCHOR: on_update_closing_bracket_1
    }
    // ANCHOR_END: on_update_closing_bracket_1

}

