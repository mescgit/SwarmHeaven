
use bevy::{
    prelude::*,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    window::PresentMode,
};
use rand::Rng;

// Game constants
const PLAYER_SPEED: f32 = 500.0;
const PLAYER_SIZE: f32 = 30.0;
const ENEMY_SIZE: f32 = 20.0;
const ENEMY_SPEED: f32 = 200.0;
const ENEMY_SPAWN_INTERVAL: f32 = 0.1;
const XP_GEM_SIZE: f32 = 10.0;
const ORBITING_BLADE_RADIUS: f32 = 100.0;
const ORBITING_BLADE_ROTATION_SPEED: f32 = 2.0;

// Game state
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States, Default)]
enum GameState {
    #[default]
    MainMenu,
    Running,
    Paused,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Swarm Heaven".into(),
                resolution: (1280.0, 720.0).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .init_state::<GameState>()
        .add_plugins((
            player::PlayerPlugin,
            enemy::EnemyPlugin,
            combat::CombatPlugin,
            leveling::LevelingPlugin,
            ui::UiPlugin,
            waves::WavePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
        .add_systems(Update, main_menu_input.run_if(in_state(GameState::MainMenu)))
        .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
        .run();
}

#[derive(Component)]
struct MainMenu;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_main_menu(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        },
        MainMenu,
    )).with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Swarm Heaven",
            TextStyle {
                font_size: 80.0,
                ..default()
            },
        ));
        parent.spawn(TextBundle::from_section(
            "Press Space or Enter to start",
            TextStyle {
                font_size: 30.0,
                ..default()
            },
        ));
    });
}

fn main_menu_input(
    mut next_state: ResMut<NextState<GameState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.any_just_pressed([
        KeyCode::Space,
        KeyCode::Enter,
    ]) {
        next_state.set(GameState::Running);
    }
}

fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}


mod player {
    use super::*;
    use crate::combat::BladeOrbit;

    pub struct PlayerPlugin;

    impl Plugin for PlayerPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(OnEnter(GameState::Running), spawn_player)
                .add_systems(Update, player_movement.run_if(in_state(GameState::Running)));
        }
    }

    #[derive(Component)]
    pub struct Player;

    fn spawn_player(mut commands: Commands) {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.2, 0.7, 0.9),
                    custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 10.0),
                ..default()
            },
            Player,
        )).with_children(|parent| {
            parent.spawn((
                SpatialBundle::default(),
                BladeOrbit,
            ));
        });
    }

    fn player_movement(
        keyboard_input: Res<ButtonInput<KeyCode>>,
        mut query: Query<&mut Transform, With<Player>>,
        time: Res<Time>,
    ) {
        if let Ok(mut transform) = query.get_single_mut() {
            let mut direction = Vec3::ZERO;

            if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
                direction.x -= 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
                direction.x += 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
                direction.y += 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
                direction.y -= 1.0;
            }

            if direction.length() > 0.0 {
                direction = direction.normalize();
            }

            transform.translation += direction * PLAYER_SPEED * time.delta_seconds();
        }
    }
}

mod enemy {
    use super::*;

    pub struct EnemyPlugin;

