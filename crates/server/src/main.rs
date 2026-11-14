//! Servidor headless autoritativo do ragnadofus (Passo 4).
//!
//! Bevy `MinimalPlugins` (sem renderer) + `replicon` (servidor) + transporte `renet2`
//! (UDP). É o dono do estado: ao conectar um cliente, spawna o player dele; recebe o
//! `MoveCommand` (só intenção), valida/aplica como `MoveIntent`, simula em `FixedUpdate`
//! (reusa o `SharedPlugin`) e replica `Position`/`Player` a todos os clientes.
//!
//! Porta via env `RAGNADOFUS_PORT` (default 5000). Bind em `0.0.0.0` para o deploy em
//! container/LAN; localhost serve para o teste entre instâncias locais.

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{NativeSocket, NetcodeServerTransport, ServerAuthentication, ServerSetupConfig},
    renet2::{ConnectionConfig, RenetServer},
    RenetChannelsExt, RepliconRenetPlugins,
};
use shared::{
    build_walkmap, subcell_center, InMemoryRepository, MoveCommand, MoveIntent, Npc, Patrol, Player,
    PlayerId, Position, ProtocolPlugin, SharedPlugin, Velocity, WorldRepository, ZoneSeed,
};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime};

mod pg;

const DEFAULT_PORT: u16 = 5000;
const MAX_CLIENTS: usize = 16;

/// Dono da entidade (server-only, NÃO replicado): liga um [`ClientId`] à entidade do
/// seu player, para rotear o `MoveCommand` à entidade certa.
#[derive(Component, Clone, Copy)]
struct Owner(ClientId);

fn main() -> anyhow::Result<()> {
    let port = std::env::var("RAGNADOFUS_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 64.0))),
        StatesPlugin,
        bevy::log::LogPlugin::default(),
        RepliconPlugins,
        RepliconRenetPlugins,
    ))
    .add_plugins((ProtocolPlugin, SharedPlugin))
    .add_systems(Startup, setup_world)
    .add_observer(on_client_connected)
    .add_observer(on_client_disconnected)
    .add_observer(on_move_command);

    // Carrega a receita da zona via repository (D4.3), backend trocável: Postgres se
    // `DATABASE_URL` estiver setado, senão em memória.
    let repo: Box<dyn WorldRepository> = match std::env::var("DATABASE_URL") {
        Ok(url) => {
            info!("repository: Postgres");
            Box::new(pg::PostgresRepository::new(url))
        }
        Err(_) => {
            info!("repository: em memória (sem DATABASE_URL)");
            Box::new(InMemoryRepository::default())
        }
    };
    let zone = repo.load_or_init_seed()?;
    info!("zona: seed={:#x} ({}x{} tiles)", zone.seed, zone.tiles_x, zone.tiles_y);
    app.insert_resource(zone);

    setup_server(&mut app, port)?;
    info!("servidor ragnadofus ouvindo em 0.0.0.0:{port}");
    app.run();
    Ok(())
}

/// Carrega a walkmap da zona (com uma parede para o A* contornar) e spawna o NPC
/// patrulheiro. No Passo 6 a walkmap/seed virá do Postgres (repository).
fn setup_world(mut commands: Commands, zone: Res<ZoneSeed>) {
    // Walkmap regenerada deterministicamente da receita (seed) — reprodutível (DoD #6).
    let map = build_walkmap(&zone);
    commands.insert_resource(map);

    commands.spawn((
        Npc,
        Position(subcell_center((10, 10))),
        Velocity(Vec2::ZERO),
        MoveIntent(Vec2::ZERO),
        Patrol::new(vec![(245, 10), (10, 10)]),
        Replicated,
    ));
    info!("mundo carregado: walkmap da seed + 1 NPC patrulhando via A*");
}

/// Cria o `RenetServer` + transporte netcode e os insere como recursos.
fn setup_server(app: &mut App, port: u16) -> anyhow::Result<()> {
    let channels = app.world().resource::<RepliconChannels>();
    let server = RenetServer::new(ConnectionConfig::from_channels(
        channels.server_configs(),
        channels.client_configs(),
    ));

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let public_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), port);
    let socket = UdpSocket::bind(public_addr)?;
    let setup = ServerSetupConfig {
        current_time,
        max_token_lifetime: 5,
        max_clients: MAX_CLIENTS,
        protocol_id: shared::protocol::PROTOCOL_ID,
        authentication: ServerAuthentication::Unsecure,
        socket_addresses: vec![vec![public_addr]],
    };
    let native = NativeSocket::new(socket).map_err(|e| anyhow::anyhow!("socket: {e:?}"))?;
    let transport = NetcodeServerTransport::new(setup, native)?;

    app.insert_resource(server);
    app.insert_resource(transport);
    Ok(())
}

/// Ao conectar um cliente, spawna o player autoritativo dele (replicado).
fn on_client_connected(trigger: On<Add<ConnectedClient>>, mut commands: Commands) {
    let client_entity = trigger.event().entity;
    let player_id = PlayerId(client_entity.to_bits());
    info!("cliente conectado: {client_entity} → player {player_id:?}");
    commands.spawn((
        Player(player_id),
        Position(Vec2::ZERO),
        Velocity(Vec2::ZERO),
        MoveIntent(Vec2::ZERO),
        Owner(ClientId::Client(client_entity)),
        Replicated,
    ));
}

/// Ao desconectar, remove o player do cliente.
fn on_client_disconnected(
    trigger: On<Remove<ConnectedClient>>,
    mut commands: Commands,
    players: Query<(Entity, &Owner)>,
) {
    let who = ClientId::Client(trigger.event().entity);
    for (entity, owner) in &players {
        if owner.0 == who {
            commands.entity(entity).despawn();
        }
    }
}

/// Aplica o comando de movimento recebido do cliente à `MoveIntent` do player dele.
/// O servidor é a autoridade: só traduz intenção; a simulação roda no `SharedPlugin`.
fn on_move_command(
    trigger: On<FromClient<MoveCommand>>,
    mut players: Query<(&Owner, &mut MoveIntent)>,
) {
    for (owner, mut intent) in &mut players {
        if owner.0 == trigger.client_id {
            intent.0 = trigger.message.0;
        }
    }
}
