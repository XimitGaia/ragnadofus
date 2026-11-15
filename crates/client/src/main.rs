//! Cliente desktop do ragnadofus (Passo 4 — em rede).
//!
//! Conecta a um servidor autoritativo (`replicon` + `renet2`/UDP), envia o input como
//! `MoveCommand` (só intenção) e **renderiza as entidades replicadas** (players) na zona
//! 2.5D isométrica. Não roda a simulação — a autoridade é do servidor.
//!
//! Servidor via env `RAGNADOFUS_SERVER` (ex.: `127.0.0.1:5000`; default localhost:5000).
//! Para o teste multiplayer: suba o `server` e abra 2+ instâncias deste `client`.
//!
//! Câmera estática centrada na zona (seguir o próprio player + clique-pra-mover voltam
//! quando a identidade do próprio player for replicada / com A* no servidor — Passo 5).

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_ecs_tilemap::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NativeSocket, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt, RepliconRenetPlugins,
};
use shared::{
    build_walkmap, AssignedPlayer, MoveTo, Npc, Player, PlayerId, Position, ProtocolPlugin, ZoneSeed,
};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

/// Player controlado por este cliente (recebido do servidor via `AssignedPlayer`).
#[derive(Resource, Default)]
struct MyPlayer(Option<PlayerId>);

/// Evita redesenhar a zona se a `ZoneSeed` chegar mais de uma vez.
#[derive(Resource, Default)]
struct ZoneBuilt(bool);

fn main() -> anyhow::Result<()> {
    let server_addr: SocketAddr = std::env::var("RAGNADOFUS_SERVER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| DEFAULT_SERVER.parse().unwrap_or(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000)));

    let mut app = App::new();
    // Atualiza o app continuamente mesmo SEM foco da janela — senão o loop throttla, o
    // cliente para de mandar keepalive do netcode e a conexão cai (timeout). Essencial
    // para multiplayer com janela em segundo plano.
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            }),
    )
    .add_plugins(TilemapPlugin)
    .add_plugins((RepliconPlugins, RepliconRenetPlugins))
    .add_plugins(ProtocolPlugin)
    .init_resource::<MyPlayer>()
    .init_resource::<ZoneBuilt>()
    .add_observer(on_assigned_player)
    .add_observer(on_zone_received)
    .add_systems(Startup, setup_scene)
    .add_systems(
        Update,
        (read_right_click, hydrate_players, hydrate_npcs, sync_transforms, camera_follow).chain(),
    )
    .add_systems(Update, zoom_control);

    connect(&mut app, server_addr)?;
    info!("cliente ragnadofus conectando a {server_addr}");
    app.run();
    Ok(())
}

/// Cria o `RenetClient` + transporte netcode e os insere como recursos.
fn connect(app: &mut App, server_addr: SocketAddr) -> anyhow::Result<()> {
    let channels = app.world().resource::<RepliconChannels>();
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

    app.insert_resource(client);
    app.insert_resource(transport);
    Ok(())
}

/// Só a câmera. A zona é desenhada quando a `ZoneSeed` chega do servidor.
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Recebe a receita da zona e desenha o tilemap 2.5D: xadrez no chão caminhável, **tile
/// de pedra nas paredes** (reconstruindo a mesma walkmap do servidor a partir da seed).
fn on_zone_received(
    trigger: On<ZoneSeed>,
    mut built: ResMut<ZoneBuilt>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if built.0 {
        return;
    }
    let zone: ZoneSeed = *trigger.event();
    let map = build_walkmap(&zone);
    let texture: Handle<Image> = asset_server.load("tiles.png");
    let map_size = TilemapSize { x: zone.tiles_x, y: zone.tiles_y };
    let mut storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    for y in 0..map_size.y {
        for x in 0..map_size.x {
            let is_wall = map.get(x, y).map(|c| !c.walkable).unwrap_or(false);
            // pedra (índice 2) na parede; xadrez grama/terra no chão.
            let index = if is_wall { 2 } else { (x + y) % 2 };
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    texture_index: TileTextureIndex(index),
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .id();
            storage.set(&tile_pos, tile_entity);
        }
    }

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
    built.0 = true;
    info!("zona desenhada: {}x{} (seed {:#x})", zone.tiles_x, zone.tiles_y, zone.seed);
}

