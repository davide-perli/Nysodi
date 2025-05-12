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
    graph::{SceneGraph, BaseSceneGraph},
    scene::{
        transform::TransformBuilder,
        base::BaseBuilder,
        animation::spritesheet::SpriteSheetAnimation,
        dim2::{
            collider::Collider, rigidbody::RigidBody,
            rectangle::{Rectangle, RectangleBuilder},
        },
        node::Node,
        rigidbody::RigidBodyType,
    },
    script::{ScriptContext, ScriptTrait},
    event::{ElementState, Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    gui::texture::Texture,
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

    reaction_timer: f32,
    reaction_state: ReactionState,
    has_reacted: bool,
    // ANCHOR_END: animation_fields

    target_handle: Option<Handle<Node>>,
    target_sprite_timer: f32,
}

#[derive(Visit, Reflect, Debug, Clone, Copy)]
enum ReactionState {
    Motionless,
    RunningAway,
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
            reaction_state: ReactionState::Motionless,
            reaction_timer: 0.0,
            has_reacted: false,
            target_handle: None,
            target_sprite_timer: 0.0,
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

    pub fn set_health_fill_handle(&mut self, handle: Handle<Node>) {
        self.health_fill_handle = handle;
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

    pub fn trigger_reaction(&mut self, total_score: f32) {
        if total_score < 80.0 {
            self.reaction_state = ReactionState::Motionless;
        } else {
            self.reaction_state = ReactionState::RunningAway;
        }
        self.reaction_timer = 3.0;
    }

    pub fn set_animations(&mut self, animations: Vec<SpriteSheetAnimation>) {
        self.animations = animations;
    }

    fn locate_target(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        self.target = game.player;
    }

    fn move_to_target(&mut self, ctx: &mut ScriptContext) {
        // Calculate the target position and the bot's position
        let tp = ctx.scene.graph[self.target].global_position().xy();
        let sp = ctx.scene.graph[ctx.handle].global_position().xy();
        let delta = tp - sp;
        let dist = (delta.x.powi(2) + delta.y.powi(2)).sqrt();

        // Adjust direction and speed based on distance
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

    fn spawn_target_sprite(&mut self, ctx: &mut ScriptContext) -> Handle<Node> {
        self.target_sprite_timer = f32::EPSILON; // Start the timer

        // Check if the target sprite already exists
        if let Some(prev_target) = self.target_handle.take() {
            if let Some(prev_node) = ctx.scene.graph.try_get_mut(prev_target) {
                ctx.scene.graph.remove_node(prev_target);
                println!("▶ Previous target sprite removed.");
            }
        }
        // Get the skeleton's current position (the target's position)
        let skeleton_position = ctx.scene.graph[ctx.handle].global_position().xy();
        let mut target_position = Vector2::new(skeleton_position.x, skeleton_position.y);

        // Request the texture for the target sprite
        let target_texture = ctx.resource_manager.request::<Texture>("data/target_img.png");

        // Create the target sprite at the calculated position
        let target_sprite = RectangleBuilder::new(
            BaseBuilder::new()
                .with_name("TargetItem")
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(target_position.x, target_position.y, 0.0))
                        .with_local_scale(Vector3::new(0.7, 0.7, 0.7))
                        .build(),
                ),
        )
        .build(&mut ctx.scene.graph);

        // Bind the texture to the sprite
        if let Some(rectangle) = ctx.scene.graph.try_get_mut(target_sprite).and_then(|n| n.cast_mut::<Rectangle>()) {
            let material = rectangle.material();
            material.data_ref().bind("diffuseTexture", target_texture);
        }

        if let Some(bot_node) = ctx.scene.graph.try_get_mut(self.target) {
            println!("▶ Target sprite spawned for {}, at position: {:?}", bot_node.name(), target_position);
        }

        target_sprite
    }
}

impl ScriptTrait for Bot {
    fn on_start(&mut self, ctx: &mut ScriptContext) {
        // Locate the player as the target
        self.locate_target(ctx);

        // Initialize health bar or other visual elements if needed
        self.update_health_bar(ctx);

<<<<<<< HEAD
        println!("Bot initialized with target: {:?}", self.target);
=======
        if let Some(bot_node) = ctx.scene.graph.try_get_mut(ctx.handle) {
            println!("▶ {} initialized with target: {:?}", bot_node.name(), self.target);
        }
>>>>>>> 12406420b884bd53db690eec2dc528f19bb8f373
    }
    
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        // 0) Always update target first
        self.locate_target(ctx);

