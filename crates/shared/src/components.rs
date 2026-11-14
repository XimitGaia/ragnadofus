//! Componentes de gameplay, granulares e data-oriented (padroes §1): markers + dados
//! pequenos, nunca um `struct Player` monolítico. A lógica mora em systems (próximos
//! passos), não aqui.

use crate::ids::PlayerId;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Posição no plano do mundo (a elevação real entra na walkmap do Passo 2/3). Replicada
/// do servidor aos clientes (P4) — `Vec2` serializado como `(f32, f32)`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position(#[serde(with = "crate::serde_helpers::vec2")] pub Vec2);

/// Velocidade atual da entidade.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Velocity(pub Vec2);

/// Marker + id estável de um jogador. Replicado (P4) para o cliente saber que a
/// entidade é um jogador.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player(pub PlayerId);

/// Marker do NPC controlado pelo servidor (A* replicado — D2.4). Replicado para o
/// cliente renderizar o NPC.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Npc;

/// Intenção de movimento: a direção desejada (não normalizada). O cliente a escreve a
/// partir do input; a simulação em `shared` a transforma em [`Velocity`]. É só intenção
/// — no Passo 4 o servidor passa a validá-la (anti-cheat, padroes §2).
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct MoveIntent(pub Vec2);
