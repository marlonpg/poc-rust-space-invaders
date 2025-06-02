use bevy::prelude::*;
use bevy::input::ButtonInput;
use rand::seq::IteratorRandom;

// === CONSTANTS ===
const BULLET_SPEED: f32 = 500.0;
const PLAYER_SHOOT_COOLDOWN: f32 = 0.3;
const ENEMY_SPEED: f32 = 100.0;
const ENEMY_STEP_DOWN: f32 = 20.0;
const ENEMY_BULLET_SPEED: f32 = 250.0;
const ENEMY_SHOOT_COOLDOWN: f32 = 1.2;

// === COMPONENTS ===
#[derive(Component)] 
struct Player;
#[derive(Component)] 
struct Enemy;
#[derive(Component)] 
struct Bullet;
#[derive(Component)] 
struct EnemyBullet;
#[derive(Component)] 
struct ScoreText;
#[derive(Component)] 
struct LivesText;
#[derive(Component)] 
struct LevelText;
#[derive(Component)] 
struct GameOverText;

// === RESOURCES ===
#[derive(Resource)] 
struct ShootTimer(Timer);
#[derive(Resource)] 
struct EnemyMovement {
    direction: f32
}
#[derive(Resource)] 
struct GameOver(bool);
#[derive(Resource)] 
struct Score(u32);
#[derive(Resource)] 
struct EnemyShootTimer(Timer);
#[derive(Resource)] 
struct PlayerLives(u32);
#[derive(Resource)] 
struct Level(u32);
#[derive(Resource)] 
struct EnemySpeed(f32);

// === MAIN ===
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, spawn_player, spawn_enemies, setup_score_ui, setup_lives_ui, setup_level_ui))
        .insert_resource(ShootTimer(Timer::from_seconds(PLAYER_SHOOT_COOLDOWN, TimerMode::Repeating)))
        .insert_resource(EnemyMovement {
            direction: 1.0,
        })
        .insert_resource(GameOver(false))
        .insert_resource(Score(0))
        .insert_resource(EnemyShootTimer(Timer::from_seconds(ENEMY_SHOOT_COOLDOWN, TimerMode::Repeating)))
        .insert_resource(PlayerLives(3))
        .insert_resource(Level(1))
        .insert_resource(EnemySpeed(ENEMY_SPEED))
        .add_systems(Update, (
            player_movement,
            bullet_movement,
            fire_bullet,
            enemy_movement,
            bullet_enemy_collision,
            check_game_over,
            check_win_condition,
            enemy_fire_bullet,
            enemy_bullet_movement,
            enemy_bullet_player_collision,
            enemy_player_collision,
            game_over_screen,
            restart_game,
            update_score_text,
            update_lives_text,
            update_level_text,
            next_level,
        ))
        .run();
}

// === SETUP SYSTEMS ===
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("player.png"),
            transform: Transform::from_xyz(0.0, -200.0, 0.0),
            sprite: Sprite {
                custom_size: Some(Vec2::new(50.0, 20.0)),
                ..default()
            },
            ..default()
        },
        Player,
    ));
}
fn spawn_enemies(mut commands: Commands, asset_server: Res<AssetServer>) {
    let rows = 5;
    let cols = 8;
    let spacing = Vec2::new(60.0, 40.0);
    let start_x = -(cols as f32 / 2.0) * spacing.x + spacing.x / 2.0;
    let start_y = 100.0;

    for row in 0..rows {
        for col in 0..cols {
            let x = start_x + col as f32 * spacing.x;
            let y = start_y + row as f32 * spacing.y;

            commands.spawn((
                SpriteBundle {
                    texture: asset_server.load("enemy2.png"),
                    transform: Transform::from_xyz(x, y, 0.0),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(40.0, 20.0)),
                        ..default()
                    },
                    ..default()
                },
                Enemy,
            ));
        }
    }
}

