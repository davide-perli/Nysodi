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
        task::TaskPool,
        type_traits::prelude::*,
        visitor::prelude::*,
        math::Rect,
    },
    engine::ScriptProcessor,
    event::{ElementState, Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::{
        animation::spritesheet::SpriteSheetAnimation,
        base::BaseBuilder,
        dim2::{
            collider::{Collider, ColliderBuilder, ColliderShape, CuboidShape},
            rectangle::{Rectangle, RectangleBuilder},
            rigidbody::{RigidBody, RigidBodyBuilder},
        },
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
    script::{ScriptContext, ScriptTrait},
    rand::{self, Rng},
    material::{Material, MaterialResource, MaterialResourceExtension},
    gui::{
        texture::{Texture, TextureResource},
        widget::WidgetBuilder,
    },
    asset::manager::ResourceManager,
};
use std::{path::Path, sync::Arc};
// ANCHOR_END: imports

#[derive(Visit, Reflect, Debug, Default)]
pub struct Game {
    scene: Handle<Scene>,

    // ANCHOR: player_field
    player: Handle<Node>,
    // ANCHOR_END: player_field
    pub total_score: f32,
}

// ANCHOR: register
impl Plugin for Game {
    fn register(&self, context: PluginRegistrationContext) {
        let script_constructors = &context.serialization_context.script_constructors;
        script_constructors.add::<Player>("Player");
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
    game_over: bool,

    animations: Vec<SpriteSheetAnimation>,
    current_animation: u32,

    health: f32,
    max_health: f32,
    health_fill_handle: Handle<Node>,

    initial_position: Vector2<f32>,

    item_timer: Option<f32>,
    bomb_timer: f32,
    last_health: f32,
    heart_pulse_timer: f32,
    explosion_timer: Option<f32>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            sprite: Handle::NONE,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
            game_over: false,
            animations: Default::default(),
            current_animation: 0,
            max_health: 100.0,
            health: 100.0,
            health_fill_handle: Handle::NONE,
            initial_position: Vector2::new(0.0, 0.0),
            item_timer: None,
            bomb_timer: 0.0,
            last_health: 100.0,
            heart_pulse_timer: 0.0,
            explosion_timer: None,
        }
    }
}

impl Player {
    fn spawn_heart(&self, context: &mut ScriptContext) -> Handle<Node> {
        let player_position = context.scene.graph[self.sprite]
            .global_position()
            .xy();

        let mut rng = rand::thread_rng();
        let offset_x: f32 = rng.gen_range(-5.0..=5.0);
        let offset_y: f32 = rng.gen_range(-5.0..=5.0);
        let mut heart_position = Vector2::new(player_position.x + offset_x, player_position.y + offset_y);
        heart_position.x = heart_position.x.clamp(-11.0, 11.0);
        heart_position.y = heart_position.y.clamp(-4.0, 17.0);

        let heart_texture = context.resource_manager.request::<Texture>("data/heart.png");

        let heart = RectangleBuilder::new(
            BaseBuilder::new()
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(heart_position.x, heart_position.y, 0.0))
                        .with_local_scale(Vector3::new(0.7, 0.7, 0.7))
                        .build(),
                ),
        )
        .build(&mut context.scene.graph);

        if let Some(rectangle) = context.scene.graph.try_get_mut(heart).and_then(|n| n.cast_mut::<Rectangle>()) {
            let material = rectangle.material();
            material.data_ref().bind("diffuseTexture", heart_texture);
        }

        println!("Heart spawned at: {:?}", heart_position);

        heart
    }

    fn spawn_item(&self, context: &mut ScriptContext) -> (Handle<Node>, Vector2<f32>) {
        let player_position = context.scene.graph[self.sprite]
            .global_position()
            .xy();

        let mut rng = rand::thread_rng();
        let offset_x: f32 = rng.gen_range(-5.0..=5.0);
        let offset_y: f32 = rng.gen_range(-5.0..=5.0);
        let mut item_position = Vector2::new(player_position.x + offset_x, player_position.y + offset_y);
        item_position.x = item_position.x.clamp(-11.0, 11.0);
        item_position.y = item_position.y.clamp(-4.0, 17.0);

        let bomb_texture = context.resource_manager.request::<Texture>("data/bomb.png");

        let bomb = RectangleBuilder::new(
            BaseBuilder::new()
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(item_position.x, item_position.y, 0.0))
                        .with_local_scale(Vector3::new(0.7, 0.7, 0.7))
                        .build(),
                ),
        )
        .build(&mut context.scene.graph);

        if let Some(rectangle) = context.scene.graph.try_get_mut(bomb).and_then(|n| n.cast_mut::<Rectangle>()) {
            let material = rectangle.material();
            material.data_ref().bind("diffuseTexture", bomb_texture);
        }

        println!("Bomb spawned at: {:?}", item_position);

        (bomb, item_position)
    }

    fn update_health_bar(&mut self, context: &mut ScriptContext) {
        if self.health_fill_handle.is_some() {
            let health_ratio = self.health / self.max_health;
            let full_width = 100.0;

            if let Some(health_fill_rect) = context.scene.graph.try_get_mut(self.health_fill_handle).and_then(|n| n.cast_mut::<Rectangle>()) {
                let mut local_transform = health_fill_rect.local_transform_mut();
                local_transform.set_scale(Vector3::new(health_ratio, local_transform.scale().y, local_transform.scale().z));
            }

            if let Some(health_fill_rect) = context.scene.graph.try_get_mut(self.health_fill_handle).and_then(|n| n.cast_mut::<Rectangle>()) {
                let mut local_transform = health_fill_rect.local_transform_mut();
                local_transform.set_position(Vector3::new((full_width - health_ratio * full_width) / 200.0, local_transform.scale().y, local_transform.scale().z));
            }
        }
    }
}

