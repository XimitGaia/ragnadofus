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
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy::text::FontSize;
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
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

/// Altura (em unidades de mundo = tiles) de 1 nível de elevação do terreno.
const LEVEL_HEIGHT: f32 = 1.0;
/// Altura da parede da casa, em tiles — 2 andares (~3 por andar).
const WALL_LEVELS: f32 = 6.0;
/// Altura de 1 andar (onde fica a faixa divisória entre os pisos).
const FLOOR_HEIGHT: f32 = 3.0;
/// Meia-altura do billboard do personagem (chibi, baixinho estilo RO).
const BILLBOARD_HALF: f32 = 0.62;
/// Largura do billboard (proporção do sprite chibi 32x40 = 0,8).
const BILLBOARD_W: f32 = 0.99;

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
fn in_house(x: u32, y: u32) -> bool {
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

/// Frames da animação de caminhada do herói (sprites autorais `hero_0..3.png`).
#[derive(Resource)]
struct HeroFrames(Vec<Handle<Image>>);

/// Estado da animação de andar de um personagem (troca o frame quando se move).
#[derive(Component)]
struct WalkAnim {
    frame: usize,
    timer: f32,
    last: Vec2,
    started: bool,
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
    .init_resource::<VisionState>()
    .init_resource::<HopAnim>()
    .add_observer(on_assigned_player)
    .add_observer(on_zone_received)
    .add_systems(Startup, (load_assets, setup_scene, setup_ui))
    .add_systems(
        Update,
        (
            read_right_click,
            read_jump,
            read_hop,
            hydrate_players,
            hydrate_npcs,
            rotate_input,
            sync_transforms,
            hop_arc,
            face_trees,
            animate_walk,
            auto_rotate_camera,
            camera_orbit,
            apply_vision,
        )
            .chain(),
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

/// Carrega os sprites autorais de caminhada do herói (`hero_0..3.png`).
fn load_assets(asset_server: Res<AssetServer>, mut commands: Commands) {
    let frames = (0..4).map(|i| asset_server.load(format!("hero_{i}.png"))).collect();
    commands.insert_resource(HeroFrames(frames));
}

/// Anima a caminhada: enquanto o personagem se move, cicla os frames do sprite; parado,
/// volta ao frame 0. Detecta movimento pela variação da `Position` replicada.
fn animate_walk(
    time: Res<Time>,
    frames: Res<HeroFrames>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut actors: Query<(&Position, &mut WalkAnim, &MeshMaterial3d<StandardMaterial>)>,
) {
    for (pos, mut anim, material) in &mut actors {
        let moving = anim.started && (pos.0 - anim.last).length() > 0.002;
        anim.last = pos.0;
        anim.started = true;
        let target = if moving {
            anim.timer += time.delta_secs();
            if anim.timer >= 0.14 {
                anim.timer = 0.0;
                anim.frame = (anim.frame + 1) % 4;
            }
            anim.frame
        } else {
            anim.timer = 0.0;
            anim.frame = 0;
            0
        };
        if let Some(mut mat) = materials.get_mut(&material.0) {
            let want = &frames.0[target];
            if mat.base_color_texture.as_ref() != Some(want) {
                mat.base_color_texture = Some(want.clone());
            }
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
                if in_house(x, y) {
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

    commands.insert_resource(map.clone());
    built.0 = true;
    info!("mundo 3D construído: {}x{} (seed {:#x})", zone.tiles_x, zone.tiles_y, zone.seed);
}

/// Dá corpo visual (billboard colorido) a cada player replicado. Cor derivada do `PlayerId`.
fn hydrate_players(
    mut commands: Commands,
    frames: Res<HeroFrames>,
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
            Mesh3d(meshes.add(Rectangle::new(BILLBOARD_W, BILLBOARD_HALF * 2.0))),
            MeshMaterial3d(materials.add(billboard_material(tint, frames.0[0].clone()))),
            Transform::from_xyz(0.0, BILLBOARD_HALF, 0.0),
            Billboard,
            WalkAnim { frame: 0, timer: 0.0, last: Vec2::ZERO, started: false },
        ));
    }
}

/// Dá corpo visual (billboard azul) ao NPC replicado.
fn hydrate_npcs(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    npcs: Query<Entity, (With<Npc>, Without<Billboard>)>,
) {
    let npc_tex = asset_server.load("npc.png");
    for entity in &npcs {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Rectangle::new(BILLBOARD_W, BILLBOARD_HALF * 2.0))),
            MeshMaterial3d(materials.add(billboard_material(Color::WHITE, npc_tex.clone()))),
            Transform::from_xyz(0.0, BILLBOARD_HALF, 0.0),
            Billboard,
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
    mut entities: Query<(&Position, Option<&Elevation>, &mut Transform), With<Billboard>>,
) {
    let face = Quat::from_rotation_y(orbit.yaw);
    for (position, elevation, mut transform) in &mut entities {
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
        transform.translation = Vec3::new(position.0.x, base_y + BILLBOARD_HALF, position.0.y);
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
    if cx >= 0.0 && cy >= 0.0 && in_house(cx as u32, cy as u32) {
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

/// UI de zoom no topo-direito: label "Zoom" + botões "+" e "-". Cada botão ajusta a
/// distância da câmera orbital (`CamOrbit::dist`) via observer nativo de clique.
fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Text::new("Zoom"),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::WHITE),
            ));
            // (+) aproxima (dist menor); (-) afasta (dist maior).
            for (label, factor) in [("+", 0.85_f32), ("-", 1.18_f32)] {
                root.spawn((
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(24.0), ..default() },
                        TextColor(Color::WHITE),
                    ));
                })
                .observe(move |_: On<PointerClick>, mut orbit: ResMut<CamOrbit>| {
                    orbit.zoom = (orbit.zoom * factor).clamp(6.0, 40.0);
                });
        }
        });
}
