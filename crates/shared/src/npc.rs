//! IA do NPC (P5) — patrulha server-authoritative seguindo um caminho A* entre `goals`,
//! em loop. Roda em `FixedUpdate` no servidor (reusa a sim de movimento); o `Position`
//! resultante é replicado a todos os clientes, igual para todos (D2.4).

use crate::components::{MoveIntent, Npc, Position};
use crate::pathfinding::find_path;
use crate::walkmap::{cell_center, Walkmap};
use bevy::prelude::*;

/// Distância (px) para considerar que chegou ao waypoint atual.
const WAYPOINT_EPS: f32 = 0.15;

/// Estado de patrulha do NPC: percorre `goals` em ordem cíclica, recalculando o path A*
/// a cada troca de objetivo.
#[derive(Component, Debug, Clone)]
pub struct Patrol {
    goals: Vec<(u32, u32)>,
    goal_idx: usize,
    path: Vec<(u32, u32)>,
    step: usize,
}

impl Patrol {
    /// Cria uma patrulha cíclica pelos objetivos (sub-células) dados.
    #[must_use]
    pub fn new(goals: Vec<(u32, u32)>) -> Self {
        Self { goals, goal_idx: 0, path: Vec::new(), step: 0 }
    }
}

/// Planeja (A*) e segue o caminho, setando `MoveIntent` rumo ao próximo waypoint.
pub fn npc_ai(
    map: Option<Res<Walkmap>>,
    mut npcs: Query<(&Position, &mut MoveIntent, &mut Patrol), With<Npc>>,
) {
    let Some(map) = map.as_deref() else {
        return;
    };
    for (position, mut intent, mut patrol) in &mut npcs {
        if patrol.goals.is_empty() {
            intent.0 = Vec2::ZERO;
            continue;
        }

        // (Re)planeja quando não há waypoints pendentes.
        if patrol.step >= patrol.path.len() {
            let Some(start) = map.cell_at_world(position.0) else {
                intent.0 = Vec2::ZERO;
                continue;
            };
            let goal = patrol.goals[patrol.goal_idx];
            match find_path(map, start, goal) {
                Some(path) if path.len() > 1 => {
                    patrol.path = path;
                    patrol.step = 1; // índice 0 é a célula atual
                }
                _ => {
                    // Chegou ao goal (ou sem caminho): vai para o próximo.
                    patrol.goal_idx = (patrol.goal_idx + 1) % patrol.goals.len();
                    patrol.path.clear();
                    patrol.step = 0;
                    intent.0 = Vec2::ZERO;
                    continue;
                }
            }
        }

        let Some(&waypoint) = patrol.path.get(patrol.step) else {
            intent.0 = Vec2::ZERO;
            continue;
        };
        let to_waypoint = cell_center(waypoint) - position.0;
        if to_waypoint.length() <= WAYPOINT_EPS {
            patrol.step += 1;
            intent.0 = Vec2::ZERO;
        } else {
            intent.0 = to_waypoint; // o shared normaliza em velocity_from_intent
        }
    }
}
