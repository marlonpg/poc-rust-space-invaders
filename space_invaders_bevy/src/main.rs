use bevy::prelude::*;
use bevy::input::ButtonInput;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera, spawn_player, spawn_enemies))
        .insert_resource(ShootTimer(Timer::from_seconds(PLAYER_SHOOT_COOLDOWN, TimerMode::Repeating)))
        .insert_resource(EnemyMovement {
            direction: 1.0,
            timer: Timer::from_seconds(ENEMY_MOVE_INTERVAL, TimerMode::Repeating),
        })
        .insert_resource(GameOver(false))
        .insert_resource(Score(0))

        .add_systems(Update, (
            player_movement,
            bullet_movement,
            fire_bullet,
            enemy_movement,
            bullet_enemy_collision,
            check_game_over,
        ))
        .run();
}

// === COMPONENTS ===

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;


#[derive(Component)]
struct Bullet;

const BULLET_SPEED: f32 = 500.0;
const PLAYER_SHOOT_COOLDOWN: f32 = 0.3; // seconds

#[derive(Resource)]
struct ShootTimer(Timer);

#[derive(Resource)]
struct EnemyMovement {
    direction: f32, // 1.0 = right, -1.0 = left
    timer: Timer,
}

#[derive(Resource)]
struct GameOver(bool); // true = game over

#[derive(Resource)]
struct Score(u32);


const ENEMY_SPEED: f32 = 20.0;
const ENEMY_MOVE_INTERVAL: f32 = 0.5;
const ENEMY_STEP_DOWN: f32 = 20.0;


// === SETUP SYSTEMS ===

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_player(mut commands: Commands) {
    let player_entity = commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.3, 0.8, 1.0),
                custom_size: Some(Vec2::new(50.0, 20.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -200.0, 0.0),
            ..default()
        },
        Player,
    )).id(); 
}

fn spawn_enemies(mut commands: Commands) {
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
                    sprite: Sprite {
                        color: Color::rgb(1.0, 0.4, 0.4),
                        custom_size: Some(Vec2::new(40.0, 20.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, 0.0),
                    ..default()
                },
                Enemy,
            ));
        }
    }
}

// === PLAYER MOVEMENT ===

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    let speed = 300.0;

    for mut transform in query.iter_mut() {
        let mut direction = 0.0;

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            direction += 1.0;
        }

        transform.translation.x += direction * speed * time.delta_seconds();
    }
}
// === BULLET MOVEMENT ===

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

        // Despawn if off-screen
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
) {
    let window = windows.single();
    let half_width = window.width() / 2.0;

    // Tick the timer
    if movement.timer.tick(time.delta()).just_finished() {
        let mut move_down = false;

        // Check if any enemy will go out of bounds next frame
        for transform in query.iter() {
            let x = transform.translation.x;
            let next_x = x + movement.direction * ENEMY_SPEED;

            if next_x > half_width - 20.0 || next_x < -half_width + 20.0 {
                move_down = true;
                movement.direction *= -1.0; // reverse direction
                break;
            }
        }

        // Apply movement
        for mut transform in query.iter_mut() {
            if move_down {
                transform.translation.y -= ENEMY_STEP_DOWN;
            } else {
                transform.translation.x += movement.direction * ENEMY_SPEED;
            }
        }
    }
}

/*
fn bullet_enemy_collision(
    mut commands: Commands,
    bullet_query: Query<(Entity, &Transform, &Sprite), With<Bullet>>,
    enemy_query: Query<(Entity, &Transform, &Sprite), With<Enemy>>,
) {
    for (bullet_entity, bullet_tf, bullet_sprite) in bullet_query.iter() {
        let bullet_size = bullet_sprite.custom_size.unwrap_or(Vec2::ZERO);
        let bullet_pos = bullet_tf.translation;

        for (enemy_entity, enemy_tf, enemy_sprite) in enemy_query.iter() {
            let enemy_size = enemy_sprite.custom_size.unwrap_or(Vec2::ZERO);
            let enemy_pos = enemy_tf.translation;

            // Simple AABB collision check
            let collision = bullet_pos.x < enemy_pos.x + enemy_size.x / 2.0
                && bullet_pos.x > enemy_pos.x - enemy_size.x / 2.0
                && bullet_pos.y < enemy_pos.y + enemy_size.y / 2.0
                && bullet_pos.y > enemy_pos.y - enemy_size.y / 2.0;

            if collision {
                // Despawn both
                commands.entity(bullet_entity).despawn();
                commands.entity(enemy_entity).despawn();
                break; // Stop checking once bullet hits
            }
        }
    }
}
*/

fn check_game_over(
    mut game_over: ResMut<GameOver>,
    enemy_query: Query<&Transform, With<Enemy>>,
) {
    for transform in enemy_query.iter() {
        if transform.translation.y <= -250.0 {
            game_over.0 = true;
            println!("💀 Game Over!");
            break;
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
    for (bullet_entity, bullet_tf, bullet_sprite) in bullet_query.iter() {
        let bullet_size = bullet_sprite.custom_size.unwrap_or(Vec2::ZERO);
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
                println!("💥 Hit! Score: {}", score.0);
                if(score.0 == 4000) {
                    println!("🏆 You win!");
                    game_over.0 = true;
                }   
                break;
            }
        }
    }
}

