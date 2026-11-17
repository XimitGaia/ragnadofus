//! Cliente desktop do ragnadofus — render **3D** (câmera estilo Ragnarok).
//!
//! Conecta a um servidor autoritativo (`replicon` + `renet2`/UDP), envia o input como
//! `MoveTo` (só a célula destino, clique-pra-mover) e **renderiza as entidades replicadas**
//! num mundo 3D: chão/paredes como malhas, personagens como **billboards** (sprites que
//! sempre encaram a câmera). A câmera **orbita** o próprio personagem e gira livre (Q/E),
//! com tilt fixo estilo isométrico-3D — como o RO renderiza o mapa.
//!
//! Servidor via env `RAGNADOFUS_SERVER` (ex.: `127.0.0.1:5000`; default localhost:5000).
//! Para o teste multiplayer: suba o `server` e abra 2+ instâncias deste `client`.

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NativeSocket, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt, RepliconRenetPlugins,
};
use shared::world::{
    is_stair_a, is_stair_b, FLOOR_LEVEL, HOUSE_X0, HOUSE_X1, HOUSE_Y0, HOUSE_Y1, ROOF_LEVEL,
    STAIR_X, STAIR_X2,
};
use shared::{
    build_walkmap, AssignedPlayer, Elevation, Hop, Jump, MoveTo, Npc, Player, PlayerId, Position,
    ProtocolPlugin, Walkmap, ZoneSeed,
};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

mod ui;
mod vision;

/// Ordem determinística dos sistemas de `Update` do cliente — substitui o `.chain()` único
/// de antes. A sequência preserva EXATAMENTE as dependências originais: `Spawn` antes de
/// `Sync` (entidade nova já com `Transform`), `Sync` antes de `Anim` (anim/hop compõem POR
/// CIMA do `Transform`), `Camera` depois de `Anim`, `Vision` depois de `Camera`
/// (`apply_vision` lê o `Transform` da câmera).
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ClientSet {
    Input,
    Spawn,
    Sync,
    Anim,
    Camera,
    Vision,
}

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

/// Altura (em unidades de mundo = tiles) de 1 nível de elevação do terreno.
const LEVEL_HEIGHT: f32 = 1.0;
/// Altura da parede da casa, em tiles — 2 andares (~3 por andar).
const WALL_LEVELS: f32 = 6.0;
/// Altura de 1 andar (onde fica a faixa divisória entre os pisos).
const FLOOR_HEIGHT: f32 = 3.0;
/// Meia-altura do billboard do personagem.
const BILLBOARD_HALF: f32 = 0.7;
/// Largura do billboard (sprite do herói 64×64 quadrado → largura = altura).
const BILLBOARD_W: f32 = 1.4;
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

/// Player controlado por este cliente (recebido do servidor via `AssignedPlayer`).
#[derive(Resource, Default)]
struct MyPlayer(Option<PlayerId>);

/// Evita redesenhar a zona se a `ZoneSeed` chegar mais de uma vez.
#[derive(Resource, Default)]
struct ZoneBuilt(bool);

/// Estado da câmera orbital estilo RO: `yaw` gira livre ao redor do personagem; `pitch` é
/// o tilt (olhar de cima); `zoom` = altura (em tiles) visível na tela — projeção
/// ORTOGRÁFICA (sem perspectiva, achatada como o RO). Menor `zoom` = mais perto.
#[derive(Resource)]
struct CamOrbit {
    yaw: f32,
    pitch: f32,
    zoom: f32,
    /// Sentido TRAVADO da auto-rotação anti-oclusão (0 = parada; ±1 = girando). Trava o
    /// sentido enquanto gira pra não "sambar" trocando de direção toda hora.
    auto_dir: f32,
}

impl Default for CamOrbit {
    fn default() -> Self {
        // pitch ~50° (olhar de cima estilo RO); ~14 tiles visíveis na vertical.
        Self { yaw: 0.6, pitch: 0.88, zoom: 14.0, auto_dir: 0.0 }
    }
}

/// Distância fixa da câmera ao foco (ortográfica: não afeta o tamanho, só o posicionamento
/// e o clipping — só precisa ser grande o bastante pra conter a cena).
const CAM_DIST: f32 = 90.0;

/// Sprite que sempre encara a câmera (personagens/NPCs).
#[derive(Component)]
struct Billboard;

/// Meia-altura do sprite deste billboard (pivô nos pés). Permite sprites de tamanhos
/// diferentes (herói retrato vs NPC quadrado) sem hardcode em `sync_transforms`.
#[derive(Component)]
struct SpriteHalf(f32);

