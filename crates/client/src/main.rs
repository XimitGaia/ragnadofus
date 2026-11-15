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

use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_ecs_tilemap::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NativeSocket, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt, RepliconRenetPlugins,
};
use shared::{MoveCommand, Npc, Player, Position, ProtocolPlugin};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

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
    .add_systems(Startup, setup_scene)
    .add_systems(
        Update,
        (read_input, hydrate_players, hydrate_npcs, sync_transforms).chain(),
    );

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

/// Câmera estática + zona 2.5D isométrica (64×64, igual à fatia 2b).
fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));

    let texture: Handle<Image> = asset_server.load("tiles.png");
    let map_size = TilemapSize { x: 64, y: 64 };
    let mut storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();
    fill_tilemap_rect(
        TileTextureIndex(0),
        TilePos { x: 0, y: 0 },
        map_size,
        TilemapId(tilemap_entity),
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
}

/// Lê WASD/setas e envia o comando de movimento ao servidor (só quando há direção).
fn read_input(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
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
    if dir != Vec2::ZERO {
        commands.client_trigger(MoveCommand(dir.normalize_or_zero()));
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
            Sprite::from_color(Color::srgb(0.3 + 0.6 * r, 0.3 + 0.6 * g, 0.3 + 0.6 * b), Vec2::new(16.0, 24.0)),
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

/// Dá corpo visual ao NPC replicado (sprite azul, distinto dos players).
fn hydrate_npcs(mut commands: Commands, npcs: Query<Entity, (With<Npc>, Without<Sprite>)>) {
    for entity in &npcs {
        commands.entity(entity).insert((
            Sprite::from_color(Color::srgb(0.2, 0.5, 0.95), Vec2::new(16.0, 24.0)),
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

/// Espelha a `Position` replicada (players e NPC) no `Transform` de render (mantém z).
fn sync_transforms(mut entities: Query<(&Position, &mut Transform)>) {
    for (position, mut transform) in &mut entities {
        transform.translation.x = position.0.x;
        transform.translation.y = position.0.y;
    }
}
