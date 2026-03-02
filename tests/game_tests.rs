use pixel_game_server::config::GameConfig;
use pixel_game_server::game::items::Item;
use pixel_game_server::game::state::GameMatch;
use pixel_game_server::game::tile::TileType;
use pixel_game_server::game::world::World;
use pixel_game_server::network::handler;
use pixel_game_server::network::messages::{ClientMessage, Direction};

// ─── World Generation ────────────────────────────────────────────────────────

#[test]
fn test_world_generation_is_deterministic() {
    let world1 = World::generate(42, 100, 100);
    let world2 = World::generate(42, 100, 100);
    for y in 0..100i32 {
        for x in 0..100i32 {
            assert_eq!(
                world1.get_tile(x, y),
                world2.get_tile(x, y),
                "Tile mismatch at ({}, {})",
                x,
                y
            );
        }
    }
}

#[test]
fn test_different_seeds_produce_different_worlds() {
    let world1 = World::generate(1, 100, 100);
    let world2 = World::generate(2, 100, 100);
    let mut differences = 0;
    for y in 0..100i32 {
        for x in 0..100i32 {
            if world1.get_tile(x, y) != world2.get_tile(x, y) {
                differences += 1;
            }
        }
    }
    assert!(differences > 0, "Different seeds should produce different worlds");
}

#[test]
fn test_spawn_area_is_walkable() {
    let world = World::generate(42, 100, 100);
    let spawns = world.get_spawn_positions();
    for (sx, sy) in &spawns {
        assert!(
            world.is_walkable(*sx, *sy),
            "Spawn at ({}, {}) should be walkable",
            sx,
            sy
        );
    }
}

#[test]
fn test_out_of_bounds_not_walkable() {
    let world = World::generate(42, 100, 100);
    assert!(!world.is_walkable(-1, 0));
    assert!(!world.is_walkable(0, -1));
    assert!(!world.is_walkable(100, 0));
    assert!(!world.is_walkable(0, 100));
}

// ─── Tile Types ──────────────────────────────────────────────────────────────

#[test]
fn test_tile_walkability() {
    assert!(TileType::Grass.is_walkable());
    assert!(TileType::Sand.is_walkable());
    assert!(TileType::Forest.is_walkable());
    assert!(!TileType::Water.is_walkable());
    assert!(!TileType::Wall.is_walkable());
}

// ─── Match Creation ──────────────────────────────────────────────────────────

#[test]
fn test_match_creation() {
    let config = GameConfig::default();
    let game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );
    assert_eq!(game.players.len(), 2);
    assert!(game.dragon.is_alive());
    assert_eq!(game.dragon.hp, config.dragon_hp);
    assert_eq!(game.items.len(), 3);
    assert_eq!(game.get_player("p1").unwrap().name, "Alice");
    assert_eq!(game.get_player("p2").unwrap().name, "Bob");
}

// ─── Player Movement ─────────────────────────────────────────────────────────

#[test]
fn test_player_movement() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );
    let initial_x = game.get_player("p1").unwrap().x;
    let initial_y = game.get_player("p1").unwrap().y;

    // Spawn area is cleared grass, so moving right should succeed
    assert!(game.try_move("p1", 1, 0));
    assert_eq!(game.get_player("p1").unwrap().x, initial_x + 1);
    assert_eq!(game.get_player("p1").unwrap().y, initial_y);
}

#[test]
fn test_cannot_walk_off_map() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    // Teleport to top-left corner
    let player = game.get_player_mut("p1").unwrap();
    player.x = 0;
    player.y = 0;

    assert!(!game.try_move("p1", -1, 0), "Should not walk left off map");
    assert!(!game.try_move("p1", 0, -1), "Should not walk up off map");
}

// ─── Item Pickup ─────────────────────────────────────────────────────────────

#[test]
fn test_item_pickup() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    let item_x = game.items[0].x;
    let item_y = game.items[0].y;
    let item_type = game.items[0].item;

    // Teleport player to item
    let player = game.get_player_mut("p1").unwrap();
    player.x = item_x;
    player.y = item_y;

    let picked_up = game.check_item_pickup("p1");
    assert_eq!(picked_up, Some(item_type));
    assert!(game.get_player("p1").unwrap().inventory.contains(&item_type));

    // Picking up same item again should return None
    assert_eq!(game.check_item_pickup("p1"), None);
}

