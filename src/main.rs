use bevy::prelude::*;
use rand::Rng;

const WINDOW_W: f32 = 800.0;
const WINDOW_H: f32 = 512.0;

// Fixed-step game logic at 60 FPS for smooth physics
const FIXED_HZ: f64 = 60.0;

// Bird
const BIRD_SIZE: Vec2 = Vec2::new(34.0, 24.0); // matches bird.png size
const BIRD_START_X: f32 = -150.0;
const BIRD_START_Y: f32 = 0.0;
const GRAVITY: f32 = -980.0; // px / s^2 (slightly reduced for better feel)
const FLAP_VELOCITY: f32 = 340.0; // px / s (strong upward impulse)
const MAX_FALL_SPEED: f32 = -500.0; // Limit fall speed so it doesn't feel too heavy

// Pipes
const PIPE_WIDTH: f32 = 80.0;
const PIPE_SPEED: f32 = -150.0; // px / s (to the left)
const PIPE_GAP: f32 = 150.0; // vertical gap
const PIPE_SPAWN_INTERVAL: f32 = 1.6; // seconds between spawns
const PIPE_SPAWN_X: f32 = WINDOW_W * 0.5 + 60.0;
const PIPE_DESPAWN_X: f32 = -WINDOW_W * 0.5 - 100.0;
const GAP_MARGIN: f32 = 32.0; // margin from top/bottom so gaps aren't unfair

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
enum GameState {
    #[default]
    Menu,
    Playing,
    GameOver,
}

#[derive(Component)]
struct Bird {
    vy: f32,
    anim_timer: Timer,
}

#[derive(Resource, Default)]
struct BirdTexture(Handle<Image>);

#[derive(Resource, Default)]
struct MusicState {
    muted: bool,
}

#[derive(Component)]
struct MuteIcon;

#[derive(Component)]
struct Pipe {
    is_top: bool,
    // Only bottom pipe tracks score to avoid double count
    scored: bool,
}

#[derive(Resource, Default)]
struct Score(u32);

#[derive(Resource)]
struct PipeSpawnTimer(Timer);

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct MenuUI;

#[derive(Component)]
struct GameOverUI;

// Resource to buffer flap input from Update to FixedUpdate
#[derive(Resource, Default)]
struct FlapInput {
    requested: bool,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92))) // light sky blue fallback
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Floopy Birb".to_string(),
                resolution: (WINDOW_W, WINDOW_H).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        // Fixed timestep for game logic at 60 Hz for smooth physics
        .insert_resource(Time::<Fixed>::from_hz(FIXED_HZ))
        .init_state::<GameState>()
        .insert_resource(Score::default())
        .insert_resource(FlapInput::default())
        .insert_resource(MusicState::default())
        .add_systems(Startup, (load_assets, setup, start_music).chain())
        // Menu
        .add_systems(OnEnter(GameState::Menu), show_menu_ui)
        .add_systems(OnExit(GameState::Menu), despawn_menu_ui)
        .add_systems(Update, menu_input.run_if(in_state(GameState::Menu)))
        // Playing - input handling in Update to catch all key presses
        .add_systems(OnEnter(GameState::Playing), start_game)
        .add_systems(
            Update,
            buffer_flap_input.run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            (
                handle_flap_input,
                animate_bird,
                apply_bird_physics,
                move_pipes,
                spawn_pipes,
                check_collisions_and_scoring,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, (update_score_text, toggle_mute))
        // Game Over
        .add_systems(OnEnter(GameState::GameOver), show_game_over_ui)
        .add_systems(OnExit(GameState::GameOver), despawn_game_over_ui)
        .add_systems(
            Update,
            game_over_input.run_if(in_state(GameState::GameOver)),
        )
        .run();
}

// --------------------------------------------
// Startup
// --------------------------------------------

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bird_handle = asset_server.load("textures/bird.png");
    commands.insert_resource(BirdTexture(bird_handle));
}

fn start_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(AudioBundle {
        source: asset_server.load("music/music.ogg"),
        settings: PlaybackSettings::LOOP,
    });
}

