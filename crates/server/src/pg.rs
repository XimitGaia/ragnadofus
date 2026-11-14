//! Backend Postgres do [`WorldRepository`] (P6). Persiste a RECEITA da zona (seed +
//! dimensões) numa tabela; na primeira vez inicializa com o default. Síncrono, chamado
//! só no startup do servidor.
//!
//! `DATABASE_URL` ex.: `postgres://ragnadofus:ragnadofus@127.0.0.1:55432/ragnadofus`.

use postgres::{Client, NoTls};
use shared::{RepoError, WorldRepository, ZoneSeed};

/// Repositório da zona em PostgreSQL.
pub struct PostgresRepository {
    url: String,
}

impl PostgresRepository {
    #[must_use]
    pub fn new(url: String) -> Self {
        Self { url }
    }

    fn connect(&self) -> Result<Client, RepoError> {
        Client::connect(&self.url, NoTls).map_err(|e| RepoError::Backend(e.to_string()))
    }
}

impl WorldRepository for PostgresRepository {
    fn load_or_init_seed(&self) -> Result<ZoneSeed, RepoError> {
        let mut client = self.connect()?;
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS zone (
                     id       INT   PRIMARY KEY,
                     seed     BIGINT NOT NULL,
                     tiles_x  INT    NOT NULL,
                     tiles_y  INT    NOT NULL
                 )",
            )
            .map_err(|e| RepoError::Backend(e.to_string()))?;

        let rows = client
            .query("SELECT seed, tiles_x, tiles_y FROM zone WHERE id = 1", &[])
            .map_err(|e| RepoError::Backend(e.to_string()))?;

        if let Some(row) = rows.first() {
            let seed: i64 = row.get(0);
            let tiles_x: i32 = row.get(1);
            let tiles_y: i32 = row.get(2);
            return Ok(ZoneSeed {
                seed: seed as u64,
                tiles_x: tiles_x as u32,
                tiles_y: tiles_y as u32,
            });
        }

        // Primeira vez: persiste a receita default e a devolve.
        let zone = ZoneSeed::default();
        client
            .execute(
                "INSERT INTO zone (id, seed, tiles_x, tiles_y) VALUES (1, $1, $2, $3)",
                &[&(zone.seed as i64), &(zone.tiles_x as i32), &(zone.tiles_y as i32)],
            )
            .map_err(|e| RepoError::Backend(e.to_string()))?;
        Ok(zone)
    }
}
