//! Conexão / netcode do cliente — replicon + transporte renet2/UDP (feature-plugin).

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NativeSocket, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt,
};
use shared::AssignedPlayer;

use crate::MyPlayer;

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

/// Lê o servidor-alvo (`RAGNADOFUS_SERVER`) e arma a conexão pós-warmup do render.
pub(crate) fn plugin(app: &mut App) {
    let server_addr: SocketAddr = std::env::var("RAGNADOFUS_SERVER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            DEFAULT_SERVER.parse().unwrap_or(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000))
        });
    info!("cliente ragnadofus (3D) — conectará a {server_addr} após warmup do render");
    app.init_resource::<MyPlayer>()
        .insert_resource(PendingConnect { addr: server_addr, frames: 0, attempted: false })
        .add_systems(Update, connect_when_ready)
        .add_observer(on_assigned_player);
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

/// Guarda qual player é o deste cliente (recebido do servidor).
fn on_assigned_player(trigger: On<AssignedPlayer>, mut me: ResMut<MyPlayer>) {
    me.0 = Some(trigger.event().0);
    info!("meu player: {:?}", trigger.event().0);
}
