//! Construção do mundo 3D — câmera/luz + malhas de chão, paredes, telhado, escadas e props
//! (feature-plugin).

use bevy::camera::ScalingMode;
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use shared::world::{
    is_stair_a, is_stair_b, FLOOR_LEVEL, HOUSE_X0, HOUSE_X1, HOUSE_Y0, HOUSE_Y1, ROOF_LEVEL,
    STAIR_X, STAIR_X2,
};
use shared::{build_walkmap, ZoneSeed};

use crate::common::{Billboard, Cell, DimPair, HouseWall, Platform, BILLBOARD_HALF, BILLBOARD_W, LEVEL_HEIGHT};
use crate::sprites::billboard_material;
use crate::vision::in_house;

/// Altura da parede da casa, em tiles — 2 andares (~3 por andar).
const WALL_LEVELS: f32 = 6.0;
/// Altura de 1 andar (onde fica a faixa divisória entre os pisos).
const FLOOR_HEIGHT: f32 = 3.0;

/// Registra a cena (câmera/luz) e a construção do mundo quando a zona chega.
pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<ZoneBuilt>()
        .add_systems(Startup, setup_scene)
        .add_observer(on_zone_received);
}

/// Evita redesenhar a zona se a `ZoneSeed` chegar mais de uma vez.
#[derive(Resource, Default)]
struct ZoneBuilt(bool);

/// Escurece uma cor (versão "na penumbra" — fora do campo de visão).
fn dim_color(c: Color) -> Color {
    let s = c.to_srgba();
    Color::srgb(s.red * 0.5, s.green * 0.5, s.blue * 0.5)
}

