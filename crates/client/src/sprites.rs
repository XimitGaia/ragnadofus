//! Billboards de personagens + animação direcional data-driven ("desenho" + "andar")
//! (feature-plugin).

use bevy::prelude::*;
use shared::{Elevation, Npc, Player, Position, Walkmap};

use crate::camera::CamOrbit;
use crate::common::{Billboard, SpriteHalf, BILLBOARD_HALF, LEVEL_HEIGHT};
use crate::app::ClientSet;

/// Lado do sprite do HERÓI no mundo. Os sprites do cavaleiro são 128×128 quadrados e
/// foot-anchored → quadrado (sem distorção) e troca de direção/frame sem salto.
const HERO_H: f32 = 2.2;
const HERO_W: f32 = 2.2;
/// Meia-altura do herói (pivô nos pés = base + HERO_HALF).
const HERO_HALF: f32 = HERO_H / 2.0;
/// Billboard do NPC mago (células 64×96 do sheet → retrato, sem distorção).
const WIZ_H: f32 = 2.1;
const WIZ_W: f32 = WIZ_H * (64.0 / 96.0);
/// Meia-altura + um leve "lift" pra erguer o mago do chão (pedido: subir um pouco).
/// Células originais já têm uma folga de pé, então o lift aqui é pequeno.
const WIZ_LIFT: f32 = 0.1;
const WIZ_HALF: f32 = WIZ_H / 2.0 + WIZ_LIFT;

/// Quanto tempo (s) o estado "andando" é mantido após o último movimento recebido.
const MOVE_HOLD: f32 = 0.25;

/// Linha do sheet do mago por índice dir8 (0=S,1=SE,2=E,3=NE,4=N,5=NW,6=W,7=SW).
/// Sheet observado: r0=S, r1=SW, r2=W, r3=NW, r4=N, r5=NE, r6=E, r7=SE → row = (8-dir8)%8.
const WIZ_ROW: [usize; 8] = [0, 7, 6, 5, 4, 3, 2, 1];

/// Registra o carregamento de assets e os sistemas de spawn/sync/anim nos sets certos.
pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_assets).add_systems(
        Update,
        (
            (hydrate_players, hydrate_npcs).in_set(ClientSet::Spawn),
            sync_transforms.in_set(ClientSet::Sync),
            (face_trees, animate_directional).in_set(ClientSet::Anim),
        ),
    );
}

/// Sprites direcionais do cavaleiro: 5 direções-base geradas (S,SE,E,NE,N); W/SW/NW são
/// espelho (flip X) dessas. Cada direção: 1 pose idle + 4 frames de walk. (128×128)
#[derive(Resource)]
struct KnightSprites {
    idle: [Handle<Image>; 5],
    walk: [[Handle<Image>; 4]; 5],
}

/// Estado da animação direcional do cavaleiro. Mantém a última direção quando parado
/// (não "vira pro sul" ao parar) e cicla os frames de walk ao mover.
#[derive(Component)]
struct DirAnim {
    last: Vec2,
    started: bool,
    /// 0=S 1=SE 2=E 3=NE 4=N 5=NW 6=W 7=SW (mantido ao parar).
    dir8: usize,
    frame: usize,
    timer: f32,
    /// Histerese de "andando" (s). A `Position` chega pela rede (~10-15Hz); sem isso o
    /// sprite voltaria pro frame 0 nos frames (60Hz) entre updates. Segura o walk ativo.
    move_cd: f32,
}

/// Mapeia índice 8-direções → (direção-base 0..4 gerada, espelhar em X?).
fn dir8_to_base(dir8: usize) -> (usize, bool) {
    match dir8 {
        0 => (0, false), // S
        1 => (1, false), // SE
        2 => (2, false), // E
        3 => (3, false), // NE
        4 => (4, false), // N
        5 => (3, true),  // NW = espelho de NE
        6 => (2, true),  // W  = espelho de E
        _ => (1, true),  // SW = espelho de SE
    }
}

/// Direção 8-vias da velocidade `v` (mundo XZ) RELATIVA à câmera (`yaw`): "sul" = indo em
/// direção à câmera (aparece de frente). Base de tela: baixo=(sin,cos), direita=(cos,-sin).
fn dir8_from_velocity(v: Vec2, yaw: f32) -> usize {
    let (s, c) = (yaw.sin(), yaw.cos());
    let south = v.x * s + v.y * c;
    let east = v.x * c - v.y * s;
    (east.atan2(south) / std::f32::consts::FRAC_PI_4).round() as i32 as usize & 7
}

/// Sprites do NPC mago: sheet 8 direções (linhas) × 8 frames (colunas), 64×96 nativo.
#[derive(Resource)]
struct WizardSprites {
    frames: [[Handle<Image>; 8]; 8], // [linha=direção][coluna=frame]
}

/// Frames de UMA direção: pose idle + ciclo de walk + se deve espelhar em X (flip).
struct DirFrames {
    idle: Handle<Image>,
    walk: Vec<Handle<Image>>,
    mirror: bool,
}

