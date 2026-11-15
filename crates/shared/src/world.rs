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
    map
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