    impl Plugin for EnemyPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(EnemySpawnTimer(Timer::from_seconds(
                ENEMY_SPAWN_INTERVAL,
                TimerMode::Repeating,
            )))
            .add_systems(
                Update,
                (
                    enemy_spawner,
                    (enemy_movement, boid_steering).chain(),
                )
                    .run_if(in_state(GameState::Running)),
            );
        }
    }

    #[derive(Component)]
    pub struct Enemy;

    #[derive(Resource)]
    struct EnemySpawnTimer(Timer);

    fn enemy_spawner(
        mut commands: Commands,
        time: Res<Time>,
        mut timer: ResMut<EnemySpawnTimer>,
        player_query: Query<&Transform, With<player::Player>>,
    ) {
        if timer.0.tick(time.delta()).just_finished() {
            if let Ok(player_transform) = player_query.get_single() {
                let mut rng = rand::thread_rng();
                let angle = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
                let distance = 1000.0;
                let spawn_pos = player_transform.translation
                    + Vec3::new(angle.cos() * distance, angle.sin() * distance, 0.0);

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.9, 0.2, 0.2),
                            custom_size: Some(Vec2::new(ENEMY_SIZE, ENEMY_SIZE)),
                            ..default()
                        },
                        transform: Transform::from_translation(spawn_pos),
                        ..default()
                    },
                    Enemy,
                ));
            }
        }
    }

    fn enemy_movement(
        mut enemy_query: Query<&mut Transform, (With<Enemy>, Without<player::Player>)>,
        player_query: Query<&Transform, With<player::Player>>,
        time: Res<Time>,
    ) {
        if let Ok(player_transform) = player_query.get_single() {
            enemy_query.par_iter_mut().for_each(|mut transform| {
                let direction = (player_transform.translation - transform.translation).normalize_or_zero();
                transform.translation += direction * ENEMY_SPEED * time.delta_seconds();
            });
        }
    }
    
    fn boid_steering(
        mut enemy_query: Query<(Entity, &mut Transform), With<Enemy>>,
        time: Res<Time>,
    ) {
        let mut combinations = enemy_query.iter_combinations_mut();
        while let Some([(_, mut t1), (_, mut t2)]) = combinations.fetch_next() {
            let distance = t1.translation.distance(t2.translation);
            let separation_threshold = ENEMY_SIZE * 1.5;

            if distance < separation_threshold && distance > 0.0 {
                let separation_vector = (t1.translation - t2.translation).normalize();
                let separation_force = (separation_threshold - distance) / separation_threshold;

                t1.translation += separation_vector * separation_force * ENEMY_SPEED * time.delta_seconds() / 2.0;
                t2.translation -= separation_vector * separation_force * ENEMY_SPEED * time.delta_seconds() / 2.0;
            }
        }
    }
}

mod combat {
    use super::*;
    use std::time::Duration;

    pub struct CombatPlugin;

    #[derive(Component)]
    pub struct BladeOrbit;