/// Tabela de sprites direcionais **já resolvida** (8 direções), montada na hidratação a
/// partir do kit do personagem. É o DADO que torna `animate_directional` ÚNICO: a diferença
/// entre personagens (cavaleiro 5-base+espelho vs mago 8 nativas) vive só aqui, não em código.
#[derive(Component)]
struct DirectionalSprite {
    dirs: [DirFrames; 8], // índice = dir8 (0=S .. 7=SW)
    frame_time: f32,      // segundos por frame de walk
    breathe: bool,        // respiração (idle) quando parado
}

impl DirectionalSprite {
    /// Cavaleiro: 5 direções-base geradas (S,SE,E,NE,N) + W/SW/NW como espelho.
    fn knight(k: &KnightSprites) -> Self {
        let dirs = core::array::from_fn(|d8| {
            let (base, mirror) = dir8_to_base(d8);
            DirFrames { idle: k.idle[base].clone(), walk: k.walk[base].to_vec(), mirror }
        });
        Self { dirs, frame_time: 0.14, breathe: true }
    }

    /// Mago: 8 direções NATIVAS do sheet (linha por `WIZ_ROW`), sem espelho.
    fn wizard(w: &WizardSprites) -> Self {
        let dirs = core::array::from_fn(|d8| {
            let row = WIZ_ROW[d8];
            DirFrames { idle: w.frames[row][0].clone(), walk: w.frames[row].to_vec(), mirror: false }
        });
        Self { dirs, frame_time: 0.12, breathe: true }
    }
}

/// Carrega os sprites direcionais do cavaleiro (`knight/{s,se,e,ne,n}{,_walk0..3}.png`).
fn load_assets(asset_server: Res<AssetServer>, mut commands: Commands) {
    let names = ["s", "se", "e", "ne", "n"];
    let idle = names.map(|d| asset_server.load(format!("knight/{d}.png")));
    let walk =
        names.map(|d| [0, 1, 2, 3].map(|i| asset_server.load(format!("knight/{d}_walk{i}.png"))));
    commands.insert_resource(KnightSprites { idle, walk });

    // NPC mago: sheet 8×8 fatiado (wizard/w_r{linha}_c{coluna}.png).
    let frames: [[Handle<Image>; 8]; 8] = core::array::from_fn(|r| {
        core::array::from_fn(|c| asset_server.load(format!("wizard/w_r{r}_c{c}.png")))
    });
    commands.insert_resource(WizardSprites { frames });
}

/// Dá corpo visual (billboard colorido) a cada player replicado. Cor derivada do `PlayerId`.
fn hydrate_players(
    mut commands: Commands,
    sprites: Res<KnightSprites>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    players: Query<(Entity, &Player), Without<Billboard>>,
) {
    for (entity, player) in &players {
        let id = player.0 .0;
        let r = ((id >> 16) & 0xFF) as f32 / 255.0;
        let g = ((id >> 8) & 0xFF) as f32 / 255.0;
        let b = (id & 0xFF) as f32 / 255.0;
        // tinta suave (clara) para distinguir players sem lavar o sprite.
        let tint = Color::srgb(0.7 + 0.3 * r, 0.7 + 0.3 * g, 0.7 + 0.3 * b);
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Rectangle::new(HERO_W, HERO_H))),
            MeshMaterial3d(materials.add(billboard_material(tint, sprites.idle[0].clone()))),
            Transform::from_xyz(0.0, HERO_HALF, 0.0),
            Billboard,
            SpriteHalf(HERO_HALF),
            DirAnim { last: Vec2::ZERO, started: false, dir8: 0, frame: 0, timer: 0.0, move_cd: 0.0 },
            DirectionalSprite::knight(&sprites),
        ));
    }
}

/// Dá corpo visual (billboard azul) ao NPC replicado.
fn hydrate_npcs(
    mut commands: Commands,
    sprites: Res<WizardSprites>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    npcs: Query<Entity, (With<Npc>, Without<Billboard>)>,
) {
    for entity in &npcs {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Rectangle::new(WIZ_W, WIZ_H))),
            MeshMaterial3d(materials.add(billboard_material(Color::WHITE, sprites.frames[0][0].clone()))),
            Transform::from_xyz(0.0, WIZ_HALF, 0.0),
            Billboard,
            SpriteHalf(WIZ_HALF),
            DirAnim { last: Vec2::ZERO, started: false, dir8: 0, frame: 0, timer: 0.0, move_cd: 0.0 },
            DirectionalSprite::wizard(&sprites),
        ));
    }
}

/// Material de billboard: sprite com transparência (máscara), cor chapada (unlit), visível
/// dos dois lados. `tint` multiplica a textura (branco = textura original).
pub(crate) fn billboard_material(tint: Color, texture: Handle<Image>) -> StandardMaterial {
    StandardMaterial {
        base_color: tint,
        base_color_texture: Some(texture),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        cull_mode: None,
        double_sided: true,
        ..default()
    }
}