#[test]
fn test_both_players_can_pick_same_item() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    let item_x = game.items[0].x;
    let item_y = game.items[0].y;
    let item_type = game.items[0].item;

    // Teleport both players to the item
    game.get_player_mut("p1").unwrap().x = item_x;
    game.get_player_mut("p1").unwrap().y = item_y;
    game.get_player_mut("p2").unwrap().x = item_x;
    game.get_player_mut("p2").unwrap().y = item_y;

    // Both should independently collect it
    assert_eq!(game.check_item_pickup("p1"), Some(item_type));
    assert_eq!(game.check_item_pickup("p2"), Some(item_type));
}

// ─── Dragon Combat ───────────────────────────────────────────────────────────

#[test]
fn test_dragon_attack_requires_all_items() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    // Teleport player next to dragon
    let dx = game.dragon.x;
    let dy = game.dragon.y;
    let player = game.get_player_mut("p1").unwrap();
    player.x = dx;
    player.y = dy + 1;

    // Attack without items should fail
    assert!(game.attack_dragon("p1").is_none());

    // Give all 3 items
    let player = game.get_player_mut("p1").unwrap();
    player.pick_up(Item::HolySword);
    player.pick_up(Item::HolyArmor);
    player.pick_up(Item::DragonMap);

    // Now attack should succeed
    let result = game.attack_dragon("p1");
    assert!(result.is_some());
    let (damage_dealt, damage_taken, _dead) = result.unwrap();
    assert_eq!(damage_dealt, config.sword_damage);
    assert_eq!(damage_taken, config.dragon_damage - config.armor_reduction);
}

#[test]
fn test_dragon_too_far_to_attack() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    // Give all items but stay far from dragon
    let player = game.get_player_mut("p1").unwrap();
    player.pick_up(Item::HolySword);
    player.pick_up(Item::HolyArmor);
    player.pick_up(Item::DragonMap);
    player.x = 0;
    player.y = 0;

    assert!(game.attack_dragon("p1").is_none());
}

#[test]
fn test_killing_dragon_ends_match() {
    let mut config = GameConfig::default();
    config.dragon_hp = 50; // One hit kill with sword_damage = 50

    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    // Give all items, teleport next to dragon
    let dx = game.dragon.x;
    let dy = game.dragon.y;
    let player = game.get_player_mut("p1").unwrap();
    player.pick_up(Item::HolySword);
    player.pick_up(Item::HolyArmor);
    player.pick_up(Item::DragonMap);
    player.x = dx;
    player.y = dy + 1;

    let result = game.attack_dragon("p1");
    assert!(result.is_some());
    let (_dmg, _taken, dragon_dead) = result.unwrap();
    assert!(dragon_dead);
    assert_eq!(
        game.status,
        pixel_game_server::game::state::MatchStatus::Finished {
            winner_id: "p1".to_string()
        }
    );
}

// ─── Message Handler ─────────────────────────────────────────────────────────

#[test]
fn test_handler_move_message() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );
    let initial_x = game.get_player("p1").unwrap().x;

    let msg = ClientMessage::Move {
        direction: Direction::Right,
    };
    let responses = handler::process_message(&mut game, "p1", msg);

    // Move should succeed (spawn area is clear), no error responses
    assert!(
        responses.iter().all(|(_, m)| !matches!(m, pixel_game_server::network::messages::ServerMessage::MoveDenied { .. })),
        "Move in spawn area should not be denied"
    );
    assert_eq!(game.get_player("p1").unwrap().x, initial_x + 1);
}

#[test]
fn test_handler_attack_without_items() {
    let config = GameConfig::default();
    let mut game = GameMatch::new(
        "p1".into(), "Alice".into(),
        "p2".into(), "Bob".into(),
        &config,
    );

    let msg = ClientMessage::Attack;
    let responses = handler::process_message(&mut game, "p1", msg);

    // Should get an error response
    assert!(responses.iter().any(|(_, m)| matches!(
        m,
        pixel_game_server::network::messages::ServerMessage::Error { .. }
    )));
}