        // 1) Pending health update & respawn
        if let Some(new_health) = self.pending_health_update.take() {
            self.health = new_health;
            self.update_health_bar(ctx);

            if self.health <= 0.0 {
                // Respawn timer
                if self.respawn_timer.is_none() {
                    // Award points and hide the bot only once
                    ctx.plugins.get_mut::<Game>().total_score += 10.0;
                    if let Some(bot_node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                        println!(
                            "▶ {} defeated! +10 points — total_score = {}",
                            bot_node.name(),
                            ctx.plugins.get::<Game>().total_score
                        );
                    }
            
                    if let Some(n) = ctx.scene.graph.try_get_mut(ctx.handle) {
                        n.set_visibility(false);
                    }
            
                    // Initialize the respawn timer
                    self.respawn_timer = Some(0.0);
                } else {
                    // Increment the respawn timer
                    if let Some(t) = &mut self.respawn_timer {
                        *t += ctx.dt;
                        if *t >= 3.0 {
                            self.health = self.max_health;
                            self.respawn_timer = None;
                            self.has_reacted = false; // Reset reaction state
                            if let Some(n) = ctx.scene.graph.try_get_mut(ctx.handle) {
                                n.set_visibility(true);
                            }
                            if let Some(bot_node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                                println!("▶ {} respawned!", bot_node.name());
                            }

                            self.update_health_bar(ctx);
                        }
                    }
                }
                return;
            }
        }

        if self.health <= 0.0 {
            if let Some(prev_target) = self.target_handle.take() {
                if let Some(prev_node) = ctx.scene.graph.try_get_mut(prev_target) {
                    ctx.scene.graph.remove_node(prev_target);
                    println!("▶ Previous target sprite removed.");
                }
            }
            // Respawn timer
            if let Some(t) = &mut self.respawn_timer {
                *t += ctx.dt;
                if *t >= 3.0 {
                    self.health = self.max_health;
                    self.respawn_timer = None;
                    self.has_reacted = false; // Reset reaction state
                    if let Some(n) = ctx.scene.graph.try_get_mut(ctx.handle) {
                        n.set_visibility(true);
                    }
                    if let Some(bot_node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                        println!("▶ {} respawned!", bot_node.name());
                    }
                    self.update_health_bar(ctx);
                }
            } else {
                self.respawn_timer = Some(0.0);
            }
            return;
        }

        // 2) Trigger reaction once when score > 50
        let total_score = ctx.plugins.get::<Game>().total_score;

        if !self.has_reacted && total_score > 50.0 && self.reaction_timer <= 0.0 {
            self.has_reacted = true;
            self.trigger_reaction(total_score);
            if let Some(bot_node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                println!(
                    "▶ Reaction triggered for {}: {:?} for 3s",
                    bot_node.name(),
                    self.reaction_state
                );
            }

        }

        if self.reaction_timer > 0.0 {
            self.reaction_timer -= ctx.dt;
            if self.reaction_timer > 0.0 {
                let me = ctx.scene.graph[ctx.handle].global_position().xy();
                let them = ctx.scene.graph[self.target].global_position().xy();
                match self.reaction_state {
                    ReactionState::Motionless => {
                        self.direction = Vector2::zeros();
                        self.speed.set_value_and_mark_modified(0.0);
                    }
                    ReactionState::RunningAway => {
                        self.direction = (me - them).normalize();
                        self.speed.set_value_and_mark_modified(2.0);
                    }
                }
                self.do_move(ctx);
                return;
            }
        }

        // 4) Normal chase & move
        self.move_to_target(ctx);
        self.do_move(ctx);

        // 5) Damage on contact
        self.update_health_bar(ctx);
        let player_pos = ctx.scene.graph[self.target].global_position().xy();
        let bot_pos = ctx.scene.graph[ctx.handle].global_position().xy();
        let dist = (player_pos - bot_pos).norm();
        if dist <= 1.5 {
            self.damage_timer += ctx.dt;
            if self.damage_timer >= 0.75 {
                if let Some(pn) = ctx.scene.graph.try_get_mut(ctx.plugins.get::<Game>().player) {
                    if let Some(ps) = pn.script_mut(0).and_then(|s| s.cast_mut::<Player>()) {
                        if !ps.game_over {
                            ps.health = (ps.health - 20.0).max(0.0);
                            println!("▶ Player hit! Health = {}", ps.health);
                            if ps.health <= 0.0 {
                                ps.game_over = true;
                                println!("▶ Player defeated!");
                            }
                        }
                    }
                }
                self.damage_timer = 0.0;
            }
        } else {
            self.damage_timer = 0.0;
        }

