//! Protocolo de replicação (P4) — registro IDÊNTICO no cliente e no servidor (o hash do
//! protocolo precisa bater nos dois lados). Componentes autoritativos do servidor são
//! replicados aos clientes; o cliente envia apenas o comando de movimento (intenção).

use crate::components::{Elevation, Npc, Player, Position};
use crate::ids::PlayerId;
use crate::world::ZoneSeed;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

/// Id de protocolo do netcode. Cliente e servidor PRECISAM usar o mesmo valor.
pub const PROTOCOL_ID: u64 = 0x5241_474E_4144_4F01; // "RAGNADO" + versão 01

/// Server → cliente: diz ao cliente qual é o player dele (para a câmera seguir).
#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AssignedPlayer(pub PlayerId);

/// Destino de movimento do cliente → servidor (TILE clicado). O servidor roda A* e move
/// o player autoritativamente (click-to-move estilo Ragnarok). Só o destino — o servidor
/// valida e pathfinda (anti-cheat, padroes §2).
#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MoveTo {
    pub tx: u32,
    pub ty: u32,
    /// Elevação-alvo (multi-nível): o servidor escolhe a superfície da célula destino mais
    /// próxima disso (ex.: clicar a escada manda `te` alto → sobe pro telhado).
    pub te: i32,
}

/// Cliente → servidor: pedido de DESCER um nível (tecla Enter). O servidor decide se há
/// superfície abaixo e baixa a [`Elevation`] (multi-nível 2.5D).
#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Jump;

/// Cliente → servidor: PULO PRA FRENTE (tecla Espaço). O servidor salta o player ~1 tile na
/// direção em que ele olha e o faz POUSAR na superfície de lá (cai de borda, se houver).
#[derive(Event, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hop;

/// Registra o protocolo. Adicionado por ambos (cliente e servidor), depois de
/// `RepliconPlugins`.
pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Position>()
            .replicate::<Player>()
            .replicate::<Npc>()
            .replicate::<Elevation>()
            .add_client_event::<MoveTo>(Channel::Ordered)
            .add_client_event::<Jump>(Channel::Ordered)
            .add_client_event::<Hop>(Channel::Ordered)
            .add_server_event::<AssignedPlayer>(Channel::Ordered)
            .add_server_event::<ZoneSeed>(Channel::Ordered);
    }
}
