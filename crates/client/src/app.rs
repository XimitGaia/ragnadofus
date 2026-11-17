//! Plugin raiz do cliente: monta o `App` (render 3D curado + netcode + UI), agrega os
//! feature-plugins e define a ORDEM determinística dos sistemas de `Update`.

use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RepliconRenetPlugins;
use shared::ProtocolPlugin;

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

/// Plugin raiz: configura `DefaultPlugins` curado, recursos globais, os feature-plugins
/// (`net`, `world`, `camera`, `sprites`, `ui`, `vision`, `input`) e a ordenação dos sets.
pub(crate) struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
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
            crate::net::plugin,
            crate::world::plugin,
            crate::camera::plugin,
            crate::sprites::plugin,
            crate::ui::plugin,
            crate::vision::plugin,
            crate::input::plugin,
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
    }
}