        // 6) Animation update
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

        // 7) Check for target item for the current self bot instance
        let bot_position = ctx.scene.graph[ctx.handle].global_position();

        let target_item_handle = ctx
            .scene
            .graph
            .pair_iter_mut()
            .find(|(_, node)| {
                node.name() == "TargetItem"
                    && node.visibility()
                    && (node.global_position() - bot_position).norm_squared() < f32::EPSILON
            })
            .map(|(handle, _)| handle);
        if let Some(target_handle) = target_item_handle {
            // If the target item exists, check its position and update or show it as necessary
            let bot_pos = ctx.scene.graph[ctx.handle].global_position().xy();
            
            // Update target item's position
            if let Some(target_node) = ctx.scene.graph.try_get_mut(target_handle) {
                // Update position to match the bot's position, you can adjust this as needed
                target_node.local_transform_mut().set_position(Vector3::new(bot_pos.x, bot_pos.y, 0.0));
            }
        }
        if self.target_sprite_timer > 0.0 {
            self.target_sprite_timer += ctx.dt;
            if self.target_sprite_timer >= 0.1 {
                if let Some(target) = self.target_handle {
                    if let Some(target_node) = ctx.scene.graph.try_get_mut(target) {
                        target_node.set_visibility(false);
                        println!("▶ Target sprite hidden after 0.1s");
                    }
                }
                self.target_sprite_timer = 0.0;
            }
        }
        
        // 8) Check for target item nodes in the scene graph
        // This is a debug print to check how many target item nodes are in the scene graph
        let target_count = ctx.scene.graph.pair_iter_mut()
            .filter(|(_, node)| node.name() == "TargetItem")
            .count();

        println!("▶ Number of target item nodes in scene graph: {}", target_count);

    }


    fn on_os_event(&mut self, event: &Event<()>, ctx: &mut ScriptContext) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::KeyboardInput { event, .. } = event {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    let released = event.state == ElementState::Released;

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::ShiftLeft) | PhysicalKey::Code(KeyCode::ShiftRight) if pressed => {
                            // Check if the player is within a 2-tile radius
                            let player_position = ctx.scene.graph[self.target].global_position().xy();
                            let bot_position = ctx.scene.graph[ctx.handle].global_position().xy();
                            let distance = (player_position - bot_position).norm();


                            if distance <= 2.0 {
                                let new_h = (self.health - 10.0).max(0.0);
                                self.set_health(new_h);                         // <<< enqueue the change
                                if let Some(bot_node) = ctx.scene.graph.try_get_mut(ctx.handle) {
                                    println!(
                                        "▶ {} took damage! Pending health = {}",
                                        bot_node.name(),
                                        new_h
                                    );
                                    if let Some(target) = &self.target_handle {
                                        if let Some(target_node) = ctx.scene.graph.try_get_mut(*target) {
                                            target_node.set_visibility(true);
                                            println!("Target sprite visible at position: {:?}", target_node.global_position().xy());
                                        }
                                    }
                                }

                                let bot_position = ctx.scene.graph[ctx.handle].global_position();

                                let target_item_handle = ctx
                                    .scene
                                    .graph
                                    .pair_iter_mut()
                                    .find(|(_, node)| {
                                        node.name() == "TargetItem"
                                            && node.visibility()
                                            && (node.global_position() - bot_position).norm_squared() < f32::EPSILON
                                    })
                                    .map(|(handle, _)| handle);

                                // If there is no existing target item, create one
                                if target_item_handle.is_none() {
                                    // Create target item sprite (similar to spawn_target_sprite function)
                                    let target_item = self.spawn_target_sprite(ctx);
                                    self.target_handle = Some(target_item);
                                    println!("▶ Target item spawned at position: {:?}", ctx.scene.graph[target_item].global_position().xy());
                                }
                            }
                            if released {
                                // Shift is released — delete the target node
                                if let Some(target) = self.target_handle.take() {
                                    if let Some(target_node) = ctx.scene.graph.try_get_mut(target) {
                                        ctx.scene.graph.remove_node(target);
                                        println!("▶ Previous target sprite removed.");
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