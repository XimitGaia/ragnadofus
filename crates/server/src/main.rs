//! Servidor headless do ragnadofus (esqueleto do Passo 1).
//!
//! Bevy com `MinimalPlugins` — **sem renderer**. A autoridade do servidor (mover,
//! colidir, A* do NPC, replicação via replicon) entra nos Passos 4–5. Aqui só provamos
//! que o binário do servidor compila e sobe sobre o mesmo `shared`/ECS do cliente, sem
//! arrastar `bevy_render`.

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use shared::SharedPlugin;
use std::time::Duration;

fn main() {
    // ~64 Hz é o alvo do loop de simulação (RFC 0005). O tick de rede (20–30 Hz) será
    // distinto e entra com o replicon.
    let tick_period = Duration::from_secs_f64(1.0 / 64.0);

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(tick_period)))
        .add_plugins(SharedPlugin)
        .run();
}