// === GAME LOGIC SYSTEMS ===
fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    windows: Query<&Window>,
    time: Res<Time>,
) {
    let speed = 300.0;
    let window = windows.single();
    let half_width = window.width() / 2.0;
    let player_half_width = 25.0; // Half of player width (50.0 / 2)

    for mut transform in query.iter_mut() {
        let mut direction = 0.0;

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            direction += 1.0;
        }

        transform.translation.x += direction * speed * time.delta_seconds();

        // Clamp player position to stay within the screen bounds
        transform.translation.x = transform.translation.x
            .clamp(-half_width + player_half_width, half_width - player_half_width);
    }
}

fn fire_bullet(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    time: Res<Time>,
    mut shoot_timer: ResMut<ShootTimer>,
    query: Query<&Transform, With<Player>>,
) {
    shoot_timer.0.tick(time.delta());
    if keyboard_input.pressed(KeyCode::Space) && shoot_timer.0.finished() {
        if let Ok(player_tf) = query.get_single() {
            let bullet_spawn = player_tf.translation + Vec3::Y * 20.0;
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(5.0, 15.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(bullet_spawn),
                    ..default()
                },
                Bullet,
            ));
        }
    }
}

fn bullet_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<Bullet>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in query.iter_mut() {
        transform.translation.y += BULLET_SPEED * time.delta_seconds();
        if transform.translation.y > 300.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_movement(
    mut movement: ResMut<EnemyMovement>,
    time: Res<Time>,
    windows: Query<&Window>,
    mut query: Query<&mut Transform, With<Enemy>>,
    enemy_speed: Res<EnemySpeed>,
) {
    let window = windows.single();
    let half_width = window.width() / 2.0;
    let mut need_step_down = false;

    // Check if any enemy would go out of bounds next frame
    for transform in query.iter() {
        let x = transform.translation.x;
        let next_x = x + movement.direction * enemy_speed.0 * time.delta_seconds();
        if next_x > half_width - 20.0 || next_x < -half_width + 20.0 {
            need_step_down = true;
            movement.direction *= -1.0;
            break;
        }
    }

    for mut transform in query.iter_mut() {
        if need_step_down {
            // Only step down once per direction change (use timer to limit how often this happens if needed)
            transform.translation.y -= ENEMY_STEP_DOWN;
        } else {
            // Smooth horizontal movement
            transform.translation.x += movement.direction * enemy_speed.0 * time.delta_seconds();
        }
    }
}

fn check_game_over(
    mut game_over: ResMut<GameOver>,
    enemy_query: Query<&Transform, With<Enemy>>,
) {
    for transform in enemy_query.iter() {
        if transform.translation.y <= -250.0 {
            game_over.0 = true;
            println!("Game Over!");
            break;
        }
    }
}

fn check_win_condition(
    enemy_query: Query<Entity, With<Enemy>>,
    mut game_over: ResMut<GameOver>,
) {
    if enemy_query.iter().next().is_none() && !game_over.0 {
        game_over.0 = true;
        println!("You win!");
    }
}

fn enemy_player_collision(
    mut game_over: ResMut<GameOver>,
    enemy_query: Query<(&Transform, &Sprite), With<Enemy>>,
    player_query: Query<(&Transform, &Sprite), With<Player>>,
) {
    if game_over.0 {
        return;
    }
    for (enemy_tf, _enemy_sprite) in enemy_query.iter() {
        let enemy_pos = enemy_tf.translation;
        for (player_tf, player_sprite) in player_query.iter() {
            let player_size = player_sprite.custom_size.unwrap_or(Vec2::ZERO);
            let player_pos = player_tf.translation;
            let collision = enemy_pos.x < player_pos.x + player_size.x / 2.0
                && enemy_pos.x > player_pos.x - player_size.x / 2.0
                && enemy_pos.y < player_pos.y + player_size.y / 2.0
                && enemy_pos.y > player_pos.y - player_size.y / 2.0;
            if collision {
                game_over.0 = true;
                println!("Game Over! Enemy collided with player.");
                return;
            }
        }
    }
}

fn bullet_enemy_collision(
    mut commands: Commands,
    mut score: ResMut<Score>,
    bullet_query: Query<(Entity, &Transform, &Sprite), With<Bullet>>,
    enemy_query: Query<(Entity, &Transform, &Sprite), With<Enemy>>,
    mut game_over: ResMut<GameOver>,
) {
    for (bullet_entity, bullet_tf, _bullet_sprite) in bullet_query.iter() {
        let bullet_pos = bullet_tf.translation;
        for (enemy_entity, enemy_tf, enemy_sprite) in enemy_query.iter() {
            let enemy_size = enemy_sprite.custom_size.unwrap_or(Vec2::ZERO);
            let enemy_pos = enemy_tf.translation;
            let collision = bullet_pos.x < enemy_pos.x + enemy_size.x / 2.0
                && bullet_pos.x > enemy_pos.x - enemy_size.x / 2.0
                && bullet_pos.y < enemy_pos.y + enemy_size.y / 2.0
                && bullet_pos.y > enemy_pos.y - enemy_size.y / 2.0;
            if collision {
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
                score.0 += 100;
                println!("Hit! Score: {}", score.0);
                if score.0 == 4000 {
                    println!("🏆 You win!");
                    game_over.0 = true;
                }
                break;
            }
        }
    }
}

fn enemy_fire_bullet(
    mut commands: Commands,
    time: Res<Time>,
    mut shoot_timer: ResMut<EnemyShootTimer>,
    enemy_query: Query<&Transform, With<Enemy>>,
) {
    shoot_timer.0.tick(time.delta());
    if shoot_timer.0.finished() {
        if let Some(enemy_tf) = enemy_query.iter().choose(&mut rand::rng()) {
            let bullet_spawn = enemy_tf.translation - Vec3::Y * 20.0;
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::YELLOW,
                        custom_size: Some(Vec2::new(5.0, 15.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(bullet_spawn),
                    ..default()
                },
                EnemyBullet,
            ));
        }
    }
}