/// Célula (tile) de uma peça de cenário — usada depois pela visão/cutaway (M2).
#[derive(Component, Clone, Copy)]
#[allow(dead_code)]
struct Cell {
    x: u32,
    y: u32,
}

/// Piso suspenso da casa (2º andar ou telhado), no `level` (altura) dado. Some (vira
/// "teto") quando o próprio player está dentro e ABAIXO dele; aparece no nível dele ou acima.
#[derive(Component)]
struct Platform {
    level: i32,
}

/// Parede da CASA (box 3D). Usada pelo cutaway: some a do lado da câmera quando se entra.
#[derive(Component)]
struct HouseWall;

/// Par de materiais (claro/escuro) de uma peça de cenário. O FOV troca entre os dois para
/// ESCURECER o que o boneco não enxerga (em vez de esconder).
#[derive(Component)]
struct DimPair {
    bright: Handle<StandardMaterial>,
    dim: Handle<StandardMaterial>,
}

/// Escurece uma cor (versão "na penumbra" — fora do campo de visão).
fn dim_color(c: Color) -> Color {
    let s = c.to_srgba();
    Color::srgb(s.red * 0.5, s.green * 0.5, s.blue * 0.5)
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

/// Quanto tempo (s) o estado "andando" é mantido após o último movimento recebido.
const MOVE_HOLD: f32 = 0.25;

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

/// Linha do sheet do mago por índice dir8 (0=S,1=SE,2=E,3=NE,4=N,5=NW,6=W,7=SW).
/// Sheet observado: r0=S, r1=SW, r2=W, r3=NW, r4=N, r5=NE, r6=E, r7=SE → row = (8-dir8)%8.
const WIZ_ROW: [usize; 8] = [0, 7, 6, 5, 4, 3, 2, 1];

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

fn main() -> anyhow::Result<()> {
    let server_addr: SocketAddr = std::env::var("RAGNADOFUS_SERVER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            DEFAULT_SERVER.parse().unwrap_or(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000))
        });

    let mut app = App::new();
    // Atualiza o app continuamente mesmo SEM foco da janela — senão o loop throttla, o
    // cliente para de mandar keepalive do netcode e a conexão cai (timeout).
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .insert_resource(ClearColor(Color::srgb(0.45, 0.65, 0.85))) // céu
    .add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            })
            // Sem VSync: o render 3D travava bloqueado no present (swapchain) de uma janela
            // sem compositor ativo (ex.: segundo plano). AutoNoVsync não espera o vblank.
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins((RepliconPlugins, RepliconRenetPlugins))
    .add_plugins(ProtocolPlugin)
    .init_resource::<MyPlayer>()
    .init_resource::<ZoneBuilt>()
    .init_resource::<CamOrbit>()
    .init_resource::<HopAnim>()
    .add_plugins((ui::plugin, vision::plugin))
    .add_observer(on_assigned_player)
    .add_observer(on_zone_received)
    .add_systems(Startup, (load_assets, setup_scene))
    .configure_sets(
        Update,
        (
            ClientSet::Input,
            ClientSet::Spawn,
            ClientSet::Sync,
            ClientSet::Anim,
            ClientSet::Camera,
            ClientSet::Vision,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            read_right_click.in_set(ClientSet::Input),
            read_jump.in_set(ClientSet::Input),
            read_hop.in_set(ClientSet::Input),
            rotate_input.in_set(ClientSet::Input),
            hydrate_players.in_set(ClientSet::Spawn),
            hydrate_npcs.in_set(ClientSet::Spawn),
            sync_transforms.in_set(ClientSet::Sync),
            hop_arc.in_set(ClientSet::Anim),
            face_trees.in_set(ClientSet::Anim),
            animate_directional.in_set(ClientSet::Anim),
            auto_rotate_camera.in_set(ClientSet::Camera),
            camera_orbit.in_set(ClientSet::Camera),
        ),
    );

    // Conecta só DEPOIS do warmup do render (veja `connect_when_ready`): o bevy_pbr
    // compila pipelines nos primeiros frames e isso trava o loop por alguns segundos —
    // se conectar nesse meio, a troca de handshake do netcode estoura o timeout. Esperar
    // o loop ficar fluido faz o handshake completar na hora.
    app.insert_resource(PendingConnect { addr: server_addr, frames: 0, attempted: false });
    app.add_systems(Update, connect_when_ready);
    info!("cliente ragnadofus (3D) — conectará a {server_addr} após warmup do render");
    app.run();
    Ok(())
}

