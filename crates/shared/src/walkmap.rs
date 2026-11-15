//! Grade de células (tiles) com **elevação real** e caminhabilidade (D1.1/D2.3).
//!
//! As posições do mundo estão em **UNIDADES DE TILE** (1 tile = 1.0). Isso casa **1:1**
//! com a grade do tilemap isométrico no cliente — clicar um tile leva o personagem ao
//! **centro exato** daquele tile (sem offset). O resolver de passo só deixa andar para
//! uma célula caminhável cujo degrau de altura seja ≤ [`STEP_MAX`].

use bevy::prelude::*;

/// Degrau máximo transponível, em níveis de altura (D2.3).
pub const STEP_MAX: i32 = 1;

/// Uma célula da grade: altura (em níveis) e se é caminhável.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubCell {
    pub height: i32,
    pub walkable: bool,
}

impl Default for SubCell {
    fn default() -> Self {
        Self { height: 0, walkable: true }
    }
}

/// Grade de células cobrindo a zona. Posições do mundo em unidades de tile.
#[derive(Resource, Debug, Clone)]
pub struct Walkmap {
    width: u32,
    height: u32,
    cells: Vec<SubCell>,
}

impl Walkmap {
    /// Grade `width × height` tiles, plana e toda caminhável.
    #[must_use]
    pub fn flat(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cells: vec![SubCell::default(); (width * height) as usize],
        }
    }

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x < self.width && y < self.height {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<SubCell> {
        self.index(x, y).map(|i| self.cells[i])
    }

    pub fn set(&mut self, x: u32, y: u32, cell: SubCell) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = cell;
        }
    }

    /// Posição do mundo (unidades de tile) → célula. Convenção do tilemap: o índice
    /// INTEIRO é o CENTRO do tile, então arredondamos (o tile `t` cobre `[t-0.5, t+0.5)`).
    #[must_use]
    pub fn cell_at_world(&self, pos: Vec2) -> Option<(u32, u32)> {
        let rx = pos.x.round();
        let ry = pos.y.round();
        if rx < 0.0 || ry < 0.0 {
            return None;
        }
        let x = rx as u32;
        let y = ry as u32;
        if x < self.width && y < self.height {
            Some((x, y))
        } else {
            None
        }
    }

    /// Pode sair de `from` para `to`? (destino caminhável e degrau ≤ STEP_MAX)
    fn can_enter(&self, from: (u32, u32), to: (u32, u32)) -> bool {
        let (Some(f), Some(t)) = (self.get(from.0, from.1), self.get(to.0, to.1)) else {
            return false;
        };
        t.walkable && (t.height - f.height).abs() <= STEP_MAX
    }

    /// Versão pública da regra de passo (usada pelo A* — P5).
    #[must_use]
    pub fn passable(&self, from: (u32, u32), to: (u32, u32)) -> bool {
        self.can_enter(from, to)
    }
}

/// Centro no mundo de uma célula. Convenção do tilemap: o índice inteiro JÁ é o centro.
#[must_use]
pub fn cell_center(cell: (u32, u32)) -> Vec2 {
    Vec2::new(cell.0 as f32, cell.1 as f32)
}

/// Resolve um passo de `from` para `to` (unidades de tile) contra a grade: tenta o
/// movimento cheio; se bloqueado, desliza por um eixo; senão fica em `from`.
#[must_use]
pub fn resolve_move(map: &Walkmap, from: Vec2, to: Vec2) -> Vec2 {
    let Some(from_cell) = map.cell_at_world(from) else {
        return from;
    };
    let allows = |target: Vec2| {
        map.cell_at_world(target)
            .is_some_and(|to_cell| map.can_enter(from_cell, to_cell))
    };
    if allows(to) {
        return to;
    }
    let slide_x = Vec2::new(to.x, from.y);
    if allows(slide_x) {
        return slide_x;
    }
    let slide_y = Vec2::new(from.x, to.y);
    if allows(slide_y) {
        return slide_y;
    }
    from
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plano_permite_andar() {
        let map = Walkmap::flat(8, 8);
        let (from, to) = (cell_center((4, 4)), cell_center((5, 4)));
        assert_eq!(resolve_move(&map, from, to), to);
    }

    #[test]
    fn degrau_de_1_sobe() {
        let mut map = Walkmap::flat(8, 8);
        map.set(5, 4, SubCell { height: 1, walkable: true });
        let (from, to) = (cell_center((4, 4)), cell_center((5, 4)));
        assert_eq!(resolve_move(&map, from, to), to);
    }

    #[test]
    fn degrau_de_2_bloqueia() {
        let mut map = Walkmap::flat(8, 8);
        map.set(5, 4, SubCell { height: 2, walkable: true });
        let (from, to) = (cell_center((4, 4)), cell_center((5, 4)));
        assert_eq!(resolve_move(&map, from, to), from);
    }

    #[test]
    fn nao_caminhavel_bloqueia() {
        let mut map = Walkmap::flat(8, 8);
        map.set(5, 4, SubCell { height: 0, walkable: false });
        let (from, to) = (cell_center((4, 4)), cell_center((5, 4)));
        assert_eq!(resolve_move(&map, from, to), from);
    }

    #[test]
    fn fora_do_mapa_bloqueia() {
        let map = Walkmap::flat(8, 8);
        let from = cell_center((0, 0));
        let to = Vec2::new(-0.5, from.y);
        assert_eq!(resolve_move(&map, from, to), from);
    }

    #[test]
    fn desliza_no_eixo_livre() {
        let mut map = Walkmap::flat(8, 8);
        map.set(5, 5, SubCell { height: 5, walkable: true }); // diagonal
        map.set(5, 4, SubCell { height: 5, walkable: true }); // eixo x
        let (from, to) = (cell_center((4, 4)), cell_center((5, 5)));
        assert_eq!(resolve_move(&map, from, to), Vec2::new(from.x, to.y));
    }
}
