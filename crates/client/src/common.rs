//! Tipos e constantes compartilhados por ≥2 feature-plugins do cliente.

use bevy::prelude::*;
use shared::PlayerId;

/// Altura (em unidades de mundo = tiles) de 1 nível de elevação do terreno.
pub(crate) const LEVEL_HEIGHT: f32 = 1.0;
/// Meia-altura do billboard do personagem.
pub(crate) const BILLBOARD_HALF: f32 = 0.7;
/// Largura do billboard (sprite do herói 64×64 quadrado → largura = altura).
pub(crate) const BILLBOARD_W: f32 = 1.4;

/// Player controlado por este cliente (recebido do servidor via `AssignedPlayer`).
#[derive(Resource, Default)]
pub(crate) struct MyPlayer(pub(crate) Option<PlayerId>);

/// Sprite que sempre encara a câmera (personagens/NPCs).
#[derive(Component)]
pub(crate) struct Billboard;

/// Meia-altura do sprite deste billboard (pivô nos pés). Permite sprites de tamanhos
/// diferentes (herói retrato vs NPC quadrado) sem hardcode em `sync_transforms`.
#[derive(Component)]
pub(crate) struct SpriteHalf(pub(crate) f32);

/// Célula (tile) de uma peça de cenário — usada depois pela visão/cutaway (M2).
#[derive(Component, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct Cell {
    pub(crate) x: u32,
    pub(crate) y: u32,
}

/// Piso suspenso da casa (2º andar ou telhado), no `level` (altura) dado. Some (vira
/// "teto") quando o próprio player está dentro e ABAIXO dele; aparece no nível dele ou acima.
#[derive(Component)]
pub(crate) struct Platform {
    pub(crate) level: i32,
}

/// Parede da CASA (box 3D). Usada pelo cutaway: some a do lado da câmera quando se entra.
#[derive(Component)]
pub(crate) struct HouseWall;

/// Par de materiais (claro/escuro) de uma peça de cenário. O FOV troca entre os dois para
/// ESCURECER o que o boneco não enxerga (em vez de esconder).
#[derive(Component)]
pub(crate) struct DimPair {
    pub(crate) bright: Handle<StandardMaterial>,
    pub(crate) dim: Handle<StandardMaterial>,
}
