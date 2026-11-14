//! Componentes de gameplay, granulares e data-oriented (padroes §1): markers + dados
//! pequenos, nunca um `struct Player` monolítico. A lógica mora em systems (próximos
//! passos), não aqui.

use crate::ids::PlayerId;
use bevy::prelude::*;

/// Posição no plano do mundo (a elevação real entra na walkmap do Passo 2/3).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Position(pub Vec2);

/// Velocidade atual da entidade.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Velocity(pub Vec2);

/// Marker + id estável de um jogador.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player(pub PlayerId);

/// Marker do NPC controlado pelo servidor (A* replicado — D2.4).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Npc;
