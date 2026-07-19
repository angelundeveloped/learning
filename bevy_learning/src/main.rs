use bevy::prelude::*;

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as f32 / f32::MAX
    }
}

// Grid Settings
const GRID_SIZE: i32 = 12;
const TILE_SIZE: f32 = 40.0;
const TILE_GAP: f32 = 4.0;

#[derive(Resource)]
struct ColonyResources {
    wood: u32,
    stone: u32,
}

#[derive(Component, Clone, Copy, PartialEq)]
enum TileType {
    Grass,
    Tree,
    Rock,
    Storage,
}

#[derive(Component)]
struct Tile {
    tile_type: TileType,
    resources_left: u32,
}

#[derive(Component, PartialEq, Clone, Copy)]
enum WorkerState {
    Idle,
    MovingToResource,
    Gathering,
    MovingToStorage,
}

#[derive(Component)]
struct Worker {
    state: WorkerState,
    target_tile: Option<Entity>,
    inventory_type: Option<TileType>,
    inventory_amount: u32,
    max_capacity: u32,
    speed: f32,
}

#[derive(Component)]
struct HUDText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ColonyResources { wood: 0, stone: 0 })
        .add_systems(Startup, setup)
        .add_systems(Update, (worker_ai, worker_movement, update_hud))
        .run();
}

fn grid_to_world(x: i32, y: i32) -> Vec3 {
    let offset = (GRID_SIZE as f32 * (TILE_SIZE + TILE_GAP)) / 2.0 - (TILE_SIZE / 2.0);
    Vec3::new(
        x as f32 * (TILE_SIZE + TILE_GAP) - offset,
        y as f32 * (TILE_SIZE + TILE_GAP) - offset,
        0.0,
    )
}

fn setup(mut commands: Commands) {
    // Spawn camera
    commands.spawn(Camera2d);

    let mut rng = SimpleRng::new(12345);

    // Spawn Grid
    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            // Determine tile type
            let rand_val = rng.next_f32();
            
            // Place Storage in the exact center
            let is_center = x == GRID_SIZE / 2 && y == GRID_SIZE / 2;
            
            let (tile_type, color, resources_left) = if is_center {
                (TileType::Storage, Color::srgb(0.6, 0.3, 0.1), 0)
            } else if rand_val < 0.12 {
                (TileType::Tree, Color::srgb(0.1, 0.7, 0.2), 50)
            } else if rand_val < 0.20 {
                (TileType::Rock, Color::srgb(0.5, 0.5, 0.5), 50)
            } else {
                (TileType::Grass, Color::srgb(0.2, 0.5, 0.2), 0)
            };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(TILE_SIZE, TILE_SIZE)),
                    ..default()
                },
                Transform::from_translation(grid_to_world(x, y)),
                Tile { tile_type, resources_left },
            ));
        }
    }

    // Spawn Workers (Colony Agents)
    for i in 0..3 {
        commands.spawn((
            Sprite {
                color: Color::srgb(1.0, 0.9, 0.2), // Yellow workers
                custom_size: Some(Vec2::new(18.0, 18.0)),
                ..default()
            },
            // Start workers near storage
            Transform::from_translation(grid_to_world(GRID_SIZE / 2, GRID_SIZE / 2) + Vec3::new((i as f32 - 1.0) * 15.0, -15.0, 1.0)),
            Worker {
                state: WorkerState::Idle,
                target_tile: None,
                inventory_type: None,
                inventory_amount: 0,
                max_capacity: 10,
                speed: 150.0,
            },
        ));
    }

    // Spawn UI Panel
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            padding: UiRect::all(Val::Px(15.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            HUDText,
        ));
    });
}

