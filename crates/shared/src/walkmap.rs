//! Walkmap sub-tile 4×4 com **elevação real** (D1.1/D2.2/D2.3). Modelo de dados +
//! resolver de movimento cinemático determinístico: só se anda para uma sub-célula
//! caminhável cujo **degrau** de altura em relação à atual seja ≤ [`STEP_MAX`] (sobe 1,
//! acima bloqueia). Sem render, sem rede — roda igual no cliente e no servidor.
//!
//! A física contínua do `avian2d` (colisão dinâmica contra geometria) entra como camada
//! seguinte; o degrau/elevação em si é regra de walkmap, implementada e testada aqui.

use bevy::prelude::*;

/// Sub-células por tile, por eixo (D2.2).
pub const SUBCELLS_PER_TILE: u32 = 4;
/// Lado de um tile no mundo (px). Sub-célula = `TILE_WORLD_SIZE / SUBCELLS_PER_TILE`.
pub const TILE_WORLD_SIZE: f32 = 32.0;
/// Lado de uma sub-célula no mundo (px).
pub const SUBCELL_SIZE: f32 = TILE_WORLD_SIZE / SUBCELLS_PER_TILE as f32;
/// Degrau máximo transponível, em níveis de altura (D2.3).
pub const STEP_MAX: i32 = 1;

/// Uma sub-célula da walkmap: altura (em níveis) e se é caminhável.
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

/// Grade de sub-células cobrindo a zona. Origem (sub-célula 0,0) no mundo (0,0).
#[derive(Resource, Debug, Clone)]
pub struct Walkmap {
    width: u32,
    height: u32,
    cells: Vec<SubCell>,
}

impl Walkmap {
    /// Mapa plano e todo caminhável, com `tiles_x × tiles_y` tiles.
    #[must_use]
    pub fn flat(tiles_x: u32, tiles_y: u32) -> Self {
        let width = tiles_x * SUBCELLS_PER_TILE;
        let height = tiles_y * SUBCELLS_PER_TILE;
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

    /// Sub-célula (sx, sy) → índice linear, se dentro dos limites.
    fn index(&self, sx: u32, sy: u32) -> Option<usize> {
        if sx < self.width && sy < self.height {
            Some((sy * self.width + sx) as usize)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get(&self, sx: u32, sy: u32) -> Option<SubCell> {
        self.index(sx, sy).map(|i| self.cells[i])
    }

    /// Define uma sub-célula (ignora se fora dos limites).
    pub fn set(&mut self, sx: u32, sy: u32, cell: SubCell) {
        if let Some(i) = self.index(sx, sy) {
            self.cells[i] = cell;
        }
    }

    /// Posição no mundo → coordenadas de sub-célula (None se fora do mapa / negativa).
    #[must_use]
    pub fn subcell_at_world(&self, pos: Vec2) -> Option<(u32, u32)> {
        if pos.x < 0.0 || pos.y < 0.0 {
            return None;
        }
        let sx = (pos.x / SUBCELL_SIZE) as u32;
        let sy = (pos.y / SUBCELL_SIZE) as u32;
        if sx < self.width && sy < self.height {
            Some((sx, sy))
        } else {
            None
        }
    }

    /// Pode sair da sub-célula de origem para o destino? (destino caminhável e degrau ≤ STEP_MAX)
    fn can_enter(&self, from: (u32, u32), to: (u32, u32)) -> bool {
        let (Some(from_cell), Some(to_cell)) = (self.get(from.0, from.1), self.get(to.0, to.1))
        else {
            return false;
        };
        to_cell.walkable && (to_cell.height - from_cell.height).abs() <= STEP_MAX
    }

    /// Versão pública da regra de passo (usada pelo pathfinding A* — P5).
    #[must_use]
    pub fn passable(&self, from: (u32, u32), to: (u32, u32)) -> bool {
        self.can_enter(from, to)
    }
}

/// Centro no mundo de uma sub-célula (para transformar waypoints de path em alvos).
#[must_use]
pub fn subcell_center(sub: (u32, u32)) -> Vec2 {
    Vec2::new(
        (sub.0 as f32 + 0.5) * SUBCELL_SIZE,
        (sub.1 as f32 + 0.5) * SUBCELL_SIZE,
    )
}

/// Resolve um passo de `from` para `to` contra a walkmap: tenta o movimento cheio;
/// se bloqueado, desliza por um eixo; se nada passa, fica em `from`.
#[must_use]
pub fn resolve_move(map: &Walkmap, from: Vec2, to: Vec2) -> Vec2 {
    let Some(from_sub) = map.subcell_at_world(from) else {
        return from;
    };
    let allows = |target: Vec2| -> bool {
        map.subcell_at_world(target)
            .is_some_and(|to_sub| map.can_enter(from_sub, to_sub))
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

    fn center_of(sx: u32, sy: u32) -> Vec2 {
        Vec2::new(
            (sx as f32 + 0.5) * SUBCELL_SIZE,
            (sy as f32 + 0.5) * SUBCELL_SIZE,
        )
    }

    #[test]
    fn plano_permite_andar() {
        let map = Walkmap::flat(4, 4);
        let from = center_of(2, 2);
        let to = center_of(3, 2);
        assert_eq!(resolve_move(&map, from, to), to);
    }

    #[test]
    fn degrau_de_1_sobe() {
        let mut map = Walkmap::flat(4, 4);
        map.set(3, 2, SubCell { height: 1, walkable: true });
        let from = center_of(2, 2);
        let to = center_of(3, 2);
        assert_eq!(resolve_move(&map, from, to), to);
    }

    #[test]
    fn degrau_de_2_bloqueia() {
        let mut map = Walkmap::flat(4, 4);
        map.set(3, 2, SubCell { height: 2, walkable: true });
        let from = center_of(2, 2);
        let to = center_of(3, 2);
        // bloqueado no eixo x; sem alternativa no mesmo movimento => fica em `from`.
        assert_eq!(resolve_move(&map, from, to), from);
    }

    #[test]
    fn nao_caminhavel_bloqueia() {
        let mut map = Walkmap::flat(4, 4);
        map.set(3, 2, SubCell { height: 0, walkable: false });
        let from = center_of(2, 2);
        let to = center_of(3, 2);
        assert_eq!(resolve_move(&map, from, to), from);
    }

    #[test]
    fn fora_do_mapa_bloqueia() {
        let map = Walkmap::flat(4, 4);
        let from = center_of(0, 0);
        let to = Vec2::new(-5.0, from.y);
        assert_eq!(resolve_move(&map, from, to), from);
    }

    #[test]
    fn desliza_no_eixo_livre() {
        // destino diagonal bloqueado em x (parede alta), mas livre em y.
        let mut map = Walkmap::flat(4, 4);
        map.set(3, 3, SubCell { height: 5, walkable: true }); // diagonal: parede
        map.set(3, 2, SubCell { height: 5, walkable: true }); // x: parede
        // y (2,3) continua plano => deslizar em y deve passar.
        let from = center_of(2, 2);
        let to = center_of(3, 3);
        assert_eq!(resolve_move(&map, from, to), Vec2::new(from.x, to.y));
    }
}
