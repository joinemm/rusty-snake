use bevy::core::FixedTimestep;
use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use rand::prelude::random;
use std::f64::consts::PI;

const ARENA_WIDTH: u32 = 16;
const ARENA_HEIGHT: u32 = 16;
const BACKGROUND_COLOR: &str = "5e81ac";
const SNAKE_COLOR: &str = "a3be8c";

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
struct Position {
    x: i32,
    y: i32,
}

struct Size {
    width: f32,
    height: f32,
}
impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}
impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

struct SnakeHead {
    direction: Direction,
    input_direction: Direction,
}

struct SnakeSegment;

#[derive(Default)]
struct SnakeSegments(Vec<Entity>);

#[derive(Default)]
struct LastTailPosition(Option<Position>);

#[derive(Default)]
struct Score(u32);

#[derive(SystemLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SnakeMovement {
    Input,
    Movement,
    Eating,
    Growth,
}

struct Food;

struct GrowthEvent;

struct DeathEvent;

struct FoodSpawnEvent;

struct ScoreText;

struct Materials {
    head_material: Handle<ColorMaterial>,
    segment_material: Handle<ColorMaterial>,
    food_material: Handle<ColorMaterial>,
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Snake!".to_string(),
            width: 500.0,
            height: 500.0,
            resizable: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::hex(BACKGROUND_COLOR).unwrap()))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .insert_resource(Score::default())
        .add_startup_system(setup.system())
        .add_startup_stage(
            "game_setup",
            SystemStage::parallel()
                .with_system(spawn_snake.system())
                .with_system(food_spawner.system()),
        )
        .add_system(
            snake_movement_input
                .system()
                .label(SnakeMovement::Input)
                .before(SnakeMovement::Movement),
        )
        .add_system(game_over.system().after(SnakeMovement::Movement))
        .add_system(food_event_reader.system().after(SnakeMovement::Eating))
        .add_system(update_score_text.system().after(SnakeMovement::Growth))
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.2))
                .with_system(snake_movement.system().label(SnakeMovement::Movement))
                .with_system(
                    snake_eating
                        .system()
                        .label(SnakeMovement::Eating)
                        .after(SnakeMovement::Movement),
                )
                .with_system(
                    snake_growth
                        .system()
                        .label(SnakeMovement::Growth)
                        .after(SnakeMovement::Eating),
                ),
        )
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation.system())
                .with_system(size_scaling.system()),
        )
        .add_event::<GrowthEvent>()
        .add_event::<DeathEvent>()
        .add_event::<FoodSpawnEvent>()
        .add_plugins(DefaultPlugins)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    let gem_handle = asset_server.load("images/cookie.png");
    let head_handle = asset_server.load("images/head.png");
    commands.insert_resource(Materials {
        head_material: materials.add(head_handle.into()),
        segment_material: materials.add(Color::hex(SNAKE_COLOR).unwrap().into()),
        food_material: materials.add(gem_handle.into()),
    });
    let font = asset_server.load("fonts/arcade.ttf");
    commands
        .spawn_bundle(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: Rect {
                    top: Val::Px(10.0),
                    left: Val::Px(10.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            // Use the `Text::with_section` constructor
            text: Text::with_section(
                "0",
                TextStyle {
                    font,
                    font_size: 32.0,
                    color: Color::WHITE,
                },
                TextAlignment {
                    horizontal: HorizontalAlign::Center,
                    ..Default::default()
                },
            ),
            ..Default::default()
        })
        .insert(ScoreText);
}

fn spawn_snake(
    mut commands: Commands,
    materials: Res<Materials>,
    mut segments: ResMut<SnakeSegments>,
) {
    segments.0 = vec![
        commands
            .spawn_bundle(SpriteBundle {
                material: materials.head_material.clone(),
                sprite: Sprite::new(Vec2::new(16.0, 16.0)),
                transform: Transform::default(),
                ..Default::default()
            })
            .insert(SnakeHead {
                direction: Direction::Up,
                input_direction: Direction::Up,
            })
            .insert(SnakeSegment)
            .insert(Position { x: 3, y: 3 })
            .insert(Size::square(1.0))
            .id(),
        spawn_segment(
            commands,
            &materials.segment_material,
            Position { x: 3, y: 2 },
        ),
    ];
}

fn spawn_segment(
    mut commands: Commands,
    material: &Handle<ColorMaterial>,
    position: Position,
) -> Entity {
    commands
        .spawn_bundle(SpriteBundle {
            material: material.clone(),
            ..Default::default()
        })
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(1.0))
        .id()
}