fn worker_ai(
    mut workers_query: Query<(Entity, &mut Worker, &Transform)>,
    mut tiles_query: Query<(Entity, &mut Tile, &mut Sprite, &Transform)>,
    storage_query: Query<(Entity, &Tile), Without<Worker>>,
) {
    // Find Storage entity
    let storage_entity = storage_query
        .iter()
        .find(|(_, tile)| tile.tile_type == TileType::Storage)
        .map(|(entity, _)| entity);

    for (worker_entity, mut worker, worker_transform) in &mut workers_query {
        let _ = worker_entity; // suppress unused warning
        match worker.state {
            WorkerState::Idle => {
                // If inventory is full, go deposit
                if worker.inventory_amount >= worker.max_capacity {
                    worker.state = WorkerState::MovingToStorage;
                    worker.target_tile = storage_entity;
                } else {
                    // Search for closest active Resource Tile (Tree or Rock)
                    let mut closest_resource: Option<(Entity, TileType, f32)> = None;

                    for (tile_entity, tile, _, tile_transform) in tiles_query.iter() {
                        if (tile.tile_type == TileType::Tree || tile.tile_type == TileType::Rock)
                            && tile.resources_left > 0
                        {
                            let dist = worker_transform.translation.distance(tile_transform.translation);
                            if closest_resource.is_none() || dist < closest_resource.unwrap().2 {
                                closest_resource = Some((tile_entity, tile.tile_type, dist));
                            }
                        }
                    }

                    if let Some((target_entity, resource_type, _)) = closest_resource {
                        worker.state = WorkerState::MovingToResource;
                        worker.target_tile = Some(target_entity);
                        worker.inventory_type = Some(resource_type);
                    }
                }
            }
            WorkerState::Gathering => {
                // We are at the resource node. Let's harvest it.
                if let Some(target) = worker.target_tile {
                    if let Ok((_, mut tile, mut sprite, _)) = tiles_query.get_mut(target) {
                        if tile.resources_left > 0 {
                            tile.resources_left = tile.resources_left.saturating_sub(1);
                            worker.inventory_amount += 1;

                            // Visual update for depleted node
                            if tile.resources_left == 0 {
                                tile.tile_type = TileType::Grass;
                                sprite.color = Color::srgb(0.2, 0.5, 0.2); // Back to grass
                            }

                            // If inventory is full or node depleted, head to storage
                            if worker.inventory_amount >= worker.max_capacity || tile.resources_left == 0 {
                                worker.state = WorkerState::MovingToStorage;
                                worker.target_tile = storage_entity;
                            }
                        } else {
                            // Node depleted by someone else
                            worker.state = WorkerState::Idle;
                            worker.target_tile = None;
                        }
                    } else {
                        // Target invalid
                        worker.state = WorkerState::Idle;
                        worker.target_tile = None;
                    }
                }
            }
            _ => {}
        }
    }
}

fn worker_movement(
    mut workers_query: Query<(&mut Worker, &mut Transform)>,
    targets_query: Query<&Transform, Without<Worker>>,
    mut resources: ResMut<ColonyResources>,
    time: Res<Time>,
) {
    for (mut worker, mut worker_transform) in &mut workers_query {
        if let Some(target_entity) = worker.target_tile {
            if let Ok(target_transform) = targets_query.get(target_entity) {
                // Move towards target
                let direction = target_transform.translation - worker_transform.translation;
                let distance = direction.length();

                // Target depth z adjustment to keep worker on top of tiles
                let direction_2d = Vec3::new(direction.x, direction.y, 0.0);

                if distance > 10.0 {
                    let move_dir = direction_2d.normalize();
                    worker_transform.translation += move_dir * worker.speed * time.delta_secs();
                } else {
                    // Arrived at target
                    match worker.state {
                        WorkerState::MovingToResource => {
                            worker.state = WorkerState::Gathering;
                        }
                        WorkerState::MovingToStorage => {
                            // Deposit resources
                            if let Some(resource_type) = worker.inventory_type {
                                match resource_type {
                                    TileType::Tree => resources.wood += worker.inventory_amount,
                                    TileType::Rock => resources.stone += worker.inventory_amount,
                                    _ => {}
                                }
                            }
                            worker.inventory_amount = 0;
                            worker.inventory_type = None;
                            worker.state = WorkerState::Idle;
                            worker.target_tile = None;
                        }
                        _ => {}
                    }
                }
            } else {
                // Target disappeared or invalid
                worker.state = WorkerState::Idle;
                worker.target_tile = None;
            }
        }
    }
}

fn update_hud(
    resources: Res<ColonyResources>,
    workers_query: Query<&Worker>,
    mut ui_query: Query<&mut Text, With<HUDText>>,
) {
    if let Ok(mut text) = ui_query.single_mut() {
        let mut gatherer_count = 0;
        let mut depositer_count = 0;
        let mut idle_count = 0;

        for worker in workers_query.iter() {
            match worker.state {
                WorkerState::Idle => idle_count += 1,
                WorkerState::MovingToResource | WorkerState::Gathering => gatherer_count += 1,
                WorkerState::MovingToStorage => depositer_count += 1,
            }
        }

        **text = format!(
            "COLONY STATUS\n\
             -----------------\n\
             Wood in Stockpile: {}\n\
             Stone in Stockpile: {}\n\n\
             Workers: {}\n\
             - Gathering: {}\n\
             - Depositing: {}\n\
             - Idle: {}",
            resources.wood,
            resources.stone,
            gatherer_count + depositer_count + idle_count,
            gatherer_count,
            depositer_count,
            idle_count
        );
    }
}
