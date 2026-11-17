//! Visão do boneco — FOV / cutaway por ambiente (feature-plugin).

use std::collections::HashSet;

use bevy::prelude::*;
use shared::world::{HOUSE_X0, HOUSE_X1, HOUSE_Y0, HOUSE_Y1, ROOF_LEVEL};
use shared::{Elevation, Npc, Player, Position, Walkmap};

use crate::common::{Cell, DimPair, HouseWall, MyPlayer, Platform};
use crate::app::ClientSet;

/// Registra o estado de visão e o sistema que o aplica (no set `Vision`).
pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<VisionState>()
        .add_systems(Update, apply_vision.in_set(ClientSet::Vision));
}

/// Estado do campo de visão (FOV) do próprio boneco.
#[derive(Resource, Default)]
struct VisionState {
    inside: bool,
    /// Última célula do boneco (recalcula o FOV só quando ele muda de tile).
    last_cell: Option<(i32, i32)>,
    /// Células atualmente visíveis pelo boneco.
    fov: HashSet<(u32, u32)>,
}

/// Raio de visão do boneco, em tiles.
const FOV_RADIUS: i32 = 9;

/// Está no footprint (retângulo) da casa?
pub(crate) fn in_house(x: u32, y: u32) -> bool {
    (HOUSE_X0..=HOUSE_X1).contains(&x) && (HOUSE_Y0..=HOUSE_Y1).contains(&y)
}

/// Linha de visão livre de `a` a `b`: nenhuma PAREDE (célula não-caminhável) no meio
/// bloqueia. Amostra as células ao longo da reta (endpoints excluídos).
fn los_clear(map: &Walkmap, a: (i32, i32), b: (i32, i32)) -> bool {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs());
    for i in 1..steps {
        let t = i as f32 / steps as f32;
        let cx = (a.0 as f32 + t * (b.0 - a.0) as f32).round() as i32;
        let cy = (a.1 as f32 + t * (b.1 - a.1) as f32).round() as i32;
        if cx >= 0 && cy >= 0 {
            if let Some(cell) = map.get(cx as u32, cy as u32) {
                if !cell.walkable {
                    return false; // parede no caminho -> corta a vista
                }
            }
        }
    }
    true
}

/// Campo de visão do boneco: células dentro do `radius` COM linha de visão livre. A parede
/// atingida é visível; o que está ATRÁS dela não (ex.: parte de trás da casa some).
fn compute_fov(map: &Walkmap, origin: (i32, i32), radius: i32) -> HashSet<(u32, u32)> {
    let mut vis = HashSet::new();
    let (ox, oy) = origin;
    if ox < 0 || oy < 0 {
        return vis;
    }
    vis.insert((ox as u32, oy as u32));
    let (w, h) = (map.width() as i32, map.height() as i32);
    for ty in (oy - radius).max(0)..=(oy + radius).min(h - 1) {
        for tx in (ox - radius).max(0)..=(ox + radius).min(w - 1) {
            let (dx, dy) = (tx - ox, ty - oy);
            if dx * dx + dy * dy <= radius * radius && los_clear(map, origin, (tx, ty)) {
                vis.insert((tx as u32, ty as u32));
            }
        }
    }
    vis
}

