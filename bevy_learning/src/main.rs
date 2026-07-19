use bevy::prelude::*;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Component)]
struct Stamina {
    current: f32,
    max: f32,
    regen_rate: f32,
}

#[derive(Component)]
struct Level {
    value: u32,
}

#[derive(Component)]
struct StatsText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, handle_stats_input, update_stats_ui))
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn camera
    commands.spawn(Camera2d);

    // Spawn player sprite (a nice blue square representing the character)
    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.6, 1.0),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Player,
        Health { current: 100.0, max: 100.0 },
        Stamina { current: 100.0, max: 100.0, regen_rate: 15.0 },
        Level { value: 1 },
    ));

    // Spawn UI Panel to display Stats
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            padding: UiRect::all(Val::Px(15.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            StatsText,
        ));
    });
}

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Stamina)>,
    time: Res<Time>,
) {
    if let Ok((mut transform, mut stamina)) = query.single_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        // Determine if sprinting
        let wants_to_sprint = keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);
        let can_sprint = wants_to_sprint && stamina.current > 0.0 && direction != Vec3::ZERO;

        let speed = if can_sprint {
            // Sprinting drains stamina
            stamina.current = (stamina.current - 35.0 * time.delta_secs()).max(0.0);
            400.0
        } else {
            // Regenerate stamina if not sprinting
            stamina.current = (stamina.current + stamina.regen_rate * time.delta_secs()).min(stamina.max);
            200.0
        };

        if direction != Vec3::ZERO {
            direction = direction.normalize();
            transform.translation += direction * speed * time.delta_secs();
        }
    }
}

fn handle_stats_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Health, &mut Stamina, &mut Level), With<Player>>,
) {
    if let Ok((mut health, mut stamina, mut level)) = query.single_mut() {
        // Press H to take damage
        if keyboard_input.just_pressed(KeyCode::KeyH) {
            health.current = (health.current - 10.0).max(0.0);
        }

        // Press K to heal
        if keyboard_input.just_pressed(KeyCode::KeyK) {
            health.current = (health.current + 10.0).min(health.max);
        }

        // Press L to level up stats
        if keyboard_input.just_pressed(KeyCode::KeyL) {
            level.value += 1;
            health.max += 10.0;
            stamina.max += 10.0;
            // Also restore health and stamina on level up
            health.current = health.max;
            stamina.current = stamina.max;
        }
    }
}

fn update_stats_ui(
    player_query: Query<(&Health, &Stamina, &Level), With<Player>>,
    mut ui_query: Query<&mut Text, With<StatsText>>,
) {
    if let Ok((health, stamina, level)) = player_query.single() {
        if let Ok(mut text) = ui_query.single_mut() {
            **text = format!(
                "CHARACTER STATS\n\
                 -----------------\n\
                 Level: {}\n\
                 Health: {:.0} / {:.0}\n\
                 Stamina: {:.0} / {:.0}\n\n\
                 CONTROLS:\n\
                 - WASD / Arrows to Move\n\
                 - Hold Shift to Sprint (uses Stamina)\n\
                 - Press H to Take Damage (-10 HP)\n\
                 - Press K to Heal (+10 HP)\n\
                 - Press L to Level Up (+10 Max HP/Stamina)",
                level.value, health.current, health.max, stamina.current, stamina.max
            );
        }
    }
}
