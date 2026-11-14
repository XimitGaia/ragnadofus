//! Mensagens de rede, serializáveis (via `serde`; o transporte usa `postcard` — RFC
//! 0005). Princípio anti-cheat (padroes §2): o cliente envia apenas **intenção**; o
//! servidor valida tudo e replica o estado resultante.
//!
//! Vetores viajam como pares `f32` crus (e não `Vec2` do Bevy) para manter o contrato de
//! wire explícito e independente do glam.

use crate::ids::PlayerId;
use serde::{Deserialize, Serialize};

/// Mensagem do cliente para o servidor — somente intenção.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Intenção de mover numa direção (será normalizada/validada pelo servidor).
    MoveIntent { dx: f32, dy: f32 },
}

/// Mensagem do servidor para o cliente — estado autoritativo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Posição autoritativa de um jogador após a simulação do tick.
    PlayerMoved { player: PlayerId, x: f32, y: f32 },
}
