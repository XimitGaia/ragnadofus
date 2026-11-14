//! A* sobre a walkmap (P5) — caminho de sub-células respeitando caminhabilidade e degrau
//! (reusa [`Walkmap::passable`]). Determinístico (ordem estável de vizinhos, tie-break por
//! coordenada) para reprodutibilidade. Vizinhança 4-conexa.

use crate::walkmap::Walkmap;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

type Cell = (u32, u32);

/// Distância de Manhattan (heurística admissível na grade 4-conexa).
fn heuristic(a: Cell, b: Cell) -> u32 {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

/// Vizinhos 4-conexos dentro dos limites do mapa, em ordem estável.
fn neighbors(map: &Walkmap, c: Cell) -> impl Iterator<Item = Cell> + '_ {
    let (x, y) = c;
    [
        (x.wrapping_sub(1), y),
        (x + 1, y),
        (x, y.wrapping_sub(1)),
        (x, y + 1),
    ]
    .into_iter()
    .filter(move |&(nx, ny)| nx < map.width() && ny < map.height())
}

/// Nó da fila de prioridade (min-heap via `Reverse` manual em `Ord`).
#[derive(PartialEq, Eq)]
struct Node {
    f: u32,
    cell: Cell,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // menor f primeiro (heap é max → invertemos); tie-break estável por célula.
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.cell.cmp(&self.cell))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* de `start` a `goal`. Retorna o caminho (incluindo start e goal) ou `None` se não há.
#[must_use]
pub fn find_path(map: &Walkmap, start: Cell, goal: Cell) -> Option<Vec<Cell>> {
    if start == goal {
        return Some(vec![start]);
    }
    if map.get(goal.0, goal.1).map(|c| c.walkable) != Some(true) {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<Cell, Cell> = HashMap::new();
    let mut g_score: HashMap<Cell, u32> = HashMap::new();

    g_score.insert(start, 0);
    open.push(Node { f: heuristic(start, goal), cell: start });

    while let Some(Node { cell, .. }) = open.pop() {
        if cell == goal {
            return Some(reconstruct(&came_from, goal));
        }
        let current_g = *g_score.get(&cell).unwrap_or(&u32::MAX);
        for next in neighbors(map, cell) {
            if !map.passable(cell, next) {
                continue;
            }
            let tentative = current_g.saturating_add(1);
            if tentative < *g_score.get(&next).unwrap_or(&u32::MAX) {
                came_from.insert(next, cell);
                g_score.insert(next, tentative);
                open.push(Node { f: tentative.saturating_add(heuristic(next, goal)), cell: next });
            }
        }
    }
    None
}

fn reconstruct(came_from: &HashMap<Cell, Cell>, goal: Cell) -> Vec<Cell> {
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
        let Some(path) = find_path(&map, (0, 0), (3, 0)) else {
            panic!("deve haver caminho");
        };
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(3, 0)));
        assert_eq!(path.len(), 4); // 0,1,2,3 em x
    }

    #[test]
    fn contorna_obstaculo() {
        // parede vertical em x=2, y=0..2 (não caminhável); precisa desviar por y=3.
        let mut map = Walkmap::flat(5, 5);
        for y in 0..3 {
            map.set(2, y, SubCell { height: 0, walkable: false });
        }
        let Some(path) = find_path(&map, (0, 0), (4, 0)) else {
            panic!("deve contornar o obstaculo");
        };
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(4, 0)));
        // nenhum passo passa pela parede
        assert!(path.iter().all(|&(x, y)| !(x == 2 && y < 3)));
    }

    #[test]
    fn sem_caminho_quando_cercado() {
        // flat(5,5) => 20x20 sub-células; cerca o goal (4,4) pelos 4 vizinhos ortogonais.
        let mut map = Walkmap::flat(5, 5);
        for (x, y) in [(3, 4), (5, 4), (4, 3), (4, 5)] {
            map.set(x, y, SubCell { height: 0, walkable: false });
        }
        assert!(find_path(&map, (0, 0), (4, 4)).is_none());
    }
}
