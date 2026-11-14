//! Cliente desktop do ragnadofus (esqueleto do Passo 1).
//!
//! Prova que `client → shared` compila e que o renderer do Bevy está acoplado ao
//! cliente (`bevy_render` no fecho de dependências — verificado pelo gate). A zona 2.5D,
//! a câmera isométrica e o personagem andando local entram no Passo 2.

use bevy::prelude::*;
use shared::SharedPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SharedPlugin)
        .run();
}
