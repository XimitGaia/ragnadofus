//! `shared` — ECS do jogo em Bevy puro, compartilhado por cliente e servidor.
//!
//! Regras (padroes §1):
//! - Sem `bevy_render`: só a base ECS do Bevy. O servidor headless compila sobre este
//!   mesmo crate, provando o desacoplamento do renderer a cada build.
//! - Grafo unidirecional: `client → shared`, `server → shared`; `shared` NUNCA depende
//!   de `client`/`server`.
//! - Nunca guardar `Entity` para rede/persistência — usar newtypes ([`PlayerId`],
//!   [`Tick`]).

use bevy::prelude::*;

pub mod components;
pub mod ids;
pub mod movement;
pub mod network;
pub mod walkmap;

pub use components::{MoveIntent, Npc, Player, Position, Velocity};
pub use ids::{PlayerId, Tick};
pub use movement::{MovementSet, WALK_SPEED};
pub use network::{ClientMessage, ServerMessage};
pub use walkmap::{resolve_move, SubCell, Walkmap};

/// Plugin raiz do ECS compartilhado. No Passo 1 é um esqueleto; sistemas de simulação
/// (`FixedUpdate`), replicação e componentes de gameplay entram nos passos seguintes.
pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        // Núcleo determinístico em FixedUpdate (D4.4): intenção → velocidade → posição,
        // encadeado para ordem estável. Mesmo conjunto roda no cliente (local, Passo 2)
        // e no servidor (autoritativo, Passo 4).
        app.add_systems(
            FixedUpdate,
            (movement::velocity_from_intent, movement::integrate_positions)
                .chain()
                .in_set(MovementSet),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::World;

    /// Teste trivial para o gate ter o que rodar (`cargo test`), e que já exercita o
    /// ECS do Bevy de forma headless: spawnar uma entidade com componentes de `shared`.
    #[test]
    fn spawna_player_no_world_headless() {
        let mut world = World::new();
        let entity = world
            .spawn((Player(PlayerId(1)), Position(Vec2::ZERO), Velocity(Vec2::ZERO)))
            .id();

        assert!(world.get::<Player>(entity).is_some());
        assert_eq!(world.get::<Position>(entity).map(|p| p.0), Some(Vec2::ZERO));
    }

    #[test]
    fn tick_avanca() {
        let mut tick = Tick(0);
        tick = tick.next();
        assert_eq!(tick, Tick(1));
    }
}