/// Servidor-alvo + contador de warmup até disparar a conexão.
#[derive(Resource)]
struct PendingConnect {
    addr: SocketAddr,
    frames: u32,
    attempted: bool,
}

/// Espera o render aquecer (~1,5s de frames fluidos) e então cria o `RenetClient` +
/// transporte netcode. Evita que o stall de compilação de shaders mate o handshake.
fn connect_when_ready(
    mut pending: ResMut<PendingConnect>,
    channels: Res<RepliconChannels>,
    existing: Option<Res<RenetClient>>,
    mut commands: Commands,
) {
    if pending.attempted || existing.is_some() {
        return;
    }
    pending.frames += 1;
    if pending.frames < 90 {
        return;
    }
    pending.attempted = true;
    match make_netcode(&channels, pending.addr) {
        Ok((client, transport)) => {
            commands.insert_resource(client);
            commands.insert_resource(transport);
            info!("conectando ao servidor {} (pós-warmup)", pending.addr);
        }
        Err(e) => error!("falha ao criar netcode: {e:?}"),
    }
}

/// Monta o `RenetClient` + transporte netcode (UDP/renet2) para `server_addr`.
fn make_netcode(
    channels: &RepliconChannels,
    server_addr: SocketAddr,
) -> anyhow::Result<(RenetClient, NetcodeClientTransport)> {
    let client = RenetClient::new(
        ConnectionConfig::from_channels(channels.server_configs(), channels.client_configs()),
        false,
    );
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: shared::protocol::PROTOCOL_ID,
        socket_id: 0,
        server_addr,
        user_data: None,
    };
    let native = NativeSocket::new(socket).map_err(|e| anyhow::anyhow!("socket: {e:?}"))?;
    let transport = NetcodeClientTransport::new(current_time, authentication, native)?;
    Ok((client, transport))
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

