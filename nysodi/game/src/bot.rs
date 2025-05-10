// ANCHOR: imports
use crate::Game;
use crate::Player;
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::{SceneGraph},
    scene::{
        animation::spritesheet::SpriteSheetAnimation,
        dim2::{
            collider::Collider, rectangle::Rectangle, rigidbody::RigidBody,
        },
        node::Node,
        rigidbody::RigidBodyType,
    },
    script::{ScriptContext, ScriptTrait},
    event::{ElementState, Event, WindowEvent}, // Added imports
    keyboard::{KeyCode, PhysicalKey},         // Added imports
};
// ANCHOR_END: imports

#[derive(Visit, Reflect, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "d2786d36-a0af-4e67-916a-438af62f818b")]
#[visit(optional)]
pub struct Bot {
    // ANCHOR: visual_fields
    rectangle: InheritableVariable<Handle<Node>>,
    // ANCHOR_END: visual_fields

    // ANCHOR: movement_fields
    speed: InheritableVariable<f32>,
    direction: Vector2<f32>,
    front_obstacle_sensor: InheritableVariable<Handle<Node>>,
    back_obstacle_sensor: InheritableVariable<Handle<Node>>,
    // ANCHOR_END: movement_fields

    // ANCHOR: target_fields
    #[visit(skip)]
    #[reflect(hidden)]
    target: Handle<Node>,
    // ANCHOR_END: target_fields

    // ANCHOR: animation_fields
    animations: Vec<SpriteSheetAnimation>,
    current_animation: InheritableVariable<u32>,
    health: f32,
    max_health: f32,
    health_fill_handle: Handle<Node>,
    damage_timer: f32,
    respawn_timer: Option<f32>,
    pending_health_update: Option<f32>,
    // ANCHOR_END: animation_fields
}

// ANCHOR: bot_defaults
impl Default for Bot {
    fn default() -> Self {
        Self {
            speed: 1.0.into(),
            direction: Vector2::new(0.0, 0.0),
            front_obstacle_sensor: Default::default(),
            back_obstacle_sensor: Default::default(),
            target: Default::default(),
            rectangle: Default::default(),
            animations: Default::default(),
            current_animation: Default::default(),
            health: 100.0,
            max_health: 100.0,
            health_fill_handle: Handle::NONE,
            damage_timer: 0.0,
            respawn_timer: None,
            pending_health_update: None,

        }
    }
}
// ANCHOR_END: bot_defaults

// ANCHOR: has_ground_in_front
impl Bot {
    pub fn get_health(&self) -> f32 {
        self.health
    }

    pub fn set_health(&mut self, new_health: f32) {
        self.pending_health_update = Some(new_health);
    }
    fn update_health_bar(&mut self, context: &mut ScriptContext) {
        if self.health_fill_handle.is_some() {
            let health_ratio = self.health / self.max_health;
            let full_width = 100.0; // Set to your bar's full width

            if let Some(health_fill_rect) = context.scene.graph.try_get_mut(self.health_fill_handle).and_then(|n| n.cast_mut::<Rectangle>()) {
                let mut local_transform = health_fill_rect.local_transform_mut();
                local_transform.set_scale(Vector3::new(health_ratio, local_transform.scale().y, local_transform.scale().z));
            }

            if let Some(health_fill_rect) = context.scene.graph.try_get_mut(self.health_fill_handle).and_then(|n| n.cast_mut::<Rectangle>()) {
                let mut local_transform = health_fill_rect.local_transform_mut();
                local_transform.set_position(Vector3::new((full_width - health_ratio * full_width) / 200.0, local_transform.position().y, local_transform.position().z));
            }
        }
    }

    fn locate_target(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        self.target = game.player;
    }

    fn move_to_target(&mut self, ctx: &mut ScriptContext) {
        // 2D chase towards player
        let tp = ctx.scene.graph[self.target].global_position().xy();
        let sp = ctx.scene.graph[ctx.handle].global_position().xy();
        let delta = tp - sp;
        let dist = (delta.x.powi(2) + delta.y.powi(2)).sqrt();
        if dist > 1.1 {
            self.direction = delta / dist;
            self.speed.set_value_and_mark_modified(1.2);
        } else {
            self.direction = Vector2::new(0.0, 0.0);
            self.speed.set_value_and_mark_modified(0.0);
        }
    }
    // ANCHOR_END: search_target

    /// Apply velocity to the bot's RigidBody2D and flip sprite to always face player
    fn do_move(&mut self, ctx: &mut ScriptContext) {
        // Set movement velocity
        if let Some(rb) = ctx.scene.graph.try_get_mut_of_type::<RigidBody>(ctx.handle) {
            let vel = Vector2::new(
                self.direction.x * *self.speed,
                self.direction.y * *self.speed,
            );
            rb.set_lin_vel(vel);
        }
        // Compute direction to face player
        let tp_x = ctx.scene.graph[self.target].global_position().x;
        let sp_x = ctx.scene.graph[ctx.handle].global_position().x;
        let flip = (tp_x - sp_x).signum();
        // Invert flip if sprite's default orientation is opposite
        let scale_x = -flip;
        // Apply sprite flip
        if let Some(rect_node) = ctx.scene.graph.try_get_mut(*self.rectangle) {
            rect_node.local_transform_mut().set_scale(Vector3::new(
                2.0 * scale_x,
                2.0,
                1.0,
            )); 
        }
    }
    // ANCHOR_END: do_move

