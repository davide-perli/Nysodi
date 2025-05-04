// ANCHOR: imports
use crate::Game;
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::{BaseSceneGraph, SceneGraph},
    scene::{
        animation::spritesheet::SpriteSheetAnimation,
        dim2::{
            collider::Collider, rectangle::Rectangle, rigidbody::RigidBody,
        },
        node::Node,
        rigidbody::RigidBodyType,
    },
    script::{ScriptContext, ScriptTrait},
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
        }
    }
}
// ANCHOR_END: bot_defaults

// ANCHOR: has_ground_in_front
impl Bot {
    fn locate_target(&mut self, ctx: &mut ScriptContext) {
        let game = ctx.plugins.get::<Game>();
        self.target = game.player;
    }
    // ANCHOR: search_target
    // fn search_target(&mut self, ctx: &mut ScriptContext) {
    //     let game = ctx.plugins.get::<Game>();
    
    //     let self_position = ctx.scene.graph[ctx.handle].global_position();
    
    //     let Some(player) = ctx.scene.graph.try_get(game.player) else {
    //         return;
    //     };
    
    //     let player_position = player.global_position();
    
    //     let dx = player_position.x - self_position.x;
    //     let dy = player_position.y - self_position.y;
    
    //     if dx.abs() < 3.0 && dy.abs() < 3.0 {
    //         self.target = game.player;
    
    //         // Set direction for both axes
    //         self.direction = Vector2::new(dx.signum(), dy.signum());
    //     }
    // }
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

    // ANCHOR: do_move
    // fn do_move(&mut self, ctx: &mut ScriptContext) {
    //     let Some(rigid_body) = ctx.scene.graph.try_get_mut_of_type::<RigidBody>(ctx.handle) else {
    //         return;
    //     };
    
    //     let speed = *self.speed; // Sau self.speed.get_value()
    //     let velocity = Vector2::new(
    //         speed * self.direction.x,
    //         speed * self.direction.y,
    //     );
    
    //     rigid_body.set_lin_vel(velocity);
    
    //     if let Some(rectangle) = ctx.scene.graph.try_get_mut(*self.rectangle) {
    //         rectangle.local_transform_mut().set_scale(Vector3::new(
    //             2.0 * self.direction.x.signum(),
    //             2.0,
    //             1.0,
    //         ));
    //     }
    // }
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
    // ANCHOR: search_target_call
    fn on_update(&mut self, ctx: &mut ScriptContext) {
        self.locate_target(ctx);
        // ANCHOR_END: search_target_call

        // ANCHOR: check_for_obstacles
        if self.has_obstacles(ctx) {
            self.direction.x = -self.direction.x;
        }
        // ANCHOR_END: check_for_obstacles

        // ANCHOR: move_to_target
        self.move_to_target(ctx);
        // ANCHOR_END: move_to_target

        // ANCHOR: do_move_call
        self.do_move(ctx);
        // ANCHOR_END: do_move_call

        // ANCHOR: animation_switching
        // if self.direction != Vector2::zeros() {
        //     self.current_animation.set_value_and_mark_modified(2);
        // }
        // if self.target.is_some() {
        //     let target_position = ctx.scene.graph[self.target].global_position();
        //     let self_position = ctx.scene.graph[ctx.handle].global_position();
        //     if target_position.metric_distance(&self_position) < 1.1 {
        //         self.current_animation.set_value_and_mark_modified(0);
        //     }
        // }
        if self.direction.x.abs() > 0.0 || self.direction.y.abs() > 0.0 {
            self.current_animation.set_value_and_mark_modified(2);
        } else {
            self.current_animation.set_value_and_mark_modified(0);
        }
        // ANCHOR_END: animation_switching

        // ANCHOR: applying_animation
        // if let Some(current_animation) = self.animations.get_mut(*self.current_animation as usize) {
        //     current_animation.update(ctx.dt);

        //     if let Some(sprite) = ctx
        //         .scene
        //         .graph
        //         .try_get_mut_of_type::<Rectangle>(*self.rectangle)
        //     {
        //         // Set new frame to the sprite.
        //         sprite
        //             .material()
        //             .data_ref()
        //             .bind("diffuseTexture", current_animation.texture());
        //         sprite.set_uv_rect(
        //             current_animation
        //                 .current_frame_uv_rect()
        //                 .unwrap_or_default(),
        //         );
        //     }
        // }
        if let Some(anim) = self.animations.get_mut(*self.current_animation as usize) {
            anim.update(ctx.dt);
            if let Some(rect) = ctx.scene.graph.try_get_mut(*self.rectangle)
                .and_then(|n| n.cast_mut::<Rectangle>())
            {
                rect.material().data_ref().bind("diffuseTexture", anim.texture());
                rect.set_uv_rect(anim.current_frame_uv_rect().unwrap_or_default());
            }
        }
        // ANCHOR_END: applying_animation
    }
}