fn enemy_bullet_movement(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<EnemyBullet>>,
    time: Res<Time>,
) {
    for (entity, mut transform) in query.iter_mut() {
        transform.translation.y -= ENEMY_BULLET_SPEED * time.delta_seconds();
        if transform.translation.y < -320.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_bullet_player_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform, &Sprite), With<EnemyBullet>>,
    player_query: Query<(Entity, &Transform, &Sprite), With<Player>>,
    mut game_over: ResMut<GameOver>,
    mut lives: ResMut<PlayerLives>,
    asset_server: Res<AssetServer>
) {
    let mut collision_detected = false;
    for (bullet_entity, bullet_tf, _bullet_sprite) in bullet_query.iter() {
        let bullet_pos = bullet_tf.translation;
        for (player_entity, player_tf, player_sprite) in player_query.iter() {
            let player_size = player_sprite.custom_size.unwrap_or(Vec2::ZERO);
            let player_pos = player_tf.translation;
            let collision = bullet_pos.x < player_pos.x + player_size.x / 2.0
                && bullet_pos.x > player_pos.x - player_size.x / 2.0
                && bullet_pos.y < player_pos.y + player_size.y / 2.0
                && bullet_pos.y > player_pos.y - player_size.y / 2.0;
            if collision {
                commands.entity(bullet_entity).despawn();
                commands.entity(player_entity).despawn();
                collision_detected = true;
                break;
            }
        }
    }

    if collision_detected {
        if lives.0 > 1 {
            lives.0 -= 1;
            println!("You were hit! Lives left: {}", lives.0);
            // Respawn player
            spawn_player(commands.reborrow(), asset_server);
        } else {
            lives.0 -= 1;
            game_over.0 = true;
            println!("You were hit! Game Over!");
        }
    }
}