/// Câmera 3D (perspectiva) + uma luz direcional. A posição da câmera é ajustada a cada
/// frame por `camera_orbit` (segue e orbita o próprio personagem).
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // ORTOGRÁFICA: sem perspectiva (chão não "foge" pro horizonte) — o look achatado
        // do RO. Tamanho controlado por `viewport_height` (atualizado por `camera_orbit`).
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: 14.0 },
            ..OrthographicProjection::default_3d()
        }),
        // sem LUTs de tonemapping no build curado (DT-03): usa None (e sem dither, que
        // depende do tonemapping — evita o warning).
        Tonemapping::None,
        DebandDither::Disabled,
        Transform::from_xyz(32.0, 20.0, 50.0).looking_at(Vec3::new(32.0, 0.0, 32.0), Vec3::Y),
        AmbientLight { brightness: 600.0, ..default() },
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, ..default() },
        Transform::from_xyz(60.0, 100.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Recebe a receita da zona e constrói o mundo 3D: 1 quad de chão por tile (xadrez
/// grama/terra, elevado pela altura da célula) e uma CAIXA de parede onde não é caminhável.
fn on_zone_received(
    trigger: On<ZoneSeed>,
    mut built: ResMut<ZoneBuilt>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    if built.0 {
        return;
    }
    let zone: ZoneSeed = *trigger.event();
    let map = build_walkmap(&zone);

    let tile_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    let wall_mesh = meshes.add(Cuboid::new(1.0, WALL_LEVELS, 1.0));
    // Material opaco (claro) + helper para gerar o par claro/escuro de uma cor.
    let lit = |c: Color| StandardMaterial { base_color: c, perceptual_roughness: 1.0, ..default() };
    let mut pair = |c: Color| DimPair {
        bright: materials.add(lit(c)),
        dim: materials.add(lit(dim_color(c))),
    };
    let grass = pair(Color::srgb(0.30, 0.55, 0.25));
    let dirt = pair(Color::srgb(0.45, 0.35, 0.22));
    let wall = pair(Color::srgb(0.52, 0.52, 0.58));
    let band = pair(Color::srgb(0.40, 0.26, 0.16));
    let stair = pair(Color::srgb(0.50, 0.38, 0.24));
    // Árvores (billboard): claro = textura cheia; escuro = textura com tinta escura.
    let tree_h = BILLBOARD_HALF * 2.0 * 1.4;
    let tree_mesh = meshes.add(Rectangle::new(BILLBOARD_W * 1.4, tree_h));
    let tree_tex: Handle<Image> = asset_server.load("tree.png");
    let tree = DimPair {
        bright: materials.add(billboard_material(Color::WHITE, tree_tex.clone())),
        dim: materials.add(billboard_material(Color::srgb(0.35, 0.35, 0.35), tree_tex)),
    };
    // Telhado da casa (material; desenhado por célula mais abaixo, com vão pra escada B).
    let roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.28, 0.24),
        perceptual_roughness: 1.0,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    // Faixa (moldura) divisória entre o 1º e o 2º andar da casa.
    let band_mesh = meshes.add(Cuboid::new(1.12, 0.45, 1.12));

    for y in 0..zone.tiles_y {
        for x in 0..zone.tiles_x {
            let cell = map.get(x, y);
            let walkable = cell.map(|c| c.walkable).unwrap_or(false);
            let h = cell.map(|c| c.height).unwrap_or(0);
            let base_y = h as f32 * LEVEL_HEIGHT;

            let gd = if (x + y) % 2 == 0 { &grass } else { &dirt };
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(gd.bright.clone()),
                Transform::from_xyz(x as f32, base_y, y as f32),
                Cell { x, y },
                DimPair { bright: gd.bright.clone(), dim: gd.dim.clone() },
            ));

            if !walkable {
                // parede: caixa de altura WALL_LEVELS apoiada no chão da célula.
                let mut wall_e = commands.spawn((
                    Mesh3d(wall_mesh.clone()),
                    MeshMaterial3d(wall.bright.clone()),
                    Transform::from_xyz(x as f32, base_y + WALL_LEVELS / 2.0, y as f32),
                    Cell { x, y },
                    DimPair { bright: wall.bright.clone(), dim: wall.dim.clone() },
                ));
                if vision::in_house(x, y) {
                    wall_e.insert(HouseWall); // parede da casa -> cutaway ao entrar
                    // faixa divisória entre os 2 andares, nesta coluna de parede.
                    commands.spawn((
                        Mesh3d(band_mesh.clone()),
                        MeshMaterial3d(band.bright.clone()),
                        Transform::from_xyz(x as f32, base_y + FLOOR_HEIGHT, y as f32),
                        Cell { x, y },
                        HouseWall,
                        DimPair { bright: band.bright.clone(), dim: band.dim.clone() },
                    ));
                }
            } else if h >= 2 && x % 3 == 0 && y % 3 == 0 {
                // morro (platô): salpica árvores para dar cara de colina arborizada.
                commands.spawn((
                    Mesh3d(tree_mesh.clone()),
                    MeshMaterial3d(tree.bright.clone()),
                    Transform::from_xyz(x as f32, base_y + tree_h / 2.0, y as f32),
                    Billboard,
                    Cell { x, y },
                    DimPair { bright: tree.bright.clone(), dim: tree.dim.clone() },
                ));
            }
        }
    }

    // Telhado por CÉLULA sobre o footprint, com VÃO onde a escada B emerge (pra ver e
    // descer a escada quando se está no telhado).
    let roof_tile = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    for y in HOUSE_Y0..=HOUSE_Y1 {
        for x in HOUSE_X0..=HOUSE_X1 {
            if is_stair_b(x, y) {
                continue; // vão da escada B no telhado
            }
            commands.spawn((
                Mesh3d(roof_tile.clone()),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_xyz(x as f32, ROOF_LEVEL as f32 * LEVEL_HEIGHT + 0.05, y as f32),
                Platform { level: ROOF_LEVEL },
            ));
        }
    }
    // Piso do 2º andar (plataforma), POR CÉLULA sobre o interior — deixando um VÃO
    // (stairwell) nas colunas das escadas, pra ver/usar a escada ao descer/subir.
    let floor2_tile = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    for y in (HOUSE_Y0 + 1)..=(HOUSE_Y1 - 1) {
        for x in (HOUSE_X0 + 1)..=(HOUSE_X1 - 1) {
            if is_stair_a(x, y) || is_stair_b(x, y) {
                continue; // vão da escada (stairwell) — piso aberto aqui
            }
            commands.spawn((
                Mesh3d(floor2_tile.clone()),
                MeshMaterial3d(stair.bright.clone()),
                Transform::from_xyz(x as f32, FLOOR_LEVEL as f32 * LEVEL_HEIGHT + 0.02, y as f32),
                Platform { level: FLOOR_LEVEL },
            ));
        }
    }

    // Escada A (térreo→2º andar), coluna STAIR_X: degraus 1..3 subindo do chão.
    for i in 0..FLOOR_LEVEL as u32 {
        let top = (i + 1) as f32 * LEVEL_HEIGHT;
        let z = HOUSE_Y1 as f32 - 2.0 - i as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, top, 0.95))),
            MeshMaterial3d(stair.bright.clone()),
            Transform::from_xyz(STAIR_X as f32, top / 2.0, z),
            Cell { x: STAIR_X, y: z.round() as u32 },
            DimPair { bright: stair.bright.clone(), dim: stair.dim.clone() },
        ));
    }
    // Escada B (2º andar→telhado), coluna STAIR_X2: degraus 4..6 saindo do piso do 2º andar.
    // É marcada como Platform(2º andar): some quando o boneco está no TÉRREO (abaixo dela),
    // pra não aparecer de baixo; reaparece quando se sobe ao 2º andar.
    for i in 0..FLOOR_LEVEL as u32 {
        let bottom = FLOOR_LEVEL as f32 * LEVEL_HEIGHT;
        let top = (FLOOR_LEVEL + 1 + i as i32) as f32 * LEVEL_HEIGHT;
        let z = HOUSE_Y0 as f32 + 4.0 + i as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, top - bottom, 0.95))),
            MeshMaterial3d(stair.bright.clone()),
            Transform::from_xyz(STAIR_X2 as f32, (bottom + top) / 2.0, z),
            Platform { level: FLOOR_LEVEL },
        ));
    }

    // Props de cenário (billboards) espalhados perto da casa — "montar coisas" no mundo.
    // (tex, x, y, largura, altura). Arte original (crate/barril/arbusto/pedra/placa/flor).
    let props: [(&str, u32, u32, f32, f32); 10] = [
        ("crate.png", 22, 31, 0.9, 0.9),
        ("crate.png", 23, 31, 0.9, 0.9),
        ("crate.png", 22, 30, 0.9, 0.9),
        ("barrel.png", 24, 30, 0.85, 0.95),
        ("sign.png", 26, 31, 0.9, 0.9),
        ("bush.png", 30, 33, 1.0, 0.88),
        ("bush.png", 18, 33, 1.0, 0.88),
        ("rock.png", 31, 29, 1.0, 0.75),
        ("flower.png", 19, 30, 0.9, 0.68),
        ("mushroom.png", 29, 34, 0.55, 0.55),
    ];
    for (file, px, py, pw, ph) in props {
        let tex: Handle<Image> = asset_server.load(file);
        let bright = materials.add(billboard_material(Color::WHITE, tex.clone()));
        let dim = materials.add(billboard_material(Color::srgb(0.5, 0.5, 0.5), tex));
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(pw, ph))),
            MeshMaterial3d(bright.clone()),
            Transform::from_xyz(px as f32, ph / 2.0, py as f32),
            Billboard,
            Cell { x: px, y: py },
            DimPair { bright, dim },
        ));
    }

    commands.insert_resource(map.clone());
    built.0 = true;
    info!("mundo 3D construído: {}x{} (seed {:#x})", zone.tiles_x, zone.tiles_y, zone.seed);
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
fn billboard_material(tint: Color, texture: Handle<Image>) -> StandardMaterial {
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

/// Guarda qual player é o deste cliente (recebido do servidor).
fn on_assigned_player(trigger: On<AssignedPlayer>, mut me: ResMut<MyPlayer>) {
    me.0 = Some(trigger.event().0);
    info!("meu player: {:?}", trigger.event().0);
}

/// Gira a câmera livre ao redor do personagem (yaw): teclas Q/E **ou** scroll do mouse.
fn rotate_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut wheel: MessageReader<MouseWheel>,
    mut orbit: ResMut<CamOrbit>,
) {
    let speed = 1.6;
    if keys.pressed(KeyCode::KeyQ) {
        orbit.yaw -= speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyE) {
        orbit.yaw += speed * time.delta_secs();
    }
    // Scroll do mouse gira a câmera em torno do personagem (~10° por entalhe).
    for ev in wheel.read() {
        orbit.yaw += ev.y * 0.18;
    }
}

