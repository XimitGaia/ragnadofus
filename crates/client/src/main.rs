//! Cliente desktop do ragnadofus (Passo 2, fatia 2a).
//!
//! Janela + câmera 2D + um player (quad colorido placeholder, sem asset/IP) que anda
//! **localmente** por **WASD/setas** ou **clique direito do mouse** (vai em linha reta
//! até o ponto; pathfinding real = Passo 5). O cliente só traduz input em [`MoveIntent`];
//! a simulação (intenção → velocidade → posição) roda no `SharedPlugin`, em `FixedUpdate`.
//! Zona (tilemap 2.5D), y-sort e câmera de upscale inteiro entram na fatia 2b.

use bevy::prelude::*;
use shared::{MoveIntent, Player, PlayerId, Position, SharedPlugin, Velocity};

/// Distância (px) para considerar que chegou ao destino clicado.
const ARRIVE_EPS: f32 = 2.0;

/// Destino de movimento por clique (client-only; no servidor isso vira ordem validada +
/// pathfinding — Passo 4/5). Marca para onde o player deve caminhar em linha reta.
#[derive(Component, Debug, Clone, Copy)]
struct MoveTarget(Vec2);

fn main() {
    App::new()
        // `default_nearest`: amostragem nearest-neighbor, pré-requisito de pixel-art.
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SharedPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (read_right_click, update_intent, sync_sprite_to_position).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Player local placeholder: retângulo colorido (sem textura/IP).
    commands.spawn((
        Player(PlayerId(1)),
        Position(Vec2::ZERO),
        Velocity(Vec2::ZERO),
        MoveIntent(Vec2::ZERO),
        Sprite::from_color(Color::srgb(0.9, 0.3, 0.3), Vec2::new(16.0, 24.0)),
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

/// Espelha a `Position` (autoritativa da simulação) no `Transform` de render.
fn sync_sprite_to_position(mut query: Query<(&Position, &mut Transform)>) {
    for (position, mut transform) in &mut query {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}
