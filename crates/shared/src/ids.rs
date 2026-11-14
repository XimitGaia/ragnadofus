//! Newtypes estáveis para rede/persistência. Nunca usar `Entity` cru fora do processo
//! (os índices de `Entity` não são estáveis entre cliente/servidor nem entre sessões).

use serde::{Deserialize, Serialize};

/// Identificador estável de um jogador, válido através da rede e da persistência.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

/// Tick de simulação (contador do `FixedUpdate` do servidor). Base para reprodutibilidade
/// e futuro rollback (DT-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tick(pub u64);

impl Tick {
    /// Próximo tick.
    #[must_use]
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
