//! A* sobre a walkmap (P5) — agora **ciente de nível** (2.5D): os nós são
//! `(x, y, superfície)`. Cada passo só liga superfícies a ≤ [`STEP_MAX`] de altura (a
//! escada, com superfícies de +1, vira a transição térreo↔2º andar↔telhado). Determinístico
//! (ordem estável de vizinhos, tie-break por nó). Vizinhança 8-conexa (anda em diagonal,
//! estilo RO), com proteção anti "corte de canto".

use crate::walkmap::{Walkmap, STEP_MAX};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

type Cell = (u32, u32);
/// Nó do A*: célula + a superfície (altura) em que se está nela.
type Node3 = (u32, u32, i32);

/// Distância de Chebyshev (heurística admissível na grade 8-conexa com custo uniforme).
fn heuristic(a: Cell, b: Cell) -> u32 {
    a.0.abs_diff(b.0).max(a.1.abs_diff(b.1))
}

/// Vizinhos 8-conexos (ortogonais + diagonais) dentro dos limites, em ordem estável.
fn neighbors(map: &Walkmap, c: Cell) -> impl Iterator<Item = Cell> + '_ {
    let (x, y) = c;
    [
        (x.wrapping_sub(1), y),
        (x + 1, y),
        (x, y.wrapping_sub(1)),
        (x, y + 1),
        (x.wrapping_sub(1), y.wrapping_sub(1)),
        (x + 1, y.wrapping_sub(1)),
        (x.wrapping_sub(1), y + 1),
        (x + 1, y + 1),
    ]
    .into_iter()
    .filter(move |&(nx, ny)| nx < map.width() && ny < map.height())
}

/// Nó da fila de prioridade (min-heap via `Reverse` manual em `Ord`).
#[derive(PartialEq, Eq)]
struct QNode {
    f: u32,
    node: Node3,
}

impl Ord for QNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // menor f primeiro (heap é max → invertemos); tie-break estável por nó.
        other.f.cmp(&self.f).then_with(|| other.node.cmp(&self.node))
    }
}

impl PartialOrd for QNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* de `start` (na superfície `start_surf`) até a CÉLULA `goal` (em qualquer superfície
/// alcançável). Retorna o caminho de `(x, y, superfície)` (incluindo start) ou `None`.
#[must_use]
pub fn find_path(
    map: &Walkmap,
    start: Cell,
    start_surf: i32,
    goal: Cell,
    target_elev: i32,
) -> Option<Vec<Node3>> {
    // Superfície-alvo na célula destino: a mais próxima de `target_elev` (ex.: clicar a
    // escada com target_elev alto → chega NO TELHADO, não na base dela no chão).
    let goal_surf = map.surface_near(goal.0, goal.1, target_elev)?;
    let start_node = (start.0, start.1, start_surf);
    if start.0 == goal.0 && start.1 == goal.1 && start_surf == goal_surf {
        return Some(vec![start_node]);
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<Node3, Node3> = HashMap::new();
    let mut g_score: HashMap<Node3, u32> = HashMap::new();

    g_score.insert(start_node, 0);
    open.push(QNode { f: heuristic(start, goal), node: start_node });

    while let Some(QNode { node, .. }) = open.pop() {
        let (x, y, surf) = node;
        if x == goal.0 && y == goal.1 && surf == goal_surf {
            return Some(reconstruct(&came_from, node));
        }
        let current_g = *g_score.get(&node).unwrap_or(&u32::MAX);
        for (nx, ny) in neighbors(map, (x, y)) {
            // Diagonal: proíbe "cortar canto" — os dois ortogonais precisam ser pisáveis
            // a partir da superfície atual.
            if nx != x && ny != y && (!map.reachable_from((nx, y), surf) || !map.reachable_from((x, ny), surf)) {
                continue;
            }
            for s2 in map.surfaces(nx, ny) {
                if (s2 - surf).abs() > STEP_MAX {
                    continue; // degrau grande demais entre superfícies
                }
                let next = (nx, ny, s2);
                let tentative = current_g.saturating_add(1);
                if tentative < *g_score.get(&next).unwrap_or(&u32::MAX) {
                    came_from.insert(next, node);
                    g_score.insert(next, tentative);
                    open.push(QNode {
                        f: tentative.saturating_add(heuristic((nx, ny), goal)),
                        node: next,
                    });
                }
            }
        }
    }
    None
}

fn reconstruct(came_from: &HashMap<Node3, Node3>, goal: Node3) -> Vec<Node3> {
    let mut path = vec![goal];
    let mut current = goal;
    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walkmap::{SubCell, Walkmap};

    #[test]
    fn caminho_reto_em_mapa_plano() {
        let map = Walkmap::flat(4, 4);
        let Some(path) = find_path(&map, (0, 0), 0, (3, 0), 0) else {
            panic!("deve haver caminho");
        };
        assert_eq!(path.first(), Some(&(0, 0, 0)));
        assert_eq!(path.last(), Some(&(3, 0, 0)));
        assert_eq!(path.len(), 4); // 0,1,2,3 em x
    }

    #[test]
    fn contorna_obstaculo() {
        // parede vertical em x=2, y=0..2 (não caminhável); precisa desviar por y=3.
        let mut map = Walkmap::flat(5, 5);
        for y in 0..3 {
            map.set(2, y, SubCell { height: 0, walkable: false });
        }
        let Some(path) = find_path(&map, (0, 0), 0, (4, 0), 0) else {
            panic!("deve contornar o obstaculo");
        };
        assert_eq!(path.first(), Some(&(0, 0, 0)));
        assert_eq!(path.last(), Some(&(4, 0, 0)));
        // nenhum passo passa pela parede
        assert!(path.iter().all(|&(x, y, _)| !(x == 2 && y < 3)));
    }

    #[test]
    fn sem_caminho_quando_cercado() {
        let mut map = Walkmap::flat(5, 5);
        for (x, y) in [(3, 4), (5, 4), (4, 3), (4, 5)] {
            map.set(x, y, SubCell { height: 0, walkable: false });
        }
        assert!(find_path(&map, (0, 0), 0, (4, 4), 0).is_none());
    }

    #[test]
    fn sobe_por_superficies_de_um_degrau() {
        // rampa de superfícies extras: (1,0)=1, (2,0)=2; térreo em 0. Deve alcançar (2,0).
        let mut map = Walkmap::flat(4, 4);
        map.add_surface(1, 0, 1);
        map.add_surface(2, 0, 2);
        let Some(path) = find_path(&map, (0, 0), 0, (2, 0), 2) else {
            panic!("deve subir a rampa");
        };
        // termina na superfície mais alta alcançável da célula destino
        assert_eq!(path.last().map(|&(x, y, _)| (x, y)), Some((2, 0)));
    }
}
