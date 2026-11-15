//! Ordem de movimento por DESTINO (click-to-move estilo Ragnarok). O cliente clica um
//! ponto; o servidor roda A* (que já existe) e preenche o caminho; esta lógica segue os
//! waypoints setando `MoveIntent` (reusa a mesma ideia do NPC/`Patrol`). Roda no servidor,
//! em `FixedUpdate`.

use crate::components::{MoveIntent, Position};
use crate::walkmap::cell_center;
use bevy::prelude::*;

const WAYPOINT_EPS: f32 = 0.15;

/// Caminho (sub-células) que o personagem deve seguir, com o índice do waypoint atual.
#[derive(Component, Debug, Clone)]
pub struct MoveOrder {
    path: Vec<(u32, u32)>,
    step: usize,
}

impl MoveOrder {
    #[must_use]
    pub fn new(path: Vec<(u32, u32)>) -> Self {
        Self { path, step: 1 } // índice 0 é a célula atual
    }
}

/// Segue o `MoveOrder`: seta `MoveIntent` rumo ao próximo waypoint; ao chegar ao fim,
/// remove a ordem e para.
pub fn follow_move_order(
    mut commands: Commands,
    mut query: Query<(Entity, &Position, &mut MoveIntent, &mut MoveOrder)>,
) {
    for (entity, position, mut intent, mut order) in &mut query {
        let Some(&waypoint) = order.path.get(order.step) else {
            intent.0 = Vec2::ZERO;
            commands.entity(entity).remove::<MoveOrder>();
            continue;
        };
        let to_waypoint = cell_center(waypoint) - position.0;
        if to_waypoint.length() <= WAYPOINT_EPS {
            order.step += 1;
            intent.0 = Vec2::ZERO;
        } else {
            intent.0 = to_waypoint;
        }
    }
}