/// Câmera 3D (perspectiva) + uma luz direcional. A posição da câmera é ajustada a cada
/// frame por `camera_orbit` (segue e orbita o próprio personagem).
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // ORTOGRÁFICA: sem perspectiva (chão não "foge" pro horizonte) — o look achatado
        // do RO. Tamanho controlado por `viewport_height` (atualizado por `camera_orbit`).
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: 14.0 },
            ..OrthographicProjection::default_3d()
        }),
        // sem LUTs de tonemapping no build curado (DT-03): usa None (e sem dither, que
        // depende do tonemapping — evita o warning).
        Tonemapping::None,
        DebandDither::Disabled,
        Transform::from_xyz(32.0, 20.0, 50.0).looking_at(Vec3::new(32.0, 0.0, 32.0), Vec3::Y),
        AmbientLight { brightness: 600.0, ..default() },
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, ..default() },
        Transform::from_xyz(60.0, 100.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Recebe a receita da zona e constrói o mundo 3D: 1 quad de chão por tile (xadrez
/// grama/terra, elevado pela altura da célula) e uma CAIXA de parede onde não é caminhável.
fn on_zone_received(
    trigger: On<ZoneSeed>,
    mut built: ResMut<ZoneBuilt>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    if built.0 {
        return;
    }
    let zone: ZoneSeed = *trigger.event();
    let map = build_walkmap(&zone);

    let tile_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    let wall_mesh = meshes.add(Cuboid::new(1.0, WALL_LEVELS, 1.0));
    // Material opaco (claro) + helper para gerar o par claro/escuro de uma cor.
    let lit = |c: Color| StandardMaterial { base_color: c, perceptual_roughness: 1.0, ..default() };
    let mut pair = |c: Color| DimPair {
        bright: materials.add(lit(c)),
        dim: materials.add(lit(dim_color(c))),
    };
    let grass = pair(Color::srgb(0.30, 0.55, 0.25));
    let dirt = pair(Color::srgb(0.45, 0.35, 0.22));
    let wall = pair(Color::srgb(0.52, 0.52, 0.58));
    let band = pair(Color::srgb(0.40, 0.26, 0.16));
    let stair = pair(Color::srgb(0.50, 0.38, 0.24));
    // Árvores (billboard): claro = textura cheia; escuro = textura com tinta escura.
    let tree_h = BILLBOARD_HALF * 2.0 * 1.4;
    let tree_mesh = meshes.add(Rectangle::new(BILLBOARD_W * 1.4, tree_h));
    let tree_tex: Handle<Image> = asset_server.load("tree.png");
    let tree = DimPair {
        bright: materials.add(billboard_material(Color::WHITE, tree_tex.clone())),
        dim: materials.add(billboard_material(Color::srgb(0.35, 0.35, 0.35), tree_tex)),
    };
    // Telhado da casa (material; desenhado por célula mais abaixo, com vão pra escada B).
    let roof_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.28, 0.24),
        perceptual_roughness: 1.0,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    // Faixa (moldura) divisória entre o 1º e o 2º andar da casa.
    let band_mesh = meshes.add(Cuboid::new(1.12, 0.45, 1.12));

    for y in 0..zone.tiles_y {
        for x in 0..zone.tiles_x {
            let cell = map.get(x, y);
            let walkable = cell.map(|c| c.walkable).unwrap_or(false);
            let h = cell.map(|c| c.height).unwrap_or(0);
            let base_y = h as f32 * LEVEL_HEIGHT;

            let gd = if (x + y) % 2 == 0 { &grass } else { &dirt };
            commands.spawn((
                Mesh3d(tile_mesh.clone()),
                MeshMaterial3d(gd.bright.clone()),
                Transform::from_xyz(x as f32, base_y, y as f32),
                Cell { x, y },
                DimPair { bright: gd.bright.clone(), dim: gd.dim.clone() },
            ));

            if !walkable {
                // parede: caixa de altura WALL_LEVELS apoiada no chão da célula.
                let mut wall_e = commands.spawn((
                    Mesh3d(wall_mesh.clone()),
                    MeshMaterial3d(wall.bright.clone()),
                    Transform::from_xyz(x as f32, base_y + WALL_LEVELS / 2.0, y as f32),
                    Cell { x, y },
                    DimPair { bright: wall.bright.clone(), dim: wall.dim.clone() },
                ));
                if in_house(x, y) {
                    wall_e.insert(HouseWall); // parede da casa -> cutaway ao entrar
                    // faixa divisória entre os 2 andares, nesta coluna de parede.
                    commands.spawn((
                        Mesh3d(band_mesh.clone()),
                        MeshMaterial3d(band.bright.clone()),
                        Transform::from_xyz(x as f32, base_y + FLOOR_HEIGHT, y as f32),
                        Cell { x, y },
                        HouseWall,
                        DimPair { bright: band.bright.clone(), dim: band.dim.clone() },
                    ));
                }
            } else if h >= 2 && x % 3 == 0 && y % 3 == 0 {
                // morro (platô): salpica árvores para dar cara de colina arborizada.
                commands.spawn((
                    Mesh3d(tree_mesh.clone()),
                    MeshMaterial3d(tree.bright.clone()),
                    Transform::from_xyz(x as f32, base_y + tree_h / 2.0, y as f32),
                    Billboard,
                    Cell { x, y },
                    DimPair { bright: tree.bright.clone(), dim: tree.dim.clone() },
                ));
            }
        }
    }

    // Telhado por CÉLULA sobre o footprint, com VÃO onde a escada B emerge (pra ver e
    // descer a escada quando se está no telhado).
    let roof_tile = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    for y in HOUSE_Y0..=HOUSE_Y1 {
        for x in HOUSE_X0..=HOUSE_X1 {
            if is_stair_b(x, y) {
                continue; // vão da escada B no telhado
            }
            commands.spawn((
                Mesh3d(roof_tile.clone()),
                MeshMaterial3d(roof_mat.clone()),
                Transform::from_xyz(x as f32, ROOF_LEVEL as f32 * LEVEL_HEIGHT + 0.05, y as f32),
                Platform { level: ROOF_LEVEL },
            ));
        }
    }
    // Piso do 2º andar (plataforma), POR CÉLULA sobre o interior — deixando um VÃO
    // (stairwell) nas colunas das escadas, pra ver/usar a escada ao descer/subir.
    let floor2_tile = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    for y in (HOUSE_Y0 + 1)..=(HOUSE_Y1 - 1) {
        for x in (HOUSE_X0 + 1)..=(HOUSE_X1 - 1) {
            if is_stair_a(x, y) || is_stair_b(x, y) {
                continue; // vão da escada (stairwell) — piso aberto aqui
            }
            commands.spawn((
                Mesh3d(floor2_tile.clone()),
                MeshMaterial3d(stair.bright.clone()),
                Transform::from_xyz(x as f32, FLOOR_LEVEL as f32 * LEVEL_HEIGHT + 0.02, y as f32),
                Platform { level: FLOOR_LEVEL },
            ));
        }
    }

    // Escada A (térreo→2º andar), coluna STAIR_X: degraus 1..3 subindo do chão.
    for i in 0..FLOOR_LEVEL as u32 {
        let top = (i + 1) as f32 * LEVEL_HEIGHT;
        let z = HOUSE_Y1 as f32 - 2.0 - i as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, top, 0.95))),
            MeshMaterial3d(stair.bright.clone()),
            Transform::from_xyz(STAIR_X as f32, top / 2.0, z),
            Cell { x: STAIR_X, y: z.round() as u32 },
            DimPair { bright: stair.bright.clone(), dim: stair.dim.clone() },
        ));
    }
    // Escada B (2º andar→telhado), coluna STAIR_X2: degraus 4..6 saindo do piso do 2º andar.
    // É marcada como Platform(2º andar): some quando o boneco está no TÉRREO (abaixo dela),
    // pra não aparecer de baixo; reaparece quando se sobe ao 2º andar.
    for i in 0..FLOOR_LEVEL as u32 {
        let bottom = FLOOR_LEVEL as f32 * LEVEL_HEIGHT;
        let top = (FLOOR_LEVEL + 1 + i as i32) as f32 * LEVEL_HEIGHT;
        let z = HOUSE_Y0 as f32 + 4.0 + i as f32;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, top - bottom, 0.95))),
            MeshMaterial3d(stair.bright.clone()),
            Transform::from_xyz(STAIR_X2 as f32, (bottom + top) / 2.0, z),
            Platform { level: FLOOR_LEVEL },
        ));
    }

    // Props de cenário (billboards) espalhados perto da casa — "montar coisas" no mundo.
    // (tex, x, y, largura, altura). Arte original (crate/barril/arbusto/pedra/placa/flor).
    let props: [(&str, u32, u32, f32, f32); 10] = [
        ("crate.png", 22, 31, 0.9, 0.9),
        ("crate.png", 23, 31, 0.9, 0.9),
        ("crate.png", 22, 30, 0.9, 0.9),
        ("barrel.png", 24, 30, 0.85, 0.95),
        ("sign.png", 26, 31, 0.9, 0.9),
        ("bush.png", 30, 33, 1.0, 0.88),
        ("bush.png", 18, 33, 1.0, 0.88),
        ("rock.png", 31, 29, 1.0, 0.75),
        ("flower.png", 19, 30, 0.9, 0.68),
        ("mushroom.png", 29, 34, 0.55, 0.55),
    ];
    for (file, px, py, pw, ph) in props {
        let tex: Handle<Image> = asset_server.load(file);
        let bright = materials.add(billboard_material(Color::WHITE, tex.clone()));
        let dim = materials.add(billboard_material(Color::srgb(0.5, 0.5, 0.5), tex));
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(pw, ph))),
            MeshMaterial3d(bright.clone()),
            Transform::from_xyz(px as f32, ph / 2.0, py as f32),
            Billboard,
            Cell { x: px, y: py },
            DimPair { bright, dim },
        ));
    }

    commands.insert_resource(map.clone());
    built.0 = true;
    info!("mundo 3D construído: {}x{} (seed {:#x})", zone.tiles_x, zone.tiles_y, zone.seed);
}