impl ScriptTrait for Player {
    fn on_start(&mut self, ctx: &mut ScriptContext) {
        ctx.plugins.get_mut::<Game>().player = ctx.handle;

        self.max_health = 100.0;
        self.health = self.max_health;
    }

    fn on_os_event(&mut self, event: &Event<()>, context: &mut ScriptContext) {
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
                            // Reset the player's position to the starting point
                            if let Some(node) = context.scene.graph.try_get_mut(context.handle) {
                                node.local_transform_mut().set_position(Vector3::new(
                                    self.initial_position.x - 1.0,
                                    self.initial_position.y - 4.0,
                                    0.0,
                                ));
                            }
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

    fn on_update(&mut self, context: &mut ScriptContext) {
        self.update_health_bar(context);

        if self.health <= 0.0 {
            self.game_over = true;
            println!("Game Over! Press R to Restart or Esc to Exit.");
            return;
        }

        if self.game_over {
            return;
        }

        // Animate the heart's pulsing effect
        self.heart_pulse_timer += context.dt;
        let pulse_scale = 0.7 + 0.1 * (self.heart_pulse_timer * 3.0).sin(); // Oscillates between 0.6 and 0.8
        // Animate the bomb's pulsing effect
        let bomb_pulse_scale = 0.7 + 0.05 * (self.heart_pulse_timer * 5.0).sin(); // Oscillates between 0.65 and 0.75


        let heart_handle = context
            .scene
            .graph
            .pair_iter_mut()
            .find(|(_, node)| node.name() == "Heart" && node.visibility())
            .map(|(handle, _)| handle);

        if let Some(hh) = heart_handle {
            let player_pos = context.scene.graph[self.sprite].global_position().xy();
            let heart_pos = context.scene.graph[hh].global_position().xy();

            if let Some(heart_node) = context.scene.graph.try_get_mut(hh) {
                heart_node
                    .local_transform_mut()
                    .set_scale(Vector3::new(pulse_scale, pulse_scale, pulse_scale));
            }

            if (player_pos - heart_pos).norm() < 1.0 {
                if let Some(node) = context.scene.graph.try_get_mut(hh) {
                    node.set_visibility(false);
                }
                self.health = (self.health + 30.0).min(self.max_health);
                self.item_timer = None;
                self.last_health = self.health;
                println!("Heart collected! Health: {}", self.health);
            }
        } else if self.health < 50.0 && self.item_timer.is_none() {
            let heart = self.spawn_heart(context);
            context.scene.graph[heart].set_name("Heart");
            context.scene.graph[heart].set_visibility(true);
            self.item_timer = Some(0.0);
        }

        if let Some(timer) = &mut self.item_timer {
            *timer += context.dt;

            if *timer >= 5.0 {
                let heart_handle = context
                    .scene
                    .graph
                    .pair_iter_mut()
                    .find(|(_, node)| node.name() == "Heart" && node.visibility())
                    .map(|(handle, _)| handle);

                if let Some(heart) = heart_handle {
                    if let Some(node) = context.scene.graph.try_get_mut(heart) {
                        node.set_visibility(false);
                    }
                }

                self.item_timer = None;
            }
        }

        self.bomb_timer += context.dt;

        if self.bomb_timer >= 30.0 {
            let (bomb, _) = self.spawn_item(context);
            context.scene.graph[bomb].set_name("Bomb");
            context.scene.graph[bomb].set_visibility(true);

            self.bomb_timer = 0.0;
        }

        let bomb_handle = context
            .scene
            .graph
            .pair_iter_mut()
            .find(|(_, node)| node.name() == "Bomb" && node.visibility())
            .map(|(handle, _)| handle);

        if let Some(bh) = bomb_handle {
            let player_pos = context.scene.graph[self.sprite].global_position().xy();
            let bomb_pos = context.scene.graph[bh].global_position().xy();

            // Only pulse if we're not in the middle of an explosion
            if self.explosion_timer.is_none() {
                let bomb_pulse_scale = 0.7 + 0.05 * (self.heart_pulse_timer * 5.0).sin();
                if let Some(bomb_node) = context.scene.graph.try_get_mut(bh) {
                    bomb_node
                        .local_transform_mut()
                        .set_scale(Vector3::new(bomb_pulse_scale, bomb_pulse_scale, bomb_pulse_scale));
                }
            }


            if (player_pos - bomb_pos).norm() < 1.0 {
                if self.explosion_timer.is_none() {
                    println!("Bomb exploded!");
            
                    // Change the bomb's texture to explosion.png
                    let explosion_texture = context.resource_manager.request::<Texture>("data/explosion.png");
                    if let Some(bomb_node) = context.scene.graph.try_get_mut(bh).and_then(|n| n.cast_mut::<Rectangle>()) {

                        let material = bomb_node.material();
                        material.data_ref().bind("diffuseTexture", explosion_texture);
                        
                        // Adjust the scale of the rectangle to make the texture appear larger
                        bomb_node
                        .local_transform_mut()
                        .set_scale(Vector3::new(3.5, 3.5, 1.0)); // Scale the rectangle (3x larger)


                    println!("Explosion scale set to: {:?}, UV rect set to full texture.", bomb_node.local_transform().scale());
                    
                    }
            
                    // Start the explosion timer
                    self.explosion_timer = Some(0.5);
            
                    // Damage bots within the explosion radius
                    let explosion_radius = 6.0; // Adjust radius as needed
                    let bots_to_hit: Vec<_> = context
                        .scene
                        .graph
                        .pair_iter_mut()
                        .filter_map(|(h, node)| {
                            let bot_pos = node.global_position().xy();
            
                            if let Some(bot_script) = node.script_mut(0).and_then(|s| s.cast_mut::<Bot>()) {
                                let distance = (bot_pos - bomb_pos).norm();
                                if distance <= explosion_radius {
                                    println!("Bot found at position: {:?}, distance: {}", bot_pos, distance);
                                    return Some((h, bot_script, distance));
                                }
                            }
                            None
                        })
                        .collect();
            
                    for (bot_h, bot_script, distance) in bots_to_hit {
                        let damage = if distance <= 3.0 {
                            100.0
                        } else if distance <= 4.0 {
                            70.0
                        } else if distance <= 5.0 {
                            40.0
                        } else if distance <= 6.0 {
                            10.0
                        } else {
                            0.0
                        };

                        let new_health = (bot_script.get_health() - damage).max(0.0);
                        bot_script.set_health(new_health);
                        println!(
                            "Bot at distance {:.2} damaged by bomb! Damage: {}, Remaining health: {}",
                            distance, damage, new_health
                        );
                    }
                }
            }
            
            // Check if the explosion timer is active and update it
            if let Some(timer) = &mut self.explosion_timer {
                *timer -= context.dt;
                if *timer <= 0.0 {
                    // Hide the bomb after the timer expires
                    context.scene.graph[bh].set_visibility(false);
                    self.explosion_timer = None; // Reset the timer
                }
            }
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
            // for sure what's the value of the sprite field. It can be unassigned and the following code won't
            // execute. A simple context.scene.graph[self.sprite] would just panicked in this case.
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
        // println!("Player health: {}", self.health);
        self.update_health_bar(context);

        if self.health <= 0.0 {
            self.game_over = true;
            println!("Game Over! Press R to Restart or Esc to Exit.");
            return;
        }

        // ANCHOR: on_update_closing
        
    }
}