fn toggle_mute(
    input: Res<ButtonInput<KeyCode>>,
    mut music_state: ResMut<MusicState>,
    music_sinks: Query<&AudioSink>,
    mut mute_icon_q: Query<&mut Text, With<MuteIcon>>,
) {
    if input.just_pressed(KeyCode::KeyM) {
        music_state.muted = !music_state.muted;

        for sink in &music_sinks {
            if music_state.muted {
                sink.pause();
            } else {
                sink.play();
            }
        }

        // Update mute text
        if let Ok(mut text) = mute_icon_q.get_single_mut() {
            if let Some(section) = text.sections.get_mut(0) {
                section.value = if music_state.muted {
                    "[M] OFF".to_string()
                } else {
                    "[M] ON".to_string()
                };
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    bird_texture: Res<BirdTexture>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Background (z = -10 to render behind everything)
    commands.spawn(SpriteBundle {
        texture: asset_server.load("textures/background.png"),
        transform: Transform::from_xyz(0.0, 0.0, -10.0),
        sprite: Sprite {
            custom_size: Some(Vec2::new(WINDOW_W, WINDOW_H)),
            ..default()
        },
        ..default()
    });

    // Bird sprite sheet (3 frames in a row, 34x24 each)
    let layout = TextureAtlasLayout::from_grid(UVec2::new(34, 24), 3, 1, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.spawn((
        SpriteBundle {
            texture: bird_texture.0.clone(),
            transform: Transform::from_xyz(BIRD_START_X, BIRD_START_Y, 1.0),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout,
            index: 1,
        },
        Bird {
            vy: 0.0,
            anim_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        },
    ));

    // Score text (top-center)
    commands.spawn((
        TextBundle::from_section(
            "0",
            TextStyle {
                font_size: 40.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(WINDOW_W * 0.5 - 10.0),
            ..default()
        }),
        ScoreText,
    ));

    // Mute text (top-right)
    commands.spawn((
        TextBundle::from_section(
            "[M] ON",
            TextStyle {
                font_size: 24.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        }),
        MuteIcon,
    ));
}

// --------------------------------------------
// Menu UI and input
// --------------------------------------------

fn show_menu_ui(mut commands: Commands) {
    // Title
    commands.spawn((
        TextBundle::from_section(
            "Floopy Birb",
            TextStyle {
                font_size: 56.0,
                color: Color::BLACK,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(140.0),
            left: Val::Px(WINDOW_W * 0.5 - 160.0),
            ..default()
        }),
        MenuUI,
    ));
    // Instructions
    commands.spawn((
        TextBundle::from_section(
            "Press Space to Start\nSpace to flap",
            TextStyle {
                font_size: 28.0,
                color: Color::BLACK,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(220.0),
            left: Val::Px(WINDOW_W * 0.5 - 140.0),
            ..default()
        }),
        MenuUI,
    ));
}

fn despawn_menu_ui(mut commands: Commands, q: Query<Entity, With<MenuUI>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

fn menu_input(input: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Playing);
    }
}

// --------------------------------------------
// Game start/reset
// --------------------------------------------

fn start_game(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut bird_q: Query<(&mut Transform, &mut Bird)>,
    pipes_q: Query<Entity, With<Pipe>>,
    mut flap_input: ResMut<FlapInput>,
) {
    // Reset score
    score.0 = 0;

    // Clear any buffered flap input
    flap_input.requested = false;

    // Reset bird
    if let Ok((mut tf, mut bird)) = bird_q.get_single_mut() {
        tf.translation.x = BIRD_START_X;
        tf.translation.y = BIRD_START_Y;
        bird.vy = 0.0;
        bird.anim_timer.reset();
    }

    // Despawn existing pipes
    for e in &pipes_q {
        commands.entity(e).despawn_recursive();
    }

    // Reset spawn timer
    commands.insert_resource(PipeSpawnTimer(Timer::from_seconds(
        PIPE_SPAWN_INTERVAL,
        TimerMode::Repeating,
    )));
}

// --------------------------------------------
// Playing: input, physics, spawn, movement
// --------------------------------------------

// Buffer input in Update so we never miss a key press
fn buffer_flap_input(input: Res<ButtonInput<KeyCode>>, mut flap_input: ResMut<FlapInput>) {
    if input.just_pressed(KeyCode::Space) {
        flap_input.requested = true;
    }
}

// Consume buffered input in FixedUpdate
fn handle_flap_input(mut flap_input: ResMut<FlapInput>, mut bird_q: Query<&mut Bird>) {
    if flap_input.requested {
        if let Ok(mut bird) = bird_q.get_single_mut() {
            // Flap - set velocity directly for consistent jump height
            bird.vy = FLAP_VELOCITY;
        }
        flap_input.requested = false;
    }
}

fn animate_bird(time: Res<Time<Fixed>>, mut q: Query<(&mut Bird, &mut TextureAtlas)>) {
    if let Ok((mut bird, mut atlas)) = q.get_single_mut() {
        bird.anim_timer.tick(time.delta());
        if bird.anim_timer.just_finished() {
            atlas.index = (atlas.index + 1) % 3;
        }
    }
}

fn apply_bird_physics(time: Res<Time<Fixed>>, mut q: Query<(&mut Transform, &mut Bird)>) {
    if let Ok((mut tf, mut bird)) = q.get_single_mut() {
        let dt = time.delta_seconds();

        // Apply gravity
        bird.vy += GRAVITY * dt;

        // Clamp fall speed so bird doesn't feel too heavy
        if bird.vy < MAX_FALL_SPEED {
            bird.vy = MAX_FALL_SPEED;
        }

        // Update position
        tf.translation.y += bird.vy * dt;
    }
}

fn spawn_pipes(mut commands: Commands, time: Res<Time<Fixed>>, mut timer: ResMut<PipeSpawnTimer>) {
    // Tick spawn timer with fixed dt
    if timer.0.tick(time.delta()).just_finished() {
        // Choose a random gap center
        // Keep some margin from the top and bottom edges
        let half_h = WINDOW_H * 0.5;
        let min_center = -half_h + GAP_MARGIN + PIPE_GAP * 0.5;
        let max_center = half_h - GAP_MARGIN - PIPE_GAP * 0.5;
        let mut rng = rand::thread_rng();
        let gap_center_y = rng.gen_range(min_center..=max_center);

        // Compute segment heights
        let top_height = half_h - (gap_center_y + PIPE_GAP * 0.5);
        let bottom_height = half_h + (gap_center_y - PIPE_GAP * 0.5);

        let top_center_y = half_h - top_height * 0.5;
        let bottom_center_y = -half_h + bottom_height * 0.5;

        // Dark purple/maroon color to match the floor of the background
        let pipe_color = Color::srgb(0.2, 0.024, 0.176);

        // Top pipe
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: pipe_color,
                    custom_size: Some(Vec2::new(PIPE_WIDTH, top_height)),
                    ..default()
                },
                transform: Transform::from_xyz(PIPE_SPAWN_X, top_center_y, 0.0),
                ..default()
            },
            Pipe {
                is_top: true,
                scored: false,
            },
        ));

        // Bottom pipe
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: pipe_color,
                    custom_size: Some(Vec2::new(PIPE_WIDTH, bottom_height)),
                    ..default()
                },
                transform: Transform::from_xyz(PIPE_SPAWN_X, bottom_center_y, 0.0),
                ..default()
            },
            Pipe {
                is_top: false,
                scored: false,
            },
        ));
    }
}