/// Clique DIREITO → converte o cursor na célula (tile) via `from_world_pos` e envia o
/// destino ao servidor (click-to-move estilo Ragnarok).
fn read_right_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let (Ok(window), Ok((camera, camera_tf))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(camera_tf, cursor) else {
        return;
    };
    let map_size = TilemapSize { x: 64, y: 64 };
    let grid_size = TilemapGridSize { x: 64.0, y: 32.0 };
    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let anchor = TilemapAnchor::Center;
    if let Some(tile) =
        TilePos::from_world_pos(&world, &map_size, &grid_size, &tile_size, &map_type, &anchor)
    {
        commands.client_trigger(MoveTo { tx: tile.x, ty: tile.y });
    }
}

/// Dá corpo visual (sprite) a cada player replicado que ainda não tem — cor derivada do
/// `PlayerId` para distinguir jogadores.
fn hydrate_players(mut commands: Commands, players: Query<(Entity, &Player), Without<Sprite>>) {
    for (entity, player) in &players {
        let id = player.0 .0;
        let r = ((id >> 16) & 0xFF) as f32 / 255.0;
        let g = ((id >> 8) & 0xFF) as f32 / 255.0;
        let b = (id & 0xFF) as f32 / 255.0;
        commands.entity(entity).insert((
            Sprite {
                color: Color::srgb(0.3 + 0.6 * r, 0.3 + 0.6 * g, 0.3 + 0.6 * b),
                custom_size: Some(Vec2::new(32.0, 48.0)), // ~0,5 tile largura, ~1,5 tile altura
                ..default()
            },
            Anchor::BOTTOM_CENTER, // pés no centro do tile (ajuda o y-sort)
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

/// Dá corpo visual ao NPC replicado (sprite azul, distinto dos players).
fn hydrate_npcs(mut commands: Commands, npcs: Query<Entity, (With<Npc>, Without<Sprite>)>) {
    for entity in &npcs {
        commands.entity(entity).insert((
            Sprite {
                color: Color::srgb(0.2, 0.5, 0.95),
                custom_size: Some(Vec2::new(32.0, 48.0)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

/// Base AFIM da projeção iso (origem + deltas de tela por +1 tile em x e y), derivada dos
/// helpers do `bevy_ecs_tilemap` — alinha EXATAMENTE com os losangos desenhados. Como a
/// projeção iso é afim nas coordenadas de tile, isto projeta posições CONTÍNUAS também.
fn iso_basis() -> (Vec2, Vec2, Vec2) {
    let map_size = TilemapSize { x: 64, y: 64 };
    let grid_size = TilemapGridSize { x: 64.0, y: 32.0 };
    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let anchor = TilemapAnchor::Center;
    let center = |x: u32, y: u32| {
        TilePos { x, y }.center_in_world(&map_size, &grid_size, &tile_size, &map_type, &anchor)
    };
    let origin = center(0, 0);
    (origin, center(1, 0) - origin, center(0, 1) - origin)
}

/// Projeta uma posição do mundo (unidades de tile) para a tela iso.
fn project(pos: Vec2, origin: Vec2, unit_x: Vec2, unit_y: Vec2) -> Vec2 {
    origin + pos.x * unit_x + pos.y * unit_y
}

/// Projeta a `Position` replicada (players e NPC) para o `Transform` de render no iso.
fn sync_transforms(mut entities: Query<(&Position, &mut Transform)>) {
    let (origin, unit_x, unit_y) = iso_basis();
    for (position, mut transform) in &mut entities {
        let screen = project(position.0, origin, unit_x, unit_y);
        transform.translation.x = screen.x;
        transform.translation.y = screen.y;
    }
}

/// Guarda qual player é o deste cliente (recebido do servidor).
fn on_assigned_player(trigger: On<AssignedPlayer>, mut me: ResMut<MyPlayer>) {
    me.0 = Some(trigger.event().0);
    info!("meu player: {:?}", trigger.event().0);
}

/// A câmera segue o PRÓPRIO player (resolve "o personagem sai do centro").
fn camera_follow(
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, position)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let (origin, unit_x, unit_y) = iso_basis();
    let screen = project(position.0, origin, unit_x, unit_y);
    for mut transform in &mut cameras {
        transform.translation.x = screen.x;
        transform.translation.y = screen.y;
    }
}

/// Zoom in/out com a roda do mouse (scroll). `scale` menor = mais zoom.
fn zoom_control(
    mut wheel: MessageReader<MouseWheel>,
    mut cameras: Query<&mut Projection, With<Camera2d>>,
) {
    let mut delta = 0.0;
    for event in wheel.read() {
        delta += event.y;
    }
    if delta == 0.0 {
        return;
    }
    for mut projection in &mut cameras {
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scale = (ortho.scale * (1.0 - delta * 0.1)).clamp(0.1, 2.0);
        }
    }
}