/// A câmera ORBITA o próprio personagem: foco no player, posição a `dist` com tilt `pitch`
/// e giro `yaw`. Resolve "seguir o player" + "girar ao redor" de uma vez.
/// Centro do footprint da casa no plano do chão (x, z).
fn house_center_xz() -> Vec2 {
    Vec2::new((HOUSE_X0 + HOUSE_X1) as f32 / 2.0, (HOUSE_Y0 + HOUSE_Y1) as f32 / 2.0)
}

/// "Cone de sombra" da casa visto do boneco: retorna `(base, lo, hi)`, onde `base` é o
/// ângulo (convenção da órbita: `atan2(dx, dz)`) do centro da casa a partir de `char`, e
/// `[lo, hi]` (relativos a `base`) é o arco angular que a casa ocupa. A câmera está ocluída
/// quando seu ângulo (relativo a `base`) cai dentro de `[lo, hi]`. Como o boneco está FORA
/// da casa (convexa), o arco é < π.
fn house_cone(char: Vec2) -> (f32, f32, f32) {
    let center = house_center_xz();
    let base = (center.x - char.x).atan2(center.y - char.y);
    let corners = [
        Vec2::new(HOUSE_X0 as f32 - 0.5, HOUSE_Y0 as f32 - 0.5),
        Vec2::new(HOUSE_X1 as f32 + 0.5, HOUSE_Y0 as f32 - 0.5),
        Vec2::new(HOUSE_X0 as f32 - 0.5, HOUSE_Y1 as f32 + 0.5),
        Vec2::new(HOUSE_X1 as f32 + 0.5, HOUSE_Y1 as f32 + 0.5),
    ];
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for c in corners {
        let d = c - char;
        let a = wrap_angle(d.x.atan2(d.y) - base);
        lo = lo.min(a);
        hi = hi.max(a);
    }
    (base, lo, hi)
}