fn game_over_screen(
    game_over: Res<GameOver>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut game_over_text_query: Query<Entity, With<GameOverText>>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    if game_over.is_changed() {
        for entity in game_over_text_query.iter_mut() {
            commands.entity(entity).despawn();
        }
        if game_over.0 {
            if enemy_query.iter().next().is_none() {
                commands.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "YOU WIN!\nPress N for Next Level",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 60.0,
                                color: Color::GREEN,
                            },
                        ),
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(25.0),
                            top: Val::Percent(40.0),
                            ..default()
                        },
                        ..default()
                    },
                    GameOverText,
                ));
            } else {
                commands.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "GAME OVER\nPress R to Restart",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 60.0,
                                color: Color::RED,
                            },
                        ),
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(25.0),
                            top: Val::Percent(40.0),
                            ..default()
                        },
                        ..default()
                    },
                    GameOverText,
                ));
         }
        } else {
            for entity in game_over_text_query.iter_mut() {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn restart_game(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_over: ResMut<GameOver>,
    mut score: ResMut<Score>,
    mut lives: ResMut<PlayerLives>,
    mut level: ResMut<Level>,
    enemy_query: Query<Entity, With<Enemy>>,
    bullet_query: Query<Entity, With<Bullet>>,
    enemy_bullet_query: Query<Entity, With<EnemyBullet>>,
    player_query: Query<Entity, With<Player>>,
    mut enemy_speed: ResMut<EnemySpeed>,
    asset_server: Res<AssetServer>,
    asset_server2: Res<AssetServer>,
) {
    if game_over.0 && keyboard_input.just_pressed(KeyCode::KeyR) {
        for entity in enemy_query.iter() { commands.entity(entity).despawn(); }
        for entity in bullet_query.iter() { commands.entity(entity).despawn(); }
        for entity in enemy_bullet_query.iter() { commands.entity(entity).despawn(); }
        for entity in player_query.iter() { commands.entity(entity).despawn(); }
        score.0 = 0;
        lives.0 = 3;
        level.0 = 1;
        game_over.0 = false;
        enemy_speed.0 = ENEMY_SPEED;
        spawn_player(commands.reborrow(), asset_server);
        spawn_enemies(commands.reborrow(), asset_server2);
    }
}

// === UI SYSTEMS ===
fn setup_score_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::from_style(TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                color: Color::WHITE,
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
        ScoreText,
    ));
}

fn update_score_text(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if score.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[1].value = score.0.to_string();
        }
    }
}

fn setup_lives_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Lives: ",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::from_style(TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                color: Color::WHITE,
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(10.0),
            ..default()
        }),
        LivesText,
    ));
}

fn update_lives_text(lives: Res<PlayerLives>, mut query: Query<&mut Text, With<LivesText>>) {
    if lives.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[1].value = lives.0.to_string();
        }
    }
}

fn setup_level_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Level: ",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::from_style(TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                color: Color::WHITE,
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(70.0),
            left: Val::Px(10.0),
            ..default()
        }),
        LevelText,
    ));
}

fn update_level_text(level: Res<Level>, mut query: Query<&mut Text, With<LevelText>>) {
    if level.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[1].value = level.0.to_string();
        }
    }
}

fn next_level(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_over: ResMut<GameOver>,
    mut level: ResMut<Level>,
    mut enemy_speed: ResMut<EnemySpeed>,
    enemy_query: Query<Entity, With<Enemy>>,
    bullet_query: Query<Entity, With<Bullet>>,
    enemy_bullet_query: Query<Entity, With<EnemyBullet>>,
    player_query: Query<Entity, With<Player>>,
    asset_server: Res<AssetServer>,
    asset_server2: Res<AssetServer>,
) {
    // Only allow next level if all enemies are gone and game_over is true
    if enemy_query.iter().next().is_none() && keyboard_input.just_pressed(KeyCode::KeyN) {
        // Clean up
        for entity in bullet_query.iter() { 
            commands.entity(entity).despawn(); 
        }
        for entity in enemy_bullet_query.iter() { 
            commands.entity(entity).despawn(); 
        }
        for entity in player_query.iter() { 
            commands.entity(entity).despawn(); 
        }
        
        level.0 += 1;
        enemy_speed.0 += 50.0;
        game_over.0 = false;
        spawn_player(commands.reborrow(), asset_server);
        spawn_enemies(commands.reborrow(), asset_server2);
    }
}

