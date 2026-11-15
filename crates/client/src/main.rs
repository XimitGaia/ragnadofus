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
use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NativeSocket, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt, RepliconRenetPlugins,
};
use shared::{
    build_walkmap, AssignedPlayer, MoveTo, Npc, Player, PlayerId, Position, ProtocolPlugin, Walkmap,
    ZoneSeed,
};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

/// Altura (em unidades de mundo = tiles) de 1 nível de elevação do terreno.
const LEVEL_HEIGHT: f32 = 1.0;
/// Altura da parede da casa, em tiles.
const WALL_LEVELS: f32 = 3.0;
/// Meia-altura do billboard do personagem (para apoiar os "pés" no chão).
const BILLBOARD_HALF: f32 = 0.7;
/// Largura do billboard (proporção do sprite 32x48 ~= 0,667).
const BILLBOARD_W: f32 = 0.95;

/// Player controlado por este cliente (recebido do servidor via `AssignedPlayer`).
#[derive(Resource, Default)]
struct MyPlayer(Option<PlayerId>);

/// Evita redesenhar a zona se a `ZoneSeed` chegar mais de uma vez.
#[derive(Resource, Default)]
struct ZoneBuilt(bool);

/// Estado da câmera orbital estilo RO: `yaw` gira livre ao redor do personagem; `pitch` é
/// o tilt (fixo, olhando de cima); `dist` é o zoom (distância ao personagem).
#[derive(Resource)]
struct CamOrbit {
    yaw: f32,
    pitch: f32,
    dist: f32,
}

impl Default for CamOrbit {
    fn default() -> Self {
        Self { yaw: 0.6, pitch: 0.95, dist: 26.0 } // pitch ~54° (iso-3D)
    }
}

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
    .add_observer(on_assigned_player)
    .add_observer(on_zone_received)
    .add_systems(Startup, (setup_scene, setup_ui))
    .add_systems(
        Update,
        (
            read_right_click,
            hydrate_players,
            hydrate_npcs,
            rotate_input,
            sync_transforms,
            face_trees,
            camera_orbit,
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

/// Câmera 3D (perspectiva) + uma luz direcional. A posição da câmera é ajustada a cada
/// frame por `camera_orbit` (segue e orbita o próprio personagem).
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
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
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.55, 0.25),
        perceptual_roughness: 1.0,
        ..default()
    });
    let dirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.35, 0.22),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.52, 0.58),
        perceptual_roughness: 1.0,
        ..default()
    });
    // Árvores (billboard) para vestir o morro (platô).
    let tree_h = BILLBOARD_HALF * 2.0 * 1.4;
    let tree_mesh = meshes.add(Rectangle::new(BILLBOARD_W * 1.4, tree_h));
    let tree_mat = materials.add(billboard_material(Color::WHITE, asset_server.load("tree.png")));

    for y in 0..zone.tiles_y {
        for x in 0..zone.tiles_x {
            let cell = map.get(x, y);
            let walkable = cell.map(|c| c.walkable).unwrap_or(false);
            let h = cell.map(|c| c.height).unwrap_or(0);
            let base_y = h as f32 * LEVEL_HEIGHT;

            let mat = if (x + y) % 2 == 0 { grass.clone() } else { dirt.clone() };
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(mat),
                Transform::from_xyz(x as f32, base_y, y as f32),
                Cell { x, y },
            ));

            if !walkable {
                // parede: caixa de altura WALL_LEVELS apoiada no chão da célula.
                commands.spawn((
                    Mesh3d(wall_mesh.clone()),
                    MeshMaterial3d(wall_mat.clone()),
                    Transform::from_xyz(x as f32, base_y + WALL_LEVELS / 2.0, y as f32),
                    Cell { x, y },
                ));
            } else if h >= 2 && x % 3 == 0 && y % 3 == 0 {
                // morro (platô): salpica árvores para dar cara de colina arborizada.
                commands.spawn((
                    Mesh3d(tree_mesh.clone()),
                    MeshMaterial3d(tree_mat.clone()),
                    Transform::from_xyz(x as f32, base_y + tree_h / 2.0, y as f32),
                    Billboard,
                ));
            }
        }
    }

    commands.insert_resource(map.clone());
    built.0 = true;
    info!("mundo 3D construído: {}x{} (seed {:#x})", zone.tiles_x, zone.tiles_y, zone.seed);
}