/// Normaliza um ângulo para `(-π, π]`.
fn wrap_angle(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (a + PI).rem_euclid(TAU) - PI
}

/// Auto-rotação anti-oclusão: se a casa fica ENTRE a câmera e o próprio boneco (e ele está
/// FORA da casa), gira a câmera sozinha — pelo caminho mais curto de se afastar da direção
/// da casa — até a linha de visão ficar livre e o boneco reaparecer.
fn auto_rotate_camera(
    time: Res<Time>,
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut orbit: ResMut<CamOrbit>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, pos)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let char_xz = pos.0;
    // Dentro da casa: a visão interna já cuida — não auto-rotaciona.
    let (cx, cy) = (char_xz.x.round(), char_xz.y.round());
    if cx >= 0.0 && cy >= 0.0 && vision::in_house(cx as u32, cy as u32) {
        orbit.auto_dir = 0.0;
        return;
    }
    // Cone de sombra da casa visto do boneco; `ya` = ângulo da câmera relativo ao centro.
    const M_OFF: f32 = 0.18; // folga angular de saída (~10°)
    let (base, lo, hi) = house_cone(char_xz);
    let ya = wrap_angle(orbit.yaw - base);

    if orbit.auto_dir == 0.0 {
        // Começa só se a câmera estiver DENTRO do cone (boneco tampado pela casa).
        if ya <= lo || ya >= hi {
            return;
        }
        // Escolhe a borda mais PRÓXIMA (caminho mais curto pra sair) — trava o lado.
        orbit.auto_dir = if ya - lo <= hi - ya { -1.0 } else { 1.0 };
    } else if ya <= lo - M_OFF || ya >= hi + M_OFF {
        orbit.auto_dir = 0.0; // já saiu do cone com folga -> para
        return;
    }

    // Alvo = borda escolhida + folga; move a yaw suave até lá (nunca mais que o necessário,
    // então não dá a "volta inteira"). Acompanha a borda enquanto o boneco anda.
    let target_rel = if orbit.auto_dir < 0.0 { lo - M_OFF } else { hi + M_OFF };
    let diff = wrap_angle(base + target_rel - orbit.yaw);
    let step = (2.6 * time.delta_secs()).min(diff.abs());
    orbit.yaw += step * diff.signum();
}

fn camera_orbit(
    orbit: Res<CamOrbit>,
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, position)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let focus = Vec3::new(position.0.x, 0.8, position.0.y);
    let dir = Quat::from_rotation_y(orbit.yaw)
        * Vec3::new(0.0, orbit.pitch.sin(), orbit.pitch.cos());
    for (mut transform, mut projection) in &mut cameras {
        transform.translation = focus + dir * CAM_DIST;
        transform.look_at(focus, Vec3::Y);
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scaling_mode = ScalingMode::FixedVertical { viewport_height: orbit.zoom };
        }
    }
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