fn move_pipes(
    time: Res<Time<Fixed>>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform), With<Pipe>>,
) {
    let dt = time.delta_seconds();
    for (e, mut tf) in &mut q {
        tf.translation.x += PIPE_SPEED * dt;

        if tf.translation.x < PIPE_DESPAWN_X {
            commands.entity(e).despawn_recursive();
        }
    }
}

fn check_collisions_and_scoring(
    mut next_state: ResMut<NextState<GameState>>,
    mut score: ResMut<Score>,
    mut pipes: Query<(&Transform, &Sprite, &mut Pipe)>,
    bird_q: Query<&Transform, With<Bird>>,
) {
    let Ok(bird_tf) = bird_q.get_single() else {
        return;
    };

    // Floor / ceiling
    let half_h = WINDOW_H * 0.5;
    let bird_top = bird_tf.translation.y + BIRD_SIZE.y * 0.5;
    let bird_bottom = bird_tf.translation.y - BIRD_SIZE.y * 0.5;

    if bird_bottom <= -half_h || bird_top >= half_h {
        next_state.set(GameState::GameOver);
        return;
    }

    // Pipes
    let bird_pos = bird_tf.translation.truncate();
    let bird_half = BIRD_SIZE * 0.5;

    for (tf, sprite, mut pipe) in &mut pipes {
        let size = sprite.custom_size.unwrap_or(Vec2::splat(1.0));
        let pipe_pos = tf.translation.truncate();
        let pipe_half = size * 0.5;

        // AABB overlap
        let overlap_x = (bird_pos.x - pipe_pos.x).abs() <= (bird_half.x + pipe_half.x);
        let overlap_y = (bird_pos.y - pipe_pos.y).abs() <= (bird_half.y + pipe_half.y);

        if overlap_x && overlap_y {
            next_state.set(GameState::GameOver);
            return;
        }

        // Scoring: only once per bottom pipe
        if !pipe.is_top && !pipe.scored {
            let pipe_right = pipe_pos.x + pipe_half.x;
            let bird_left = bird_pos.x - bird_half.x;
            if pipe_right < bird_left {
                score.0 += 1;
                pipe.scored = true;
            }
        }
    }
}

// --------------------------------------------
// Score UI
// --------------------------------------------

fn update_score_text(score: Res<Score>, mut q: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }
    if let Ok(mut text) = q.get_single_mut() {
        if let Some(section) = text.sections.get_mut(0) {
            section.value = score.0.to_string();
        }
    }
}

// --------------------------------------------
// Game Over UI and input
// --------------------------------------------

fn show_game_over_ui(mut commands: Commands, score: Res<Score>) {
    // Game over text
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Game Over\n",
                TextStyle {
                    font_size: 56.0,
                    color: Color::srgb(1.0, 0.0, 0.0),
                    ..default()
                },
            ),
            TextSection::new(
                format!("Score: {}\n\nPress Space or R to Retry", score.0),
                TextStyle {
                    font_size: 28.0,
                    color: Color::BLACK,
                    ..default()
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(160.0),
            left: Val::Px(WINDOW_W * 0.5 - 220.0),
            ..default()
        }),
        GameOverUI,
    ));
}

fn despawn_game_over_ui(mut commands: Commands, q: Query<Entity, With<GameOverUI>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

fn game_over_input(input: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if input.just_pressed(KeyCode::Space) || input.just_pressed(KeyCode::KeyR) {
        next_state.set(GameState::Playing);
    }
}
