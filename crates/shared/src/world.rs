//! Zona como RECEITA, não bytes (D4.3 / P6): guardamos `(seed, dimensões)` e
//! regeneramos a walkmap deterministicamente. A persistência fica atrás de um trait
//! [`WorldRepository`] (backend trocável — Postgres, memória, etc.).

use crate::walkmap::{SubCell, Walkmap};
use bevy::prelude::{Event, Resource};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Receita reprodutível da zona. Persistida; a geometria é derivada dela. Também é
/// enviada ao cliente (server event) para ele reconstruir a walkmap e desenhar a zona.
#[derive(Resource, Event, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneSeed {
    pub seed: u64,
    pub tiles_x: u32,
    pub tiles_y: u32,
}

impl Default for ZoneSeed {
    fn default() -> Self {
        Self { seed: 0x5EED_0001, tiles_x: 64, tiles_y: 64 }
    }
}

/// Erro do backend de persistência (thiserror em `shared` — padroes §1).
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("falha no backend de persistência: {0}")]
    Backend(String),
}

/// Fonte de verdade da zona — backend trocável (D4.3). Na demo só precisa da receita.
pub trait WorldRepository: Send + Sync {
    /// Lê a receita da zona; se não existir, inicializa (persistindo o default) e retorna.
    fn load_or_init_seed(&self) -> Result<ZoneSeed, RepoError>;
}

/// Regenera a walkmap a partir da receita — **determinístico** (mesma seed → mesma
/// walkmap). Parede vertical com uma passagem cuja posição deriva da seed.
#[must_use]
pub fn build_walkmap(zone: &ZoneSeed) -> Walkmap {
    let mut map = Walkmap::flat(zone.tiles_x, zone.tiles_y);
    let sub_w = zone.tiles_x;
    let sub_h = zone.tiles_y;
    if sub_w < 2 || sub_h < 1 {
        return map;
    }
    let wall_x = (zone.seed % u64::from(sub_w - 1)) as u32;
    let gap_y = (zone.seed / 7 % u64::from(sub_h)) as u32;
    for y in 0..sub_h {
        if y == gap_y || y == gap_y.saturating_add(1) {
            continue; // passagem na parede
        }
        map.set(wall_x, y, SubCell { height: 0, walkable: false });
    }

    build_house(&mut map);
    build_house_floors(&mut map);
    build_plateau(&mut map);
    map
}

/// Níveis da casa (multi-nível 2.5D): térreo (0), 2º andar e telhado, ambos caminháveis.
pub const FLOOR_LEVEL: i32 = 3;
pub const ROOF_LEVEL: i32 = 6;
/// Colunas (x) das duas escadas: A (térreo→2º andar, lado leste) e B (2º andar→telhado,
/// lado oeste). Cada escada é uma rampa de superfícies de +1 (≤ STEP_MAX).
pub const STAIR_X: u32 = HOUSE_X1 - 2;
pub const STAIR_X2: u32 = HOUSE_X0 + 2;

fn build_house_floors(map: &mut Walkmap) {
    if HOUSE_X1 >= map.width() || HOUSE_Y1 >= map.height() {
        return;
    }
    for x in HOUSE_X0..=HOUSE_X1 {
        for y in HOUSE_Y0..=HOUSE_Y1 {
            map.add_surface(x, y, ROOF_LEVEL); // telhado caminhável sobre tudo
            let interior = x > HOUSE_X0 && x < HOUSE_X1 && y > HOUSE_Y0 && y < HOUSE_Y1;
            if interior {
                map.add_surface(x, y, FLOOR_LEVEL); // piso do 2º andar sobre o interior
            }
        }
    }
    // Escada A (térreo 0 → 2º andar 3): superfícies 1,2 (o 3 já é o 2º andar na célula topo).
    map.add_surface(STAIR_X, HOUSE_Y1 - 2, 1);
    map.add_surface(STAIR_X, HOUSE_Y1 - 3, 2);
    // Escada B (2º andar 3 → telhado 6): superfícies 4,5 (o 6 já é o telhado na célula topo).
    map.add_surface(STAIR_X2, HOUSE_Y0 + 4, 4);
    map.add_surface(STAIR_X2, HOUSE_Y0 + 5, 5);
}

