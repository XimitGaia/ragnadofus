//! Cliente desktop do ragnadofus (Passo 2, fatia 2b).
//!
//! Zona 2.5D isométrica (`bevy_ecs_tilemap`, 64×64, diamante — D2.1/D1.5) + câmera que
//! segue o player com upscale (~2×, nearest). O player (quad placeholder, sem asset/IP)
//! anda **localmente** por WASD/setas ou clique direito. A simulação (intenção →
//! velocidade → posição) roda no `SharedPlugin`, em `FixedUpdate`.
//!
//! Elevação real (walkmap 4×4 + colisão `avian2d`), y-sort fino e oclusão por fade
//! (D1.3) entram no Passo 3. Rede (autoridade do servidor) no Passo 4.

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use shared::{MoveIntent, Player, PlayerId, Position, SharedPlugin, Velocity};

/// Distância (px) para considerar que chegou ao destino clicado.
const ARRIVE_EPS: f32 = 2.0;

/// Destino de movimento por clique (client-only; no servidor vira ordem validada +
/// pathfinding — Passo 4/5). Para onde o player caminha em linha reta.
#[derive(Component, Debug, Clone, Copy)]
struct MoveTarget(Vec2);

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // nearest-neighbor: pré-requisito de pixel-art.
                .set(ImagePlugin::default_nearest())
                // assets ficam com o crate do cliente, independente do CWD.
                .set(AssetPlugin {
                    file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                    ..default()
                }),
        )
        .add_plugins(TilemapPlugin)
        .add_plugins(SharedPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (read_right_click, update_intent, sync_sprite_to_position, camera_follow).chain(),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Zona 2.5D: 64×64 tiles isométricos diamante (D2.1/D1.5), chão de grama placeholder.
    let texture: Handle<Image> = asset_server.load("tiles.png");
    let map_size = TilemapSize { x: 64, y: 64 };
    let mut storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();
    let tilemap_id = TilemapId(tilemap_entity);
    fill_tilemap_rect(
        TileTextureIndex(0),
        TilePos { x: 0, y: 0 },
        map_size,
        tilemap_id,
        &mut commands,
        &mut storage,
    );
    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };
    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size: tile_size.into(),
        size: map_size,
        storage,
        texture: TilemapTexture::Single(texture),
        tile_size,
        map_type: TilemapType::Isometric(IsoCoordSystem::Diamond),
        anchor: TilemapAnchor::Center,
        ..default()
    });

    // Player local placeholder: quad colorido com z alto (acima do chão). y-sort fino
    // contra objetos/elevação vem no Passo 3.
    commands.spawn((
        Player(PlayerId(1)),
        Position(Vec2::ZERO),
        Velocity(Vec2::ZERO),
        MoveIntent(Vec2::ZERO),
        Sprite::from_color(Color::srgb(0.9, 0.3, 0.3), Vec2::new(16.0, 24.0)),
        Transform::from_xyz(0.0, 0.0, 100.0),
    ));
}

/// Direção de teclado (WASD/setas), não normalizada (o `shared` normaliza).
fn keyboard_dir(keys: &ButtonInput<KeyCode>) -> Vec2 {
    let mut dir = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    dir
}

/// Clique direito → define um destino (convertendo o cursor da tela para o mundo).
fn read_right_click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    players: Query<Entity, With<Player>>,
    mut commands: Commands,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let (Ok(window), Ok((camera, camera_tf)), Ok(player)) =
        (windows.single(), cameras.single(), players.single())
    else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(camera_tf, cursor) else {
        return;
    };
    commands.entity(player).insert(MoveTarget(world));
}

/// Calcula a intenção de movimento: teclado tem prioridade (e cancela o destino do
/// clique); senão, caminha em direção ao `MoveTarget` até chegar.
fn update_intent(
    keys: Res<ButtonInput<KeyCode>>,
    mut players: Query<(Entity, &Position, &mut MoveIntent, Option<&MoveTarget>), With<Player>>,
    mut commands: Commands,
) {
    let kb = keyboard_dir(&keys);
    for (entity, position, mut intent, target) in &mut players {
        if kb != Vec2::ZERO {
            intent.0 = kb;
            if target.is_some() {
                commands.entity(entity).remove::<MoveTarget>();
            }
        } else if let Some(target) = target {
            let to_target = target.0 - position.0;
            if to_target.length() <= ARRIVE_EPS {
                intent.0 = Vec2::ZERO;
                commands.entity(entity).remove::<MoveTarget>();
            } else {
                intent.0 = to_target; // shared normaliza
            }
        } else {
            intent.0 = Vec2::ZERO;
        }
    }
}

/// Espelha a `Position` (autoritativa da simulação) no `Transform` de render (mantém z).
fn sync_sprite_to_position(mut query: Query<(&Position, &mut Transform)>) {
    for (position, mut transform) in &mut query {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}

/// A câmera segue o player e aplica o upscale (~2×) da pixel-art.
fn camera_follow(
    players: Query<&Position, With<Player>>,
    mut cameras: Query<(&mut Transform, &mut Projection), (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    for (mut transform, mut projection) in &mut cameras {
        transform.translation.x = player.0.x;
        transform.translation.y = player.0.y;
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = 0.5;
        }
    }
}
