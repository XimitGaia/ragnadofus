//! Cliente desktop do ragnadofus — render **3D** (câmera estilo Ragnarok).
//!
//! Conecta a um servidor autoritativo (`replicon` + `renet2`/UDP), envia o input como
//! `MoveTo` (só a célula destino, clique-pra-mover) e **renderiza as entidades replicadas**
//! num mundo 3D: chão/paredes como malhas, personagens como **billboards** (sprites que
//! sempre encaram a câmera). A câmera **orbita** o próprio personagem e gira livre (Q/E),
//! com tilt fixo estilo isométrico-3D — como o RO renderiza o mapa.
//!
//! Servidor via env `RAGNADOFUS_SERVER` (ex.: `127.0.0.1:5000`; default localhost:5000).
//! Para o teste multiplayer: suba o `server` e abra 2+ instâncias deste `client`.
//!
//! Organização **feature-plugin**: cada módulo expõe `pub(crate) fn plugin(app)` (ou um
//! `Plugin`) e cuida de uma feature isolada; `app.rs` (`AppPlugin`) agrega tudo e ordena os
//! sistemas. Mexer só no desenho/andar → `sprites.rs`; câmera → `camera.rs`; visão →
//! `vision.rs`; input → `input.rs`; mundo → `world.rs`; rede → `net.rs`; UI → `ui.rs`.

use bevy::prelude::*;

mod app;
mod camera;
mod common;
mod input;
mod net;
mod sprites;
mod ui;
mod vision;
mod world;

fn main() -> anyhow::Result<()> {
    App::new().add_plugins(app::AppPlugin).run();
    Ok(())
}