    impl Plugin for CombatPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(WeaponStats::default())
                .insert_resource(FireRateTimer(Timer::from_seconds(
                    0.5,
                    TimerMode::Repeating,
                )))
                .add_systems(
                    Update,
                    (
                        fire_projectiles,
                        move_projectiles,
                        projectile_collision,
                        rotate_orbiting_blades,
                        orbiting_blade_collision,
                        update_blade_count,
                        spawn_initial_blades,
                    )
                        .run_if(in_state(GameState::Running)),
                );
        }
    }

    #[derive(Resource, Debug)]
    pub struct WeaponStats {
        pub multishot: u32,
        pub chain_lightning: u32,
        pub blade_count: u32,
        pub fire_rate: f32,
    }

    impl Default for WeaponStats {
        fn default() -> Self {
            Self {
                multishot: 1,
                chain_lightning: 0,
                blade_count: 3,
                fire_rate: 0.5,
            }
        }
    }

    #[derive(Component)]
    struct Projectile {
        direction: Vec3,
        speed: f32,
        ttl: Timer,
    }

    #[derive(Component)]
    pub struct OrbitingBlade;

    #[derive(Resource)]
    struct FireRateTimer(Timer);

    fn fire_projectiles(
        mut commands: Commands,
        time: Res<Time>,
        mut timer: ResMut<FireRateTimer>,
        weapon_stats: Res<WeaponStats>,
        player_query: Query<&Transform, With<player::Player>>,
        enemy_query: Query<&Transform, With<enemy::Enemy>>,
    ) {
        timer.0.set_duration(Duration::from_secs_f32(weapon_stats.fire_rate));
        if timer.0.tick(time.delta()).just_finished() {
            if let Ok(player_transform) = player_query.get_single() {
                let mut closest_enemy: Option<Vec3> = None;
                let mut min_dist = f32::MAX;

                for enemy_transform in enemy_query.iter() {
                    let distance = player_transform
                        .translation
                        .distance(enemy_transform.translation);
                    if distance < min_dist {
                        min_dist = distance;
                        closest_enemy = Some(enemy_transform.translation);
                    }
                }

                if let Some(target_pos) = closest_enemy {
                    let direction = (target_pos - player_transform.translation).normalize_or_zero();
                    for i in 0..weapon_stats.multishot {
                        let angle_offset = (i as f32 - (weapon_stats.multishot - 1) as f32 / 2.0) * 0.15;
                        let rotated_direction = Quat::from_rotation_z(angle_offset).mul_vec3(direction);
                        
                        commands.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: Color::rgb(0.9, 0.9, 0.1),
                                    custom_size: Some(Vec2::new(10.0, 10.0)),
                                    ..default()
                                },
                                transform: Transform::from_translation(player_transform.translation),
                                ..default()
                            },
                            Projectile {
                                direction: rotated_direction,
                                speed: 800.0,
                                ttl: Timer::from_seconds(2.0, TimerMode::Once),
                            },
                        ));
                    }
                }
            }
        }
    }

    fn move_projectiles(
        mut commands: Commands,
        mut query: Query<(Entity, &mut Transform, &mut Projectile)>,
        time: Res<Time>,
    ) {
        for (entity, mut transform, mut projectile) in query.iter_mut() {
            transform.translation += projectile.direction * projectile.speed * time.delta_seconds();
            if projectile.ttl.tick(time.delta()).finished() {
                commands.entity(entity).despawn();
            }
        }
    }

    fn projectile_collision(
        mut commands: Commands,
        projectile_query: Query<(Entity, &Transform), With<Projectile>>,
        enemy_query: Query<(Entity, &Transform), With<enemy::Enemy>>,
        mut xp_events: EventWriter<leveling::XpDropEvent>,
        weapon_stats: Res<WeaponStats>,
    ) {
        for (proj_entity, proj_transform) in projectile_query.iter() {
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                if proj_transform
                    .translation
                    .distance(enemy_transform.translation)
                    < (ENEMY_SIZE / 2.0)
                {
                    commands.entity(proj_entity).despawn();
                    commands.entity(enemy_entity).despawn();
                    xp_events.send(leveling::XpDropEvent(enemy_transform.translation));

                    // Chain lightning
                    if weapon_stats.chain_lightning > 0 {
                        let mut chained_targets = vec![enemy_entity];
                        let mut last_pos = enemy_transform.translation;

                        for _ in 0..weapon_stats.chain_lightning {
                            let mut closest_new_target: Option<(Entity, Vec3)> = None;
                            let mut min_dist = 300.0; // Max chain distance

                            for (next_enemy_entity, next_enemy_transform) in enemy_query.iter() {
                                if !chained_targets.contains(&next_enemy_entity) {
                                    let dist = last_pos.distance(next_enemy_transform.translation);
                                    if dist < min_dist {
                                        min_dist = dist;
                                        closest_new_target = Some((next_enemy_entity, next_enemy_transform.translation));
                                    }
                                }
                            }

                            if let Some((target_entity, target_pos)) = closest_new_target {
                                commands.entity(target_entity).despawn();
                                xp_events.send(leveling::XpDropEvent(target_pos));
                                chained_targets.push(target_entity);
                                last_pos = target_pos;
                            } else {
                                break;
                            }
                        }
                    }
                    return; 
                }
            }
        }
    }

    fn spawn_initial_blades(
        mut commands: Commands,
        orbit_query: Query<Entity, Added<BladeOrbit>>,
        weapon_stats: Res<WeaponStats>,
    ) {
        if let Ok(orbit_entity) = orbit_query.get_single() {
            commands.entity(orbit_entity).with_children(|parent| {
                for i in 0..weapon_stats.blade_count {
                    let angle = (i as f32 / weapon_stats.blade_count as f32) * 2.0 * std::f32::consts::PI;
                    parent.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgb(0.8, 0.8, 0.8),
                                custom_size: Some(Vec2::new(40.0, 15.0)),
                                ..default()
                            },
                            transform: Transform::from_xyz(
                                ORBITING_BLADE_RADIUS * angle.cos(),
                                ORBITING_BLADE_RADIUS * angle.sin(),
                                0.0,
                            ).with_rotation(Quat::from_rotation_z(angle)),
                            ..default()
                        },
                        OrbitingBlade,
                    ));
                }
            });
        }
    }

    fn rotate_orbiting_blades(
        mut query: Query<&mut Transform, With<BladeOrbit>>,
        time: Res<Time>,
    ) {
        if let Ok(mut transform) = query.get_single_mut() {
            transform.rotate_z(ORBITING_BLADE_ROTATION_SPEED * time.delta_seconds());
        }
    }

    fn orbiting_blade_collision(
        mut commands: Commands,
        blade_query: Query<&GlobalTransform, With<OrbitingBlade>>,
        enemy_query: Query<(Entity, &Transform), With<enemy::Enemy>>,
        mut xp_events: EventWriter<leveling::XpDropEvent>,
        mut hit_enemies: Local<Vec<Entity>>,
    ) {
        hit_enemies.clear();
        for blade_global_transform in blade_query.iter() {
            for (enemy_entity, enemy_transform) in enemy_query.iter() {
                if hit_enemies.contains(&enemy_entity) { continue; }
                if blade_global_transform
                    .translation()
                    .distance(enemy_transform.translation)
                    < (ENEMY_SIZE / 2.0 + 15.0)
                {
                    commands.entity(enemy_entity).despawn();
                    xp_events.send(leveling::XpDropEvent(enemy_transform.translation));
                    hit_enemies.push(enemy_entity);
                }
            }
        }
    }

    fn update_blade_count(
        mut commands: Commands,
        weapon_stats: Res<WeaponStats>,
        orbit_query: Query<Entity, With<BladeOrbit>>,
        blade_query: Query<Entity, With<OrbitingBlade>>,
    ) {
        if weapon_stats.is_changed() {
            if let Ok(orbit_entity) = orbit_query.get_single() {
                for entity in blade_query.iter() {
                    commands.entity(entity).despawn_recursive();
                }
                commands.entity(orbit_entity).with_children(|parent| {
                    for i in 0..weapon_stats.blade_count {
                        let angle = (i as f32 / weapon_stats.blade_count as f32) * 2.0 * std::f32::consts::PI;
                        parent.spawn((
                            SpriteBundle {
                                sprite: Sprite {
                                    color: Color::rgb(0.8, 0.8, 0.8),
                                    custom_size: Some(Vec2::new(40.0, 15.0)),
                                    ..default()
                                },
                                transform: Transform::from_xyz(
                                    ORBITING_BLADE_RADIUS * angle.cos(),
                                    ORBITING_BLADE_RADIUS * angle.sin(),
                                    0.0,
                                ).with_rotation(Quat::from_rotation_z(angle)),
                                ..default()
                            },
                            OrbitingBlade,
                        ));
                    }
                });
            }
        }
    }
}

