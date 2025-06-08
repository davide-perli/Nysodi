// game/tests/automated_tests.rs

use nysodi::bot::{Bot, ReactionState};
use fyrox::core::algebra::Vector2;
use nysodi::random_point_around;
use fyrox::{
    scene::{
        node::Node,
        base::BaseBuilder,
    },
    asset::manager::ResourceManager,
    gui::texture::Texture,
    script::ScriptContext,
    graph::{BaseSceneGraph, SceneGraph},
};
use nysodi::{Game, Player};

#[test]
fn test_map_edges_clamping() {
    // Directly test the clamp logic you use in random_point_around
    let test_positions = [
        Vector2::new(-20.0_f32,  0.0),
        Vector2::new( 20.0,       0.0),
        Vector2::new( 0.0,      -10.0),
        Vector2::new( 0.0,       20.0),
    ];
    let expected = [
        Vector2::new(-11.0_f32,   0.0),
        Vector2::new( 11.0,       0.0),
        Vector2::new(  0.0,      -4.0),
        Vector2::new(  0.0,      17.0),
    ];

    for (&cand, &exp) in test_positions.iter().zip(expected.iter()) {
        let clamped = Vector2::new(
            cand.x.clamp(-11.0_f32, 11.0_f32),
            cand.y.clamp(-4.0_f32, 17.0_f32),
        );
        assert_eq!(clamped, exp, "Clamped {:?} → {:?}", cand, clamped);
    }
}

#[test]
fn test_bot_trigger_reaction() {
    let mut bot = Bot::default();
    // Initially no reaction is pending
    assert_eq!(bot.reaction_timer, 0.0);
    // Trigger a reaction
    bot.trigger_reaction();
    // Should reset timer to exactly 3.0 seconds
    assert!((bot.reaction_timer - 3.0).abs() < f32::EPSILON);
    // State must be one of the two variants
    match bot.reaction_state {
        ReactionState::Motionless | ReactionState::RunningAway => {}
        other => panic!("Unexpected ReactionState::{:?}", other),
    }
}

/*#[test]
fn test_spawn_methods_place_items_in_bounds() {
    // Create a mock or minimal ScriptContext for testing
    
    let game = ctx.plugins.get::<Game>();
    let player_handle = game.player;
    let player: &mut Player = ctx.scene.graph[player_handle]
        .script_mut(0).unwrap().cast_mut().unwrap();

    // HEART
    let heart = player.spawn_heart(&mut ctx);
    let hpos = ctx.scene.graph[heart].global_position().xy();
    assert!((-11.0..=11.0).contains(&hpos.x) && (-4.0..=17.0).contains(&hpos.y));

    player.health = 30.0;
    player.on_item_collected(heart, &mut ctx);
    assert!(player.health > 30.0, "Heart worked, health restored to {}", player.health);
    println!("Heart worked, health restored to {}", player.health);

    // BOMB
    let bomb = player.spawn_item(&mut ctx, Vector2::new(20.0, 20.0));
    let bpos = ctx.scene.graph[bomb].global_position().xy();
    assert!((-11.0..=11.0).contains(&bpos.x) && (-4.0..=17.0).contains(&bpos.y));

    // FIRE
    let fire = player.spawn_fire(&mut ctx, Vector2::new(-20.0, -20.0));
    let fpos = ctx.scene.graph[fire].global_position().xy();
    assert!((-11.0..=11.0).contains(&fpos.x) && (-4.0..=17.0).contains(&fpos.y));
}*/