/// Projeta a `Position` replicada (tile x,y) para o mundo 3D e faz o billboard encarar a
/// câmera (gira só no eixo vertical — fica sempre em pé). Ergue pela altura do terreno.
fn sync_transforms(
    orbit: Res<CamOrbit>,
    map: Option<Res<Walkmap>>,
    mut entities: Query<
        (&Position, Option<&Elevation>, Option<&SpriteHalf>, &mut Transform),
        With<Billboard>,
    >,
) {
    let face = Quat::from_rotation_y(orbit.yaw);
    for (position, elevation, sprite_half, mut transform) in &mut entities {
        let half = sprite_half.map(|h| h.0).unwrap_or(BILLBOARD_HALF);
        // Multi-nível: a altura vem da `Elevation` replicada (sobe escada / cai). Entidades
        // sem elevação (NPC) usam a altura do térreo na walkmap.
        let base_y = match elevation {
            Some(e) => e.0 as f32 * LEVEL_HEIGHT,
            None => match map.as_deref() {
                Some(m) => m
                    .cell_at_world(position.0)
                    .and_then(|(x, y)| m.get(x, y))
                    .map(|c| c.height as f32 * LEVEL_HEIGHT)
                    .unwrap_or(0.0),
                None => 0.0,
            },
        };
        transform.translation = Vec3::new(position.0.x, base_y + half, position.0.y);
        transform.rotation = face;
    }
}

/// Billboards ESTÁTICOS (árvores) encaram a câmera (giram só no yaw); posição é fixa.
fn face_trees(
    orbit: Res<CamOrbit>,
    mut trees: Query<&mut Transform, (With<Billboard>, Without<Position>)>,
) {
    let face = Quat::from_rotation_y(orbit.yaw);
    for mut transform in &mut trees {
        transform.rotation = face;
    }
}

/// Animação direcional ÚNICA para qualquer personagem com `DirectionalSprite` (cavaleiro,
/// mago, futuros). Em 8 direções relativas à câmera:
/// - escolhe a direção pela velocidade (variação da `Position`) projetada no yaw da câmera,
///   com histerese (`MOVE_HOLD`) pra o walk ciclar suave entre updates de rede;
/// - andando: cicla os frames de walk da direção; parado: pose idle (+ respiração, se `breathe`);
/// - espelho em X conforme o `DirFrames.mirror` da direção.
///
/// É o "um lugar só" pra ajustar o andar — a diferença entre personagens é DADO
/// (`DirectionalSprite`), não código. Roda DEPOIS de `sync_transforms`/`hop_arc`.
fn animate_directional(
    time: Res<Time>,
    orbit: Res<CamOrbit>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut actors: Query<(
        Entity,
        &Position,
        &mut DirAnim,
        &DirectionalSprite,
        &MeshMaterial3d<StandardMaterial>,
        Option<&SpriteHalf>,
        &mut Transform,
    )>,
) {
    let t = time.elapsed_secs();
    for (e, pos, mut anim, sprite, material, sprite_half, mut transform) in &mut actors {
        // --- detecção de movimento com histerese (único lugar) ---
        let v = pos.0 - anim.last;
        anim.last = pos.0;
        if anim.started && v.length() > 0.002 {
            anim.dir8 = dir8_from_velocity(v, orbit.yaw);
            anim.move_cd = MOVE_HOLD; // segura "andando" entre updates de rede
        }
        anim.started = true;
        if anim.move_cd > 0.0 {
            anim.move_cd -= time.delta_secs();
        }
        let moving = anim.move_cd > 0.0;

        let df = &sprite.dirs[anim.dir8];

        // --- textura: walk (ciclo) ou idle ---
        let want = if moving && !df.walk.is_empty() {
            anim.timer += time.delta_secs();
            if anim.timer >= sprite.frame_time {
                anim.timer = 0.0;
                anim.frame = (anim.frame + 1) % df.walk.len();
            }
            df.walk.get(anim.frame).unwrap_or(&df.idle)
        } else {
            anim.timer = 0.0;
            anim.frame = 0;
            &df.idle
        };
        if let Some(mut mat) = materials.get_mut(&material.0) {
            if mat.base_color_texture.as_ref() != Some(want) {
                mat.base_color_texture = Some(want.clone());
            }
        }

        // --- pose: espelho (sempre) + respiração/bob (só parado, se breathe) ---
        let mirror_sign = if df.mirror { -1.0 } else { 1.0 };
        if moving || !sprite.breathe {
            transform.scale = Vec3::new(mirror_sign, 1.0, 1.0);
        } else {
            let half = sprite_half.map(|h| h.0).unwrap_or(BILLBOARD_HALF);
            let phase = (e.to_bits() % 17) as f32 * 0.37;
            let breath = (t * 2.2 + phase).sin();
            let sy = 1.0 + 0.025 * breath;
            let sx = 1.0 - 0.015 * breath;
            transform.scale = Vec3::new(mirror_sign * sx, sy, 1.0);
            transform.translation.y += half * (sy - 1.0);
            transform.translation.y += 0.015 * (t * 1.3 + phase).sin();
        }
    }
}