/// Visão por ambiente (3D): ao ENTRAR na casa, o telhado some, as paredes do lado da
/// câmera somem (cutaway dinâmico pelo ângulo) e só o interior + faixa curta na porta fica
/// visível (resto do mundo e players lá fora ocultos, fundo preto). De FORA, players que
/// estão dentro ficam ocultos (não se vê por cima do telhado/paredes).
#[allow(clippy::too_many_arguments)]
fn apply_vision(
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position, Option<&Elevation>)>,
    cameras: Query<&Transform, With<Camera3d>>,
    map: Option<Res<Walkmap>>,
    mut state: ResMut<VisionState>,
    mut clear: ResMut<ClearColor>,
    mut platforms: Query<
        (&Platform, &mut Visibility),
        (Without<HouseWall>, Without<Cell>, Without<Player>, Without<Npc>),
    >,
    mut house_walls: Query<
        (&Transform, &Cell, &DimPair, &mut MeshMaterial3d<StandardMaterial>, &mut Visibility),
        (With<HouseWall>, Without<Platform>),
    >,
    mut scenery: Query<
        (&Cell, &DimPair, &mut MeshMaterial3d<StandardMaterial>),
        (Without<HouseWall>, Without<Platform>, Without<Player>, Without<Npc>),
    >,
    mut actors: Query<
        (&Position, &mut Visibility),
        (Or<(With<Player>, With<Npc>)>, Without<Platform>, Without<HouseWall>, Without<Cell>),
    >,
) {
    // Fundo = névoa escura (além do mapa / onde não há geometria).
    clear.0 = Color::srgb(0.03, 0.04, 0.05);

    let Some(map) = map.as_deref() else {
        return;
    };
    let Some((char_pos, own_elev)) = me
        .0
        .and_then(|id| players.iter().find(|(p, _, _)| p.0 == id))
        .map(|(_, pos, elev)| (pos.0, elev.map_or(0, |e| e.0)))
    else {
        return;
    };
    let cell = (char_pos.x.round() as i32, char_pos.y.round() as i32);
    // "Dentro" = no footprint E ABAIXO do telhado (se está EM CIMA do telhado, não esconde).
    let inside = cell.0 >= 0
        && cell.1 >= 0
        && in_house(cell.0 as u32, cell.1 as u32)
        && own_elev < ROOF_LEVEL;

    // Recalcula o FOV + ESCURECE o cenário fora da visão (troca material) só quando o
    // boneco MUDA de tile. Tudo continua renderizando — o que ele não vê fica escuro.
    if state.last_cell != Some(cell) {
        state.last_cell = Some(cell);
        state.inside = inside;
        state.fov = compute_fov(map, cell, FOV_RADIUS);
        // Em cima do telhado: escurece o INTERIOR da casa (não se vê lá dentro de cima).
        let on_roof = own_elev >= ROOF_LEVEL;
        for (c, dim, mut mat) in &mut scenery {
            let visible =
                state.fov.contains(&(c.x, c.y)) && !(on_roof && in_house(c.x, c.y));
            let want = if visible { &dim.bright } else { &dim.dim };
            if mat.0 != *want {
                mat.0 = want.clone();
            }
        }
    }

    // Plataformas (2º andar, telhado): viram "teto" (somem) quando o boneco está dentro e
    // ABAIXO delas; aparecem no nível delas ou acima. Só se a casa está no FOV.
    let house_center = ((HOUSE_X0 + HOUSE_X1) / 2, (HOUSE_Y0 + HOUSE_Y1) / 2);
    let house_seen = state.fov.contains(&house_center);
    let in_fp = cell.0 >= 0 && cell.1 >= 0 && in_house(cell.0 as u32, cell.1 as u32);
    for (platform, mut vis) in &mut platforms {
        let ceiling = in_fp && own_elev < platform.level;
        *vis = if !ceiling && house_seen { Visibility::Visible } else { Visibility::Hidden };
    }

    // Paredes da casa: claro/escuro por FOV + cutaway (some a da frente quando se entra).
    let player_xz = Vec2::new(char_pos.x, char_pos.y);
    let to_cam = cameras
        .iter()
        .next()
        .map(|t| Vec2::new(t.translation.x, t.translation.z) - player_xz)
        .unwrap_or(Vec2::Y);
    for (tf, c, dim, mut mat, mut vis) in &mut house_walls {
        let cutaway = inside && {
            let w = Vec2::new(tf.translation.x, tf.translation.z) - player_xz;
            w.dot(to_cam) > 0.0
        };
        *vis = if cutaway { Visibility::Hidden } else { Visibility::Visible };
        let want = if state.fov.contains(&(c.x, c.y)) { &dim.bright } else { &dim.dim };
        if mat.0 != *want {
            mat.0 = want.clone();
        }
    }

    // Atores (players/NPC): visíveis só se estão no FOV do boneco.
    for (pos, mut vis) in &mut actors {
        let (ax, ay) = (pos.0.x.round(), pos.0.y.round());
        let show = ax >= 0.0 && ay >= 0.0 && state.fov.contains(&(ax as u32, ay as u32));
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
}
