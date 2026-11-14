//! Simulação de movimento, compartilhada por cliente e servidor. A lógica vive aqui,
//! em systems de `FixedUpdate` (núcleo determinístico — D4.4 / padroes §4), nunca no
//! cliente: no Passo 2 o cliente só escreve [`MoveIntent`]; no Passo 4 o servidor vira
//! o dono e valida. Sem rede, sem render.

use crate::components::{MoveIntent, Position, Velocity};
use bevy::prelude::*;

/// Velocidade de caminhada, em px/s no plano do mundo.
pub const WALK_SPEED: f32 = 64.0;

/// Conjunto dos sistemas de movimento, rodando encadeados em `FixedUpdate`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MovementSet;

/// Integra uma posição por `dt` segundos dada a velocidade. Função pura — base dos
/// testes determinísticos (contrato `(seed, rules, engine)` do P4/P6).
#[must_use]
pub fn step_position(position: Vec2, velocity: Vec2, dt: f32) -> Vec2 {
    position + velocity * dt
}

/// Converte a intenção (direção) em velocidade de caminhada. Normaliza para que a
/// diagonal não ande mais rápido.
pub fn velocity_from_intent(mut query: Query<(&MoveIntent, &mut Velocity)>) {
    for (intent, mut velocity) in &mut query {
        velocity.0 = intent.0.normalize_or_zero() * WALK_SPEED;
    }
}

/// Integra as posições pela velocidade, no passo fixo.
pub fn integrate_positions(time: Res<Time<Fixed>>, mut query: Query<(&Velocity, &mut Position)>) {
    let dt = time.delta_secs();
    for (velocity, mut position) in &mut query {
        position.0 = step_position(position.0, velocity.0, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::ecs::world::World;

    #[test]
    fn step_position_integra_linearmente() {
        let p = step_position(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), 0.5);
        assert_eq!(p, Vec2::new(5.0, 0.0));
    }

    #[test]
    fn player_anda_na_simulacao_fixedupdate() {
        use crate::SharedPlugin;
        use crate::components::Position;
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        // Passo fixo determinístico: cada update() avança 1/64s de tempo virtual.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 64.0,
        )));
        app.add_plugins(SharedPlugin);

        let player = app
            .world_mut()
            .spawn((
                MoveIntent(Vec2::new(1.0, 0.0)),
                Velocity(Vec2::ZERO),
                Position(Vec2::ZERO),
            ))
            .id();

        // ~1s de simulação.
        for _ in 0..64 {
            app.update();
        }

        match app.world().get::<Position>(player).map(|p| p.0.x) {
            // andou para +x (~64px); assertiva frouxa para não depender do nº exato de passos.
            Some(x) => assert!(x > 1.0, "player deveria ter andado para +x, x={x}"),
            None => panic!("sem Position"),
        }
    }

    #[test]
    fn velocidade_vem_da_intencao_e_diagonal_nao_acelera() {
        let mut world = World::new();
        let entity = world
            .spawn((MoveIntent(Vec2::new(1.0, 1.0)), Velocity(Vec2::ZERO)))
            .id();

        assert!(world.run_system_once(velocity_from_intent).is_ok());

        let v = world.get::<Velocity>(entity).map(|v| v.0.length());
        // diagonal normalizada => módulo == WALK_SPEED (não WALK_SPEED*sqrt(2)).
        match v {
            Some(len) => assert!((len - WALK_SPEED).abs() < 1e-3, "len={len}"),
            None => panic!("sem Velocity"),
        }
    }
}