/// Platô (altura 2) com uma RAMPA (altura 1) numa borda — a rampa liga o chão (0) ao
/// platô (2) em degraus de 1 (caminhável); as outras bordas são penhasco 0→2 (degrau 2,
/// bloqueado). Exercita a elevação real (D1.1/D2.3): só sobe pela rampa.
fn build_plateau(map: &mut Walkmap) {
    const X0: u32 = 38;
    const X1: u32 = 46;
    const Y0: u32 = 22;
    const Y1: u32 = 30;
    if X1 >= map.width() || Y1 + 1 >= map.height() {
        return;
    }
    for x in X0..=X1 {
        for y in Y0..=Y1 {
            if let Some(mut cell) = map.get(x, y) {
                cell.height = 2;
                map.set(x, y, cell);
            }
        }
    }
    // Rampa (altura 1) na borda sul (y = Y1+1): chão(0) → rampa(1) → platô(2).
    let ramp_y = Y1 + 1;
    for x in X0..=X1 {
        if let Some(mut cell) = map.get(x, ramp_y) {
            cell.height = 1;
            map.set(x, ramp_y, cell);
        }
    }
}

/// Footprint da casa, em tiles: retângulo `(X0,Y0)`–`(X1,Y1)`; porta na parede de cima.
pub const HOUSE_X0: u32 = 20;
pub const HOUSE_Y0: u32 = 34;
pub const HOUSE_X1: u32 = 28;
pub const HOUSE_Y1: u32 = 42;
/// Coluna do vão da porta (2 tiles), na parede de cima (`HOUSE_Y0`).
pub const HOUSE_DOOR_X: u32 = 24;

/// Casa: retângulo de paredes (não-caminháveis) com um **vão de porta** por onde se
/// entra. Interior caminhável. Como o cliente desenha pedra onde não é caminhável, a
/// casa aparece sozinha; o telhado (que some ao entrar, D1.4) é desenhado pelo cliente.
fn build_house(map: &mut Walkmap) {
    if HOUSE_X1 >= map.width() || HOUSE_Y1 >= map.height() {
        return;
    }
    for x in HOUSE_X0..=HOUSE_X1 {
        for y in HOUSE_Y0..=HOUSE_Y1 {
            let perimeter =
                x == HOUSE_X0 || x == HOUSE_X1 || y == HOUSE_Y0 || y == HOUSE_Y1;
            let door = y == HOUSE_Y0 && (x == HOUSE_DOOR_X || x == HOUSE_DOOR_X + 1);
            if perimeter && !door {
                map.set(x, y, SubCell { height: 0, walkable: false });
            }
        }
    }
}

/// Backend em memória (default/testes e fallback quando não há Postgres).
pub struct InMemoryRepository {
    seed: ZoneSeed,
}

impl InMemoryRepository {
    #[must_use]
    pub fn new(seed: ZoneSeed) -> Self {
        Self { seed }
    }
}

impl Default for InMemoryRepository {
    fn default() -> Self {
        Self::new(ZoneSeed::default())
    }
}

impl WorldRepository for InMemoryRepository {
    fn load_or_init_seed(&self) -> Result<ZoneSeed, RepoError> {
        Ok(self.seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walkable_grid(map: &Walkmap) -> Vec<bool> {
        let mut v = Vec::new();
        for y in 0..map.height() {
            for x in 0..map.width() {
                v.push(map.get(x, y).map(|c| c.walkable).unwrap_or(false));
            }
        }
        v
    }

    #[test]
    fn walkmap_e_deterministica_por_seed() {
        let zone = ZoneSeed { seed: 123, tiles_x: 8, tiles_y: 8 };
        assert_eq!(walkable_grid(&build_walkmap(&zone)), walkable_grid(&build_walkmap(&zone)));
    }

    #[test]
    fn seeds_diferentes_mudam_a_zona() {
        let a = build_walkmap(&ZoneSeed { seed: 1, tiles_x: 16, tiles_y: 16 });
        let b = build_walkmap(&ZoneSeed { seed: 999, tiles_x: 16, tiles_y: 16 });
        assert_ne!(walkable_grid(&a), walkable_grid(&b));
    }

    #[test]
    fn in_memory_round_trip() {
        let repo = InMemoryRepository::new(ZoneSeed { seed: 42, tiles_x: 64, tiles_y: 64 });
        match repo.load_or_init_seed() {
            Ok(s) => assert_eq!(s.seed, 42),
            Err(e) => panic!("{e}"),
        }
    }
}
