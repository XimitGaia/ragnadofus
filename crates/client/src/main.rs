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

use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RepliconRenetPlugins;
use shared::ProtocolPlugin;

mod camera;
mod common;
mod input;
mod net;
mod sprites;
mod ui;
mod vision;
mod world;

/// Ordem determinística dos sistemas de `Update` do cliente — substitui o `.chain()` único
/// de antes. A sequência preserva EXATAMENTE as dependências originais: `Spawn` antes de
/// `Sync` (entidade nova já com `Transform`), `Sync` antes de `Anim` (anim/hop compõem POR
/// CIMA do `Transform`), `Camera` depois de `Anim`, `Vision` depois de `Camera`
/// (`apply_vision` lê o `Transform` da câmera).
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ClientSet {
    Input,
    Spawn,
    Sync,
    Anim,
    Camera,
    Vision,
}

fn main() -> anyhow::Result<()> {
    let mut app = App::new();
    // Atualiza o app continuamente mesmo SEM foco da janela — senão o loop throttla, o
    // cliente para de mandar keepalive do netcode e a conexão cai (timeout).
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .insert_resource(ClearColor(Color::srgb(0.45, 0.65, 0.85))) // céu
    .add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            })
            // Sem VSync: o render 3D travava bloqueado no present (swapchain) de uma janela
            // sem compositor ativo (ex.: segundo plano). AutoNoVsync não espera o vblank.
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins((RepliconPlugins, RepliconRenetPlugins))
    .add_plugins(ProtocolPlugin)
    .add_plugins((
        net::plugin,
        world::plugin,
        camera::plugin,
        sprites::plugin,
        ui::plugin,
        vision::plugin,
        input::plugin,
    ))
    .configure_sets(
        Update,
        (
            ClientSet::Input,
            ClientSet::Spawn,
            ClientSet::Sync,
            ClientSet::Anim,
            ClientSet::Camera,
            ClientSet::Vision,
        )
            .chain(),
    );

    app.run();
    Ok(())
}