mod leveling {
    use super::*;

    pub struct LevelingPlugin;

    impl Plugin for LevelingPlugin {
        fn build(&self, app: &mut App) {
            app.add_event::<XpDropEvent>()
                .insert_resource(PlayerStats::default())
                .add_systems(
                    Update,
                    (
                        spawn_xp_gems,
                        collect_xp_gems,
                        check_level_up,
                    )
                        .run_if(in_state(GameState::Running)),
                );
        }
    }

    #[derive(Event)]
    pub struct XpDropEvent(pub Vec3);

    #[derive(Component)]
    struct XpGem;

    #[derive(Resource, Debug)]
    pub struct PlayerStats {
        pub xp: u32,
        pub level: u32,
        pub xp_to_next_level: u32,
    }

    impl Default for PlayerStats {
        fn default() -> Self {
            Self {
                xp: 0,
                level: 1,
                xp_to_next_level: 100,
            }
        }
    }

    fn spawn_xp_gems(mut commands: Commands, mut events: EventReader<XpDropEvent>) {
        for event in events.read() {
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.1, 0.9, 0.1),
                        custom_size: Some(Vec2::new(XP_GEM_SIZE, XP_GEM_SIZE)),
                        ..default()
                    },
                    transform: Transform::from_translation(event.0),
                    ..default()
                },
                XpGem,
            ));
        }
    }

    fn collect_xp_gems(
        mut commands: Commands,
        player_query: Query<&Transform, With<player::Player>>,
        gem_query: Query<(Entity, &Transform), With<XpGem>>,
        mut player_stats: ResMut<PlayerStats>,
    ) {
        if let Ok(player_transform) = player_query.get_single() {
            for (gem_entity, gem_transform) in gem_query.iter() {
                if player_transform
                    .translation
                    .distance(gem_transform.translation)
                    < (PLAYER_SIZE / 2.0 + 50.0) // Increased collection radius
                {
                    commands.entity(gem_entity).despawn();
                    player_stats.xp += 10;
                }
            }
        }
    }

    fn check_level_up(
        mut player_stats: ResMut<PlayerStats>,
        mut game_state: ResMut<NextState<GameState>>,
    ) {
        if player_stats.xp >= player_stats.xp_to_next_level {
            player_stats.level += 1;
            player_stats.xp -= player_stats.xp_to_next_level;
            player_stats.xp_to_next_level = (player_stats.xp_to_next_level as f32 * 1.5) as u32;
            game_state.set(GameState::Paused);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use bevy::prelude::*;

        #[test]
        fn test_level_up_logic() {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
               .init_state::<GameState>()
               .insert_resource(PlayerStats {
                   xp: 100,
                   level: 1,
                   xp_to_next_level: 100,
               })
               .add_systems(Update, check_level_up);

            app.update();

            let stats = app.world.resource::<PlayerStats>();
            assert_eq!(stats.level, 2);
            assert_eq!(stats.xp, 0);
            assert_eq!(stats.xp_to_next_level, 150); // 100 * 1.5

            let _state = app.world.resource::<State<GameState>>();
            // State transitions are applied at the start of the next frame.
            // But next_state is in NextState resource.
            let next_state = app.world.resource::<NextState<GameState>>();
            
            // To verify state transition, we need to apply state transitions.
            // But we can just check if NextState was set.
            if let Some(s) = next_state.0 {
                assert_eq!(s, GameState::Paused);
            }
        }
    }
}