/// Dá corpo visual (billboard colorido) a cada player replicado. Cor derivada do `PlayerId`.
fn hydrate_players(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    players: Query<(Entity, &Player), Without<Billboard>>,
) {
    let hero = asset_server.load("hero.png");
    for (entity, player) in &players {
        let id = player.0 .0;
        let r = ((id >> 16) & 0xFF) as f32 / 255.0;
        let g = ((id >> 8) & 0xFF) as f32 / 255.0;
        let b = (id & 0xFF) as f32 / 255.0;
        // tinta suave (clara) para distinguir players sem lavar o sprite.
        let tint = Color::srgb(0.7 + 0.3 * r, 0.7 + 0.3 * g, 0.7 + 0.3 * b);
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Rectangle::new(BILLBOARD_W, BILLBOARD_HALF * 2.0))),
            MeshMaterial3d(materials.add(billboard_material(tint, hero.clone()))),
            Transform::from_xyz(0.0, BILLBOARD_HALF, 0.0),
            Billboard,
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
    mut entities: Query<(&Position, &mut Transform), With<Billboard>>,
) {
    let face = Quat::from_rotation_y(orbit.yaw);
    for (position, mut transform) in &mut entities {
        let base_y = match map.as_deref() {
            Some(m) => m
                .cell_at_world(position.0)
                .and_then(|(x, y)| m.get(x, y))
                .map(|c| c.height as f32 * LEVEL_HEIGHT)
                .unwrap_or(0.0),
            None => 0.0,
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

/// Q/E giram a câmera livre ao redor do personagem (yaw).
fn rotate_input(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut orbit: ResMut<CamOrbit>) {
    let speed = 1.6;
    if keys.pressed(KeyCode::KeyQ) {
        orbit.yaw -= speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyE) {
        orbit.yaw += speed * time.delta_secs();
    }
}

/// A câmera ORBITA o próprio personagem: foco no player, posição a `dist` com tilt `pitch`
/// e giro `yaw`. Resolve "seguir o player" + "girar ao redor" de uma vez.
fn camera_orbit(
    orbit: Res<CamOrbit>,
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, position)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let focus = Vec3::new(position.0.x, 0.8, position.0.y);
    let offset = Quat::from_rotation_y(orbit.yaw)
        * Vec3::new(0.0, orbit.dist * orbit.pitch.sin(), orbit.dist * orbit.pitch.cos());
    for mut transform in &mut cameras {
        transform.translation = focus + offset;
        transform.look_at(focus, Vec3::Y);
    }
}

/// Botão DIREITO **segurado** → lança um raio da câmera pelo cursor até o chão (plano y=0),
/// converte o ponto na célula (tile) e re-envia o destino (click-to-move contínuo).
fn read_right_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if !buttons.pressed(MouseButton::Right) {
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
    // Interseção com o plano do chão (y = 0).
    let dir = ray.direction;
    if dir.y.abs() < 1e-4 {
        return;
    }
    let t = -ray.origin.y / dir.y;
    if t <= 0.0 {
        return;
    }
    let hit = ray.origin + ray.direction * t;
    let (tx, ty) = (hit.x.round(), hit.z.round());
    if tx < 0.0 || ty < 0.0 {
        return;
    }
    commands.client_trigger(MoveTo { tx: tx as u32, ty: ty as u32 });
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
                    orbit.dist = (orbit.dist * factor).clamp(6.0, 60.0);
                });
        }
        });
}
