//! Ordem de movimento por DESTINO (click-to-move estilo Ragnarok). O cliente clica um
//! ponto; o servidor roda A* (que já existe) e preenche o caminho; esta lógica segue os
//! waypoints setando `MoveIntent` (reusa a mesma ideia do NPC/`Patrol`). Roda no servidor,
//! em `FixedUpdate`.

use crate::components::{Elevation, MoveIntent, Position};
use crate::walkmap::cell_center;
use bevy::prelude::*;

const WAYPOINT_EPS: f32 = 0.15;

/// Caminho que o personagem deve seguir: `(x, y, superfície)` por waypoint (multi-nível),
/// com o índice do waypoint atual.
#[derive(Component, Debug, Clone)]
pub struct MoveOrder {
    path: Vec<(u32, u32, i32)>,
    step: usize,
}

impl MoveOrder {
    #[must_use]
    pub fn new(path: Vec<(u32, u32, i32)>) -> Self {
        Self { path, step: 1 } // índice 0 é a célula atual
    }
}

/// Segue o `MoveOrder`: seta `MoveIntent` rumo ao próximo waypoint e atualiza a `Elevation`
/// para a superfície do trecho atual (sobe pela escada). Ao chegar ao fim, remove a ordem.
pub fn follow_move_order(
    mut commands: Commands,
    mut query: Query<(Entity, &Position, &mut MoveIntent, &mut MoveOrder, Option<&mut Elevation>)>,
) {
    for (entity, position, mut intent, mut order, mut elevation) in &mut query {
        let Some(&(wx, wy, _)) = order.path.get(order.step) else {
            intent.0 = Vec2::ZERO;
            commands.entity(entity).remove::<MoveOrder>();
            continue;
        };
        let to_waypoint = cell_center((wx, wy)) - position.0;
        if to_waypoint.length() <= WAYPOINT_EPS {
            order.step += 1;
            intent.0 = Vec2::ZERO;
        } else {
            intent.0 = to_waypoint;
        }
        // Elevação = superfície do waypoint "base" (o nível do trecho atual do caminho).
        let base = order.step.saturating_sub(1).min(order.path.len().saturating_sub(1));
        if let (Some(elevation), Some(&(_, _, surf))) = (elevation.as_mut(), order.path.get(base)) {
            elevation.0 = surf;
        }
    }
}