fn snake_movement_input(keyboard_input: Res<Input<KeyCode>>, mut heads: Query<&mut SnakeHead>) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::A) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::D) {
            Direction::Right
        } else if keyboard_input.pressed(KeyCode::S) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::W) {
            Direction::Up
        } else {
            head.input_direction
        };
        if dir != head.direction.opposite() {
            head.input_direction = dir;
        }
    }
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut heads: Query<(Entity, &mut Transform, &mut SnakeHead)>,
    mut positions: Query<&mut Position>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut death_writer: EventWriter<DeathEvent>,
) {
    if let Some((head_entity, mut head_transform, mut head)) = heads.iter_mut().next() {
        let segment_positions = segments
            .0
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = positions.get_mut(head_entity).unwrap();
        head.direction = head.input_direction;
        match head.direction {
            Direction::Left => {
                head_pos.x -= 1;
                head_transform.rotation = Quat::from_rotation_z((PI / 2.0) as f32)
            }
            Direction::Right => {
                head_pos.x += 1;
                head_transform.rotation = Quat::from_rotation_z((PI * 1.5) as f32)
            }
            Direction::Up => {
                head_pos.y += 1;
                head_transform.rotation = Quat::from_rotation_z(0.0)
            }
            Direction::Down => {
                head_pos.y -= 1;
                head_transform.rotation = Quat::from_rotation_z(PI as f32)
            }
        };
        if head_pos.x < 0
            || head_pos.y < 0
            || head_pos.x as u32 >= ARENA_WIDTH
            || head_pos.y as u32 >= ARENA_HEIGHT
        {
            death_writer.send(DeathEvent);
        }
        if segment_positions.contains(&head_pos) {
            death_writer.send(DeathEvent);
        }
        segment_positions
            .iter()
            .zip(segments.0.iter().skip(1))
            .for_each(|(pos, segment)| {
                *positions.get_mut(*segment).unwrap() = *pos;
            });
        last_tail_position.0 = Some(*segment_positions.last().unwrap());
    }
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<DeathEvent>,
    materials: Res<Materials>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
    mut food_spawner: EventWriter<FoodSpawnEvent>,
    mut score: ResMut<Score>,
    mut text_query: Query<&mut Text, With<ScoreText>>,
) {
    if reader.iter().next().is_some() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_snake(commands, materials, segments_res);
        food_spawner.send(FoodSpawnEvent);
        score.0 = 0;
        for mut text in text_query.iter_mut() {
            text.sections[0].value = format!("{}", score.0)
        }
    }
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    mut food_spawner: EventWriter<FoodSpawnEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
                food_spawner.send(FoodSpawnEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
    mut score: ResMut<Score>,
    materials: Res<Materials>,
) {
    if growth_reader.iter().next().is_some() {
        segments.0.push(spawn_segment(
            commands,
            &materials.segment_material,
            last_tail_position.0.unwrap(),
        ));
        score.0 += 1;
    }
}

fn food_event_reader(
    commands: Commands,
    mut reader: EventReader<FoodSpawnEvent>,
    materials: Res<Materials>,
    blockers: Query<&Position, Or<(With<Food>, With<SnakeSegment>)>>,
) {
    if reader.iter().next().is_some() {
        food_spawner(commands, materials, blockers)
    }
}

fn food_spawner(
    mut commands: Commands,
    materials: Res<Materials>,
    blockers: Query<&Position, Or<(With<Food>, With<SnakeSegment>)>>,
) {
    fn get_empty_pos(blockers: Vec<&Position>) -> Position {
        fn random_pos() -> Position {
            Position {
                x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
                y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
            }
        }
        let mut pos = random_pos();
        while blockers.iter().any(|x| x == &&pos) {
            pos = random_pos()
        }
        pos
    }
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite::new(Vec2::new(16.0, 16.0)),
            material: materials.food_material.clone(),
            transform: Transform::from_rotation(Quat::from_rotation_z(random::<f32>())),
            ..Default::default()
        })
        .insert(Food)
        .insert(get_empty_pos(blockers.iter().collect()))
        .insert(Size::square(1.0));
}

fn update_score_text(
    mut query: Query<&mut Text, With<ScoreText>>,
    score: Res<Score>,
    mut reader: EventReader<GrowthEvent>,
) {
    if reader.iter().next().is_some() {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("{}", score.0)
        }
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Sprite)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut sprite) in q.iter_mut() {
        sprite.size = Vec2::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
        )
    }
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Position, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.0) + (tile_size / 2.0)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        )
    }
}
