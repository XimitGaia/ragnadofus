//! Protocolo de replicação (P4) — registro IDÊNTICO no cliente e no servidor (o hash do
//! protocolo precisa bater nos dois lados). Componentes autoritativos do servidor são
//! replicados aos clientes; o cliente envia apenas o comando de movimento (intenção).

use crate::components::{Player, Position};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

/// Id de protocolo do netcode. Cliente e servidor PRECISAM usar o mesmo valor.
pub const PROTOCOL_ID: u64 = 0x5241_474E_4144_4F01; // "RAGNADO" + versão 01

/// Comando de movimento do cliente → servidor. Só a direção desejada (intenção); o
/// servidor valida e simula (anti-cheat, padroes §2).
#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MoveCommand(#[serde(with = "crate::serde_helpers::vec2")] pub Vec2);

/// Registra o protocolo. Adicionado por ambos (cliente e servidor), depois de
/// `RepliconPlugins`.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Position>()
            .replicate::<Player>()
            .add_client_event::<MoveCommand>(Channel::Ordered);
    }
}