    // ANCHOR: has_obstacles
    fn has_obstacles(&mut self, ctx: &mut ScriptContext) -> bool {
        let graph = &ctx.scene.graph;

        // Select the sensor using current walking direction.
        let sensor_handle = if self.direction.x < 0.0 {
            *self.back_obstacle_sensor
        } else {
            *self.front_obstacle_sensor
        };

        // Check if it intersects something.
        let Some(obstacle_sensor) = graph.try_get_of_type::<Collider>(sensor_handle) else {
            return false;
        };

        for intersection in obstacle_sensor
            .intersects(&ctx.scene.graph.physics2d)
            .filter(|i| i.has_any_active_contact)
        {
            for collider_handle in [intersection.collider1, intersection.collider2] {
                let Some(other_collider) = graph.try_get_of_type::<Collider>(collider_handle)
                else {
                    continue;
                };

                let Some(rigid_body) = graph.try_get_of_type::<RigidBody>(other_collider.parent())
                else {
                    continue;
                };

                if rigid_body.body_type() == RigidBodyType::Static {
                    return true;
                }
            }
        }

        false
    }
    // ANCHOR_END: has_obstacles
}

impl ScriptTrait for Bot {
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        // Apply pending health update if any
        if let Some(new_health) = self.pending_health_update.take() {
            self.health = new_health;
            self.update_health_bar(ctx);
            if self.health <= 0.0 {
                if let Some(node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                    node.set_visibility(false); // Make the bot invisible
                }
                println!("Bot defeated!");
            }
        }
        
        // If the bot is defeated, start the respawn timer
        if self.health <= 0.0 {
            if let Some(timer) = &mut self.respawn_timer {
                *timer += ctx.dt; // Increment the respawn timer
                if *timer >= 3.0 {
                    // Respawn the bot after 3 seconds
                    self.health = self.max_health;
                    self.respawn_timer = None; // Reset the timer
                    if let Some(node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                        node.set_visibility(true); // Make the bot visible again
                    }
                    println!("Bot respawned!");
                }
            } else {
                // Start the respawn timer if it hasn't started yet
                self.respawn_timer = Some(0.0);
            }
            return; // Skip the rest of the update logic while the bot is defeated
        }

        self.locate_target(ctx);
        self.move_to_target(ctx);
        self.do_move(ctx);

        // Update the bot's health bar
        self.update_health_bar(ctx);

        // Check if the bot is within a 1-tile radius of the player
        let player_position = ctx.scene.graph[self.target].global_position().xy();
        let bot_position = ctx.scene.graph[ctx.handle].global_position().xy();
        let distance = (player_position - bot_position).norm();

        if distance <= 1.5 {
            // Increment the damage timer
            self.damage_timer += ctx.dt;
        
            // Deal damage every second
            if self.damage_timer >= 0.75 {
                // Reduce the player's health by 20
                let game = ctx.plugins.get::<Game>();
                if let Some(player) = ctx.scene.graph.try_get_mut(game.player) {
                    if let Some(player_script) = player
                        .script_mut(0)
                        .and_then(|s| s.cast_mut::<Player>()) 
                    {
                        // Skip damage logic if the player is already defeated
                        if player_script.game_over {
                            return;
                        }
        
                        player_script.health = (player_script.health - 20.0).max(0.0);
                        println!("Player took damage! Health: {}", player_script.health);
        
                        // Check if the player is dead
                        if player_script.health <= 0.0 {
                            println!("Player defeated!");
                            player_script.game_over = true;
                        }
                    }
                }
        
                // Reset the damage timer
                self.damage_timer = 0.0;
            }
        } else {
            // Reset the damage timer if the bot is not within range
            self.damage_timer = 0.0;
        }

        if self.direction.x.abs() > 0.0 || self.direction.y.abs() > 0.0 {
            self.current_animation.set_value_and_mark_modified(2);
        } else {
            self.current_animation.set_value_and_mark_modified(0);
        }

        if let Some(anim) = self.animations.get_mut(*self.current_animation as usize) {
            anim.update(ctx.dt);
            if let Some(rect) = ctx.scene.graph.try_get_mut(*self.rectangle)
                .and_then(|n| n.cast_mut::<Rectangle>())
            {
                rect.material().data_ref().bind("diffuseTexture", anim.texture());
                rect.set_uv_rect(anim.current_frame_uv_rect().unwrap_or_default());
            }
        }
    }

    fn on_os_event(&mut self, event: &Event<()>, ctx: &mut ScriptContext) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { event, .. } = event {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ShiftLeft) | PhysicalKey::Code(KeyCode::ShiftRight) if pressed => {
                            // Check if the player is within a 2-tile radius
                            let player_position = ctx.scene.graph[self.target].global_position().xy();
                            let bot_position = ctx.scene.graph[ctx.handle].global_position().xy();
                            let distance = (player_position - bot_position).norm();

                            if distance <= 2.0 {
                                // Reduce bot's health by 10
                                self.health = (self.health - 10.0).max(0.0);
                                println!("Bot took damage! Health: {}", self.health);

                                // Update the health bar
                                self.update_health_bar(ctx);

                                // Check if the bot is dead
                                if self.health <= 0.0 {
                                    println!("Bot defeated!");
                                    // Optionally, deactivate the bot or trigger a death animation
                                    if let Some(node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                                        node.set_visibility(false);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}