//! Input de ações do jogador — clique-pra-mover, descer, pulo pra frente (feature-plugin).

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use shared::world::{is_stair_a, is_stair_b, FLOOR_LEVEL, ROOF_LEVEL};
use shared::{Elevation, Hop, Jump, MoveTo, Player};

use crate::common::{MyPlayer, LEVEL_HEIGHT};
use crate::app::ClientSet;

/// Registra os sistemas de input nos sets de ordenação (`Input`/`Anim`).
pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<HopAnim>().add_systems(
        Update,
        (
            (read_right_click, read_jump, read_hop).in_set(ClientSet::Input),
            hop_arc.in_set(ClientSet::Anim),
        ),
    );
}

/// Botão DIREITO **segurado** → click-to-move. Lança um raio da câmera e testa dois planos:
/// se aponta pro TELHADO (altura do telhado, dentro da casa) → manda subir (elevação-alvo
/// = telhado); senão, mira o chão no NÍVEL ATUAL do boneco. (Pra descer, use Enter.)
fn read_right_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    me: Res<MyPlayer>,
    players: Query<(&Player, &Elevation)>,
    mut last_sent: Local<Option<(u32, u32, i32)>>,
) {
    if !buttons.pressed(MouseButton::Right) {
        *last_sent = None; // solta o botão → próximo clique pode reenviar
        return;
    }
    let (Ok(window), Ok((camera, camera_tf))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_tf, cursor) else {
        return;
    };
    let dir = ray.direction;
    if dir.y.abs() < 1e-4 {
        return;
    }
    let own_elev = me
        .0
        .and_then(|id| players.iter().find(|(p, _)| p.0 == id))
        .map_or(0, |(_, e)| e.0);

    // Célula atingida num plano horizontal de altura `h` (unidades de mundo).
    let hit_at = |h: f32| -> Option<(u32, u32)> {
        let t = (h - ray.origin.y) / dir.y;
        if t <= 0.0 {
            return None;
        }
        let p = ray.origin + ray.direction * t;
        let (x, z) = (p.x.round(), p.z.round());
        (x >= 0.0 && z >= 0.0).then_some((x as u32, z as u32))
    };

    // Raio no plano do nível ATUAL do boneco → célula alvo (respeita o que você vê).
    let Some((gx, gz)) = hit_at(own_elev as f32 * LEVEL_HEIGHT) else {
        return;
    };
    // Mudar de nível SÓ clicando numa ESCADA: lance A (térreo↔2º andar), lance B (2º
    // andar↔telhado). Clicar o CHÃO anda no nível atual (não sobe mais sem querer).
    let flight_a = is_stair_a(gx, gz);
    let flight_b = is_stair_b(gx, gz);
    let te = if own_elev < FLOOR_LEVEL && flight_a {
        FLOOR_LEVEL // térreo → sobe pelo lance A ao 2º andar
    } else if own_elev == FLOOR_LEVEL && flight_b {
        ROOF_LEVEL // 2º andar → sobe pelo lance B ao telhado
    } else if own_elev == FLOOR_LEVEL && flight_a {
        0 // 2º andar → desce pelo lance A ao térreo
    } else if own_elev == ROOF_LEVEL && flight_b {
        FLOOR_LEVEL // telhado → desce pelo lance B ao 2º andar
    } else {
        own_elev // anda no nível atual
    };
    // Só reenvia quando o alvo MUDA (evita spammar MoveTo + A* no servidor todo frame).
    let msg = (gx, gz, te);
    if *last_sent != Some(msg) {
        *last_sent = Some(msg);
        commands.client_trigger(MoveTo { tx: gx, ty: gz, te });
    }
}

/// Tecla **Enter** → DESCER um nível (o servidor decide se há onde cair).
fn read_jump(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Enter) {
        commands.client_trigger(Jump);
    }
}

/// Arco visual do pulo pra frente (tempo restante da animação do PRÓPRIO player).
#[derive(Resource, Default)]
struct HopAnim(f32);

const HOP_DUR: f32 = 0.45;
const HOP_HEIGHT: f32 = 0.9;

/// Tecla **Espaço** → PULO PRA FRENTE (`Hop`) + dispara o arco visual local.
fn read_hop(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>, mut anim: ResMut<HopAnim>) {
    if keys.just_pressed(KeyCode::Space) {
        commands.client_trigger(Hop);
        anim.0 = HOP_DUR;
    }
}

/// Aplica o arco (parábola) na altura do sprite do PRÓPRIO player durante o pulo. Roda
/// DEPOIS de `sync_transforms` (soma por cima da altura-base do nível).
fn hop_arc(
    time: Res<Time>,
    mut anim: ResMut<HopAnim>,
    me: Res<MyPlayer>,
    mut players: Query<(&Player, &mut Transform)>,
) {
    if anim.0 <= 0.0 {
        return;
    }
    anim.0 = (anim.0 - time.delta_secs()).max(0.0);
    let Some(my_id) = me.0 else {
        return;
    };
    let p = 1.0 - anim.0 / HOP_DUR; // 0 → 1 ao longo do pulo
    let arc = 4.0 * p * (1.0 - p) * HOP_HEIGHT; // parábola, pico no meio
    for (player, mut transform) in &mut players {
        if player.0 == my_id {
            transform.translation.y += arc;
        }
    }
}