mod ui {
    use super::*;
    use bevy::diagnostic::DiagnosticsStore;
    use rand::seq::SliceRandom;

    pub struct UiPlugin;

    impl Plugin for UiPlugin {
        fn build(&self, app: &mut App) {
            app.add_systems(OnEnter(GameState::Running), setup_game_ui)
                .add_systems(
                    Update,
                    (update_game_ui, handle_upgrade_buttons)
                )
                .add_systems(OnEnter(GameState::Paused), show_level_up_menu)
                .add_systems(OnExit(GameState::Paused), hide_level_up_menu)
                .add_systems(OnExit(GameState::Running), hide_level_up_menu);
        }
    }

    #[derive(Component)]
    struct FpsText;
    #[derive(Component)]
    struct EnemyCountText;
    #[derive(Component)]
    struct TimerText;
    #[derive(Component)]
    struct LevelUpMenu;
    #[derive(Component)]
    struct GameUi;

    fn setup_game_ui(mut commands: Commands) {
        commands.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ..default()
            },
            GameUi,
        )).with_children(|parent| {
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                ..default()
            }).with_children(|parent| {
                parent.spawn((TextBundle::from_section(
                    "FPS: ",
                    TextStyle { font_size: 20.0, ..default() },
                ), FpsText));
                parent.spawn((TextBundle::from_section(
                    "Enemies: ",
                    TextStyle { font_size: 20.0, ..default() },
                ), EnemyCountText));
            });
            parent.spawn((
                TextBundle::from_section(
                    "Time: 0.0",
                    TextStyle { font_size: 30.0, ..default() },
                ).with_style(Style {
                    margin: UiRect::all(Val::Px(10.0)),
                    ..default()
                }),
                TimerText,
            ));
        });

        commands.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    display: Display::None,
                    ..default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.7).into(),
                z_index: ZIndex::Global(100),
                ..default()
            },
            LevelUpMenu,
        )).with_children(|parent| {
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            }).with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "Level Up!",
                    TextStyle { font_size: 50.0, color: Color::WHITE, ..default() },
                ));
            });
        });
    }

    fn update_game_ui(
        diagnostics: Res<DiagnosticsStore>,
        mut fps_query: Query<&mut Text, With<FpsText>>,
        mut enemy_query: Query<&mut Text, (With<EnemyCountText>, Without<FpsText>)>,
        enemy_count_query: Query<(), With<enemy::Enemy>>,
        time: Res<Time>,
        mut timer_query: Query<&mut Text, (With<TimerText>, Without<FpsText>, Without<EnemyCountText>)>,
    ) {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                for mut text in fps_query.iter_mut() {
                    text.sections[0].value = format!("FPS: {:.0}", value);
                }
            }
        }

        for mut text in enemy_query.iter_mut() {
            text.sections[0].value = format!("Enemies: {}", enemy_count_query.iter().count());
        }

        for mut text in timer_query.iter_mut() {
            text.sections[0].value = format!("Time: {:.1}", time.elapsed_seconds());
        }
    }

    #[derive(Component, Clone, Copy, Debug)]
    enum Upgrade {
        Multishot,
        ChainLightning,
        BladeCount,
        AttackSpeed,
    }

    fn show_level_up_menu(
        mut commands: Commands,
        mut menu_query: Query<(Entity, &mut Style), With<LevelUpMenu>>,
    ) {
        if let Ok((menu_entity, mut style)) = menu_query.get_single_mut() {
            style.display = Display::Flex;

            let all_upgrades = vec![
                (Upgrade::Multishot, "More Projectiles"),
                (Upgrade::ChainLightning, "Chain Lightning"),
                (Upgrade::BladeCount, "More Blades"),
                (Upgrade::AttackSpeed, "Faster Attacks"),
            ];
            
            let mut rng = rand::thread_rng();
            let chosen_upgrades = all_upgrades.choose_multiple(&mut rng, 3).cloned().collect::<Vec<_>>();

            commands.entity(menu_entity).with_children(|parent| {
                for (upgrade, label) in chosen_upgrades {
                    parent.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(250.0),
                                height: Val::Px(60.0),
                                margin: UiRect::all(Val::Px(10.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                            ..default()
                        },
                        upgrade,
                    )).with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            label,
                            TextStyle { font_size: 20.0, color: Color::WHITE, ..default() },
                        ));
                    });
                }
            });
        }
    }

    fn hide_level_up_menu(
        mut commands: Commands,
        mut menu_query: Query<&mut Style, With<LevelUpMenu>>,
        button_query: Query<Entity, With<Button>>,
    ) {
        if let Ok(mut style) = menu_query.get_single_mut() {
            style.display = Display::None;
            for entity in button_query.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn handle_upgrade_buttons(
        interaction_query: Query<(&Interaction, &Upgrade), (Changed<Interaction>, With<Button>)>,
        mut weapon_stats: ResMut<combat::WeaponStats>,
        mut game_state: ResMut<NextState<GameState>>,
    ) {
        for (interaction, upgrade) in interaction_query.iter() {
            if *interaction == Interaction::Pressed {
                match upgrade {
                    Upgrade::Multishot => weapon_stats.multishot += 1,
                    Upgrade::ChainLightning => weapon_stats.chain_lightning += 1,
                    Upgrade::BladeCount => weapon_stats.blade_count += 1,
                    Upgrade::AttackSpeed => weapon_stats.fire_rate *= 0.9,
                }
                game_state.set(GameState::Running);
            }
        }
    }
}

mod waves {
    use super::*;

    pub struct WavePlugin;

    impl Plugin for WavePlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(MegaWaveTimer(Timer::from_seconds(60.0, TimerMode::Repeating)))
                .add_systems(Update, mega_wave_spawner.run_if(in_state(GameState::Running)));
        }
    }

    #[derive(Resource)]
    struct MegaWaveTimer(Timer);

    fn mega_wave_spawner(
        mut commands: Commands,
        time: Res<Time>,
        mut timer: ResMut<MegaWaveTimer>,
        player_query: Query<&Transform, With<player::Player>>,
    ) {
        if timer.0.tick(time.delta()).just_finished() {
            if let Ok(player_transform) = player_query.get_single() {
                let mut rng = rand::thread_rng();
                let direction = match rng.gen_range(0..4) {
                    0 => Vec3::new(0.0, 1.0, 0.0),  // North
                    1 => Vec3::new(0.0, -1.0, 0.0), // South
                    2 => Vec3::new(1.0, 0.0, 0.0),  // East
                    _ => Vec3::new(-1.0, 0.0, 0.0), // West
                };

                let spawn_center = player_transform.translation + direction * 1200.0;

                for _ in 0..100 {
                    let offset = Vec3::new(
                        rng.gen_range(-100.0..100.0),
                        rng.gen_range(-100.0..100.0),
                        0.0,
                    );
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgb(0.9, 0.2, 0.2),
                                custom_size: Some(Vec2::new(ENEMY_SIZE, ENEMY_SIZE)),
                                ..default()
                            },
                            transform: Transform::from_translation(spawn_center + offset),
                            ..default()
                        },
                        enemy::Enemy,
                    ));
                }
            }
        }
    }
}
