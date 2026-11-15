//! Cliente desktop do ragnadofus (Passo 4 — em rede).
//!
//! Conecta a um servidor autoritativo (`replicon` + `renet2`/UDP), envia o input como
//! `MoveCommand` (só intenção) e **renderiza as entidades replicadas** (players) na zona
//! 2.5D isométrica. Não roda a simulação — a autoridade é do servidor.
//!
//! Servidor via env `RAGNADOFUS_SERVER` (ex.: `127.0.0.1:5000`; default localhost:5000).
//! Para o teste multiplayer: suba o `server` e abra 2+ instâncias deste `client`.
//!
//! Câmera estática centrada na zona (seguir o próprio player + clique-pra-mover voltam
//! quando a identidade do próprio player for replicada / com A* no servidor — Passo 5).

use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::FontSize;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_ecs_tilemap::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NativeSocket, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt, RepliconRenetPlugins,
};
use shared::world::{HOUSE_DOOR_X, HOUSE_X0, HOUSE_X1, HOUSE_Y0, HOUSE_Y1};
use shared::{
    build_walkmap, AssignedPlayer, MoveTo, Npc, Player, PlayerId, Position, ProtocolPlugin, Walkmap,
    ZoneSeed,
};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::SystemTime;

const DEFAULT_SERVER: &str = "127.0.0.1:5000";

/// Player controlado por este cliente (recebido do servidor via `AssignedPlayer`).
#[derive(Resource, Default)]
struct MyPlayer(Option<PlayerId>);

/// Evita redesenhar a zona se a `ZoneSeed` chegar mais de uma vez.
#[derive(Resource, Default)]
struct ZoneBuilt(bool);

/// Camada do telhado da casa (some quando o player entra — D1.4).
#[derive(Component)]
struct RoofLayer;

/// Parede da FRENTE (lado da câmera: `x==X0` ou `y==Y0`). Fica transparente quando o
/// próprio player está dentro, pra enxergar o interior (cutaway estilo iso).
#[derive(Component)]
struct NearWall;

/// Célula (tile) de uma peça de cenário desenhada como sprite (parede/terreno). Permite
/// ao sistema de VISÃO saber se a peça está no campo de visão do player e escondê-la.
#[derive(Component, Clone, Copy)]
struct Cell {
    x: u32,
    y: u32,
}

/// Estado atual da visão: `true` = o próprio player está DENTRO de um ambiente fechado.
/// Guardado pra só reprocessar o cenário estático (tiles) na TRANSIÇÃO entrar/sair.
#[derive(Resource)]
struct VisionState {
    inside: bool,
}

impl Default for VisionState {
    fn default() -> Self {
        // começa "indeterminado" (!= qualquer estado real) forçando a 1ª aplicação.
        Self { inside: true }
    }
}

/// Quantos tiles pra fora da porta o player enxerga quando está dentro (visão curta).
const DOOR_VIEW: u32 = 3;

/// Está no footprint (retângulo) da casa?
fn in_house(x: u32, y: u32) -> bool {
    (HOUSE_X0..=HOUSE_X1).contains(&x) && (HOUSE_Y0..=HOUSE_Y1).contains(&y)
}

/// Campo de visão quando o player está DENTRO: todo o interior/paredes da casa + uma
/// faixa curta à frente da porta (para enxergar quem chega/sair).
fn in_vision(x: u32, y: u32) -> bool {
    in_house(x, y)
        || (y < HOUSE_Y0
            && HOUSE_Y0 - y <= DOOR_VIEW
            && x + 1 >= HOUSE_DOOR_X
            && x <= HOUSE_DOOR_X + 2)
}

/// Altura visual (px na tela) da parede da casa — 3 tiles (tile = 32px). As paredes ficam
/// num z baixo, então não "mordem" o boneco mesmo altas.
const WALL_HEIGHT_PX: f32 = 96.0;

/// Altura visual (px na tela) de 1 nível de elevação do terreno (platô/rampa).
const HEIGHT_PER_LEVEL: f32 = 18.0;

fn main() -> anyhow::Result<()> {
    let server_addr: SocketAddr = std::env::var("RAGNADOFUS_SERVER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| DEFAULT_SERVER.parse().unwrap_or(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000)));

    let mut app = App::new();
    // Atualiza o app continuamente mesmo SEM foco da janela — senão o loop throttla, o
    // cliente para de mandar keepalive do netcode e a conexão cai (timeout). Essencial
    // para multiplayer com janela em segundo plano.
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                ..default()
            }),
    )
    .add_plugins(TilemapPlugin)
    .add_plugins((RepliconPlugins, RepliconRenetPlugins))
    .add_plugins(ProtocolPlugin)
    .init_resource::<MyPlayer>()
    .init_resource::<ZoneBuilt>()
    .init_resource::<VisionState>()
    .add_observer(on_assigned_player)
    .add_observer(on_zone_received)
    .add_systems(Startup, (setup_scene, setup_ui))
    .add_systems(
        Update,
        (read_right_click, hydrate_players, hydrate_npcs, sync_transforms, camera_follow).chain(),
    )
    .add_systems(Update, (roof_fade, apply_vision));

    connect(&mut app, server_addr)?;
    info!("cliente ragnadofus conectando a {server_addr}");
    app.run();
    Ok(())
}

/// Cria o `RenetClient` + transporte netcode e os insere como recursos.
fn connect(app: &mut App, server_addr: SocketAddr) -> anyhow::Result<()> {
    let channels = app.world().resource::<RepliconChannels>();
    let client = RenetClient::new(
        ConnectionConfig::from_channels(channels.server_configs(), channels.client_configs()),
        false,
    );

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: shared::protocol::PROTOCOL_ID,
        socket_id: 0,
        server_addr,
        user_data: None,
    };
    let native = NativeSocket::new(socket).map_err(|e| anyhow::anyhow!("socket: {e:?}"))?;
    let transport = NetcodeClientTransport::new(current_time, authentication, native)?;

    app.insert_resource(client);
    app.insert_resource(transport);
    Ok(())
}

/// Só a câmera. A zona é desenhada quando a `ZoneSeed` chega do servidor.
fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Recebe a receita da zona e desenha o tilemap 2.5D: xadrez no chão caminhável, **tile
/// de pedra nas paredes** (reconstruindo a mesma walkmap do servidor a partir da seed).
fn on_zone_received(
    trigger: On<ZoneSeed>,
    mut built: ResMut<ZoneBuilt>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if built.0 {
        return;
    }
    let zone: ZoneSeed = *trigger.event();
    let map = build_walkmap(&zone);
    let texture: Handle<Image> = asset_server.load("tiles.png");
    let map_size = TilemapSize { x: zone.tiles_x, y: zone.tiles_y };
    let mut storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    for y in 0..map_size.y {
        for x in 0..map_size.x {
            let is_wall = map.get(x, y).map(|c| !c.walkable).unwrap_or(false);
            // pedra (índice 2) na parede; xadrez grama/terra no chão.
            let index = if is_wall { 2 } else { (x + y) % 2 };
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    texture_index: TileTextureIndex(index),
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .id();
            storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };
    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size: tile_size.into(),
        size: map_size,
        storage,
        texture: TilemapTexture::Single(texture),
        tile_size,
        map_type: TilemapType::Isometric(IsoCoordSystem::Diamond),
        anchor: TilemapAnchor::Center,
        ..default()
    });

    let (origin, unit_x, unit_y) = iso_basis();

    // Telhado: 1 sprite de losango por célula do footprint, erguido à altura das paredes,
    // ordenado por PROFUNDIDADE (depth_z): quem está NA FRENTE da casa renderiza por cima
    // do telhado; quem está atrás/dentro fica coberto. Some ao entrar (roof_fade).
    for y in HOUSE_Y0..=HOUSE_Y1.min(map_size.y.saturating_sub(1)) {
        for x in HOUSE_X0..=HOUSE_X1.min(map_size.x.saturating_sub(1)) {
            let base = project(Vec2::new(x as f32, y as f32), origin, unit_x, unit_y);
            commands.spawn((
                Sprite {
                    image: asset_server.load("tiles.png"),
                    rect: Some(Rect::new(192.0, 0.0, 256.0, 32.0)), // tile 3 = telhado
                    custom_size: Some(Vec2::new(64.0, 32.0)),
                    ..default()
                },
                Transform::from_xyz(base.x, base.y + WALL_HEIGHT_PX, depth_z(x as f32, y as f32)),
                RoofLayer,
            ));
        }
    }

    // Faces verticais das paredes (dão ALTURA à casa): sprite escuro do chão até o topo,
    // em cada célula de parede. z baixo (não morde o boneco).
    // Centro da casa na TELA: paredes abaixo dele são as da FRENTE (lado da câmera).
    let house_center = project(
        Vec2::new(
            (HOUSE_X0 + HOUSE_X1) as f32 / 2.0,
            (HOUSE_Y0 + HOUSE_Y1) as f32 / 2.0,
        ),
        origin,
        unit_x,
        unit_y,
    );
    for y in HOUSE_Y0..=HOUSE_Y1.min(map_size.y.saturating_sub(1)) {
        for x in HOUSE_X0..=HOUSE_X1.min(map_size.x.saturating_sub(1)) {
            let is_wall = map.get(x, y).map(|c| !c.walkable).unwrap_or(false);
            if !is_wall {
                continue;
            }
            let base = project(Vec2::new(x as f32, y as f32), origin, unit_x, unit_y);
            let mut wall = commands.spawn((
                Sprite {
                    color: Color::srgb(0.30, 0.31, 0.36),
                    custom_size: Some(Vec2::new(64.0, WALL_HEIGHT_PX)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                // z baixo (acima do chão, ABAIXO dos personagens): a parede nunca "morde"
                // o boneco. A oclusão iso bonita volta com a arte real das paredes.
                Transform::from_xyz(base.x, base.y, 10.0),
                Cell { x, y },
            ));
            // Paredes da FRENTE (abaixo do centro da casa na tela, lado da câmera) viram
            // transparentes ao entrar (cutaway) — as duas de baixo (esq+dir).
            if base.y < house_center.y {
                wall.insert(NearWall);
            }
        }
    }

    // Terreno elevado (platô/rampa): penhasco (face) + topo de grama erguido, por célula
    // com altura > 0. z baixo (abaixo dos personagens) para não "morder" o boneco.
    for y in 0..map_size.y {
        for x in 0..map_size.x {
            let h = map.get(x, y).map(|c| c.height).unwrap_or(0);
            if h <= 0 {
                continue;
            }
            let base = project(Vec2::new(x as f32, y as f32), origin, unit_x, unit_y);
            let lift = h as f32 * HEIGHT_PER_LEVEL;
            commands.spawn((
                Sprite {
                    color: Color::srgb(0.42, 0.34, 0.24), // penhasco (terra)
                    custom_size: Some(Vec2::new(64.0, lift)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(base.x, base.y, depth_z(x as f32, y as f32) - 0.1),
                Cell { x, y },
            ));
            commands.spawn((
                Sprite {
                    image: asset_server.load("tiles.png"),
                    rect: Some(Rect::new(0.0, 0.0, 64.0, 32.0)), // topo = losango de grama
                    custom_size: Some(Vec2::new(64.0, 32.0)),
                    ..default()
                },
                Transform::from_xyz(base.x, base.y + lift, depth_z(x as f32, y as f32)),
                Cell { x, y },
            ));
        }
    }

    // Guarda a walkmap no cliente (para o render erguer player/NPC sobre o relevo).
    commands.insert_resource(map.clone());

    built.0 = true;
    info!("zona desenhada: {}x{} (seed {:#x})", zone.tiles_x, zone.tiles_y, zone.seed);
}

/// Botão DIREITO **segurado** → converte o cursor na célula (tile) via `from_world_pos` e
/// re-envia o destino a cada frame (click-to-move contínuo, estilo Diablo/Ragnarok: segura
/// e o personagem segue o cursor).
fn read_right_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    if !buttons.pressed(MouseButton::Right) {
        return;
    }
    let (Ok(window), Ok((camera, camera_tf))) = (windows.single(), cameras.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(camera_tf, cursor) else {
        return;
    };
    let map_size = TilemapSize { x: 64, y: 64 };
    let grid_size = TilemapGridSize { x: 64.0, y: 32.0 };
    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let anchor = TilemapAnchor::Center;
    if let Some(tile) =
        TilePos::from_world_pos(&world, &map_size, &grid_size, &tile_size, &map_type, &anchor)
    {
        commands.client_trigger(MoveTo { tx: tile.x, ty: tile.y });
    }
}

/// Dá corpo visual (sprite) a cada player replicado que ainda não tem — cor derivada do
/// `PlayerId` para distinguir jogadores.
fn hydrate_players(mut commands: Commands, players: Query<(Entity, &Player), Without<Sprite>>) {
    for (entity, player) in &players {
        let id = player.0 .0;
        let r = ((id >> 16) & 0xFF) as f32 / 255.0;
        let g = ((id >> 8) & 0xFF) as f32 / 255.0;
        let b = (id & 0xFF) as f32 / 255.0;
        commands.entity(entity).insert((
            Sprite {
                color: Color::srgb(0.3 + 0.6 * r, 0.3 + 0.6 * g, 0.3 + 0.6 * b),
                custom_size: Some(Vec2::new(32.0, 48.0)), // ~0,5 tile largura, ~1,5 tile altura
                ..default()
            },
            Anchor::BOTTOM_CENTER, // pés no centro do tile (ajuda o y-sort)
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

/// Dá corpo visual ao NPC replicado (sprite azul, distinto dos players).
fn hydrate_npcs(mut commands: Commands, npcs: Query<Entity, (With<Npc>, Without<Sprite>)>) {
    for entity in &npcs {
        commands.entity(entity).insert((
            Sprite {
                color: Color::srgb(0.2, 0.5, 0.95),
                custom_size: Some(Vec2::new(32.0, 48.0)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, 0.0, 100.0),
        ));
    }
}

/// Base AFIM da projeção iso (origem + deltas de tela por +1 tile em x e y), derivada dos
/// helpers do `bevy_ecs_tilemap` — alinha EXATAMENTE com os losangos desenhados. Como a
/// projeção iso é afim nas coordenadas de tile, isto projeta posições CONTÍNUAS também.
fn iso_basis() -> (Vec2, Vec2, Vec2) {
    let map_size = TilemapSize { x: 64, y: 64 };
    let grid_size = TilemapGridSize { x: 64.0, y: 32.0 };
    let tile_size = TilemapTileSize { x: 64.0, y: 32.0 };
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let anchor = TilemapAnchor::Center;
    let center = |x: u32, y: u32| {
        TilePos { x, y }.center_in_world(&map_size, &grid_size, &tile_size, &map_type, &anchor)
    };
    let origin = center(0, 0);
    (origin, center(1, 0) - origin, center(0, 1) - origin)
}

/// Projeta uma posição do mundo (unidades de tile) para a tela iso.
fn project(pos: Vec2, origin: Vec2, unit_x: Vec2, unit_y: Vec2) -> Vec2 {
    origin + pos.x * unit_x + pos.y * unit_y
}

/// Z de profundidade iso: quem está MAIS À FRENTE (menor x+y, mais perto da câmera)
/// desenha por cima; quem está atrás é ocultado. Banda ~[60,100] — acima do chão/paredes,
/// abaixo do telhado (z=200).
fn depth_z(tile_x: f32, tile_y: f32) -> f32 {
    100.0 - (tile_x + tile_y) * 0.3
}

/// Projeta a `Position` replicada (players e NPC) para o `Transform` de render no iso.
fn sync_transforms(
    map: Option<Res<Walkmap>>,
    mut entities: Query<(&Position, &mut Transform)>,
) {
    let (origin, unit_x, unit_y) = iso_basis();
    for (position, mut transform) in &mut entities {
        let screen = project(position.0, origin, unit_x, unit_y);
        // ergue a entidade pela altura do tile onde está (fica EM CIMA do relevo).
        let lift = match map.as_deref() {
            Some(m) => m
                .cell_at_world(position.0)
                .and_then(|(x, y)| m.get(x, y))
                .map(|c| c.height as f32 * HEIGHT_PER_LEVEL)
                .unwrap_or(0.0),
            None => 0.0,
        };
        transform.translation.x = screen.x;
        transform.translation.y = screen.y + lift;
        // +0.1: desempata para a entidade ficar EM CIMA do relevo onde ela pisa.
        transform.translation.z = depth_z(position.0.x, position.0.y) + 0.1;
    }
}

/// Guarda qual player é o deste cliente (recebido do servidor).
fn on_assigned_player(trigger: On<AssignedPlayer>, mut me: ResMut<MyPlayer>) {
    me.0 = Some(trigger.event().0);
    info!("meu player: {:?}", trigger.event().0);
}

/// A câmera segue o PRÓPRIO player (resolve "o personagem sai do centro").
fn camera_follow(
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, position)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let (origin, unit_x, unit_y) = iso_basis();
    let screen = project(position.0, origin, unit_x, unit_y);
    for mut transform in &mut cameras {
        transform.translation.x = screen.x;
        transform.translation.y = screen.y;
    }
}

/// Ao entrar na casa: o telhado SOME e as paredes da FRENTE ficam transparentes — pra
/// enxergar o interior (cutaway iso, D1.4). Ao sair, tudo volta ao normal.
fn roof_fade(
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut roofs: Query<&mut Visibility, With<RoofLayer>>,
    mut near_walls: Query<&mut Sprite, With<NearWall>>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, position)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let (cx, cy) = (position.0.x.round(), position.0.y.round());
    let inside = cx >= HOUSE_X0 as f32
        && cx <= HOUSE_X1 as f32
        && cy >= HOUSE_Y0 as f32
        && cy <= HOUSE_Y1 as f32;
    for mut visibility in &mut roofs {
        *visibility = if inside {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    // Paredes da frente: fantasma (quase transparentes) dentro, sólidas fora.
    let alpha = if inside { 0.15 } else { 1.0 };
    for mut sprite in &mut near_walls {
        sprite.color = sprite.color.with_alpha(alpha);
    }
}

/// Visão por ambiente: quando o PRÓPRIO player está DENTRO da casa, só o interior (+ uma
/// faixa curta à frente da porta) fica visível; o resto do mundo e os players lá fora são
/// ocultados e o fundo vira preto (sensação de ambiente fechado). Quando está FORA, o mundo
/// aparece normal e os players que estão DENTRO de casas ficam ocultos (não se vê por cima
/// do telhado). O cenário estático só é reprocessado na TRANSIÇÃO entrar/sair.
fn apply_vision(
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut state: ResMut<VisionState>,
    mut clear: ResMut<ClearColor>,
    mut tiles: Query<(&TilePos, &mut TileVisible)>,
    mut scenery: Query<(&Cell, &mut Visibility), (Without<Player>, Without<Npc>)>,
    mut actors: Query<(&Position, &mut Visibility), Or<(With<Player>, With<Npc>)>>,
) {
    let inside = me
        .0
        .and_then(|id| players.iter().find(|(p, _)| p.0 == id))
        .map(|(_, pos)| {
            let (x, y) = (pos.0.x.round(), pos.0.y.round());
            x >= 0.0 && y >= 0.0 && in_house(x as u32, y as u32)
        })
        .unwrap_or(false);

    // Fundo: preto dentro (ambiente fechado), neutro fora.
    clear.0 = if inside { Color::BLACK } else { Color::srgb(0.1, 0.1, 0.1) };

    // Atores (players/NPC) — todo frame, pois se movem.
    for (pos, mut vis) in &mut actors {
        let (ax, ay) = (pos.0.x.round(), pos.0.y.round());
        let show = if ax < 0.0 || ay < 0.0 {
            !inside
        } else if inside {
            in_vision(ax as u32, ay as u32)
        } else {
            !in_house(ax as u32, ay as u32) // de fora, não se vê quem está dentro
        };
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // Cenário estático (chão + paredes/terreno) — só muda na transição entrar/sair.
    if state.inside != inside {
        state.inside = inside;
        for (tp, mut tv) in &mut tiles {
            tv.0 = !inside || in_vision(tp.x, tp.y);
        }
        for (cell, mut vis) in &mut scenery {
            let show = !inside || in_vision(cell.x, cell.y);
            *vis = if show { Visibility::Visible } else { Visibility::Hidden };
        }
    }
}

/// UI de zoom no topo-direito: label "Zoom" + botões "+" e "-". Cada botão tem um
/// observer nativo de clique (picking de UI) que ajusta o `scale` da câmera.
fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Text::new("Zoom"),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::WHITE),
            ));
            // (+) aproxima (scale menor); (-) afasta (scale maior).
            for (label, factor) in [("+", 0.8_f32), ("-", 1.25_f32)] {
                root.spawn((
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(24.0), ..default() },
                        TextColor(Color::WHITE),
                    ));
                })
                .observe(
                    move |_: On<PointerClick>, mut cameras: Query<&mut Projection, With<Camera2d>>| {
                        for mut projection in &mut cameras {
                            if let Projection::Orthographic(ortho) = &mut *projection {
                                ortho.scale = (ortho.scale * factor).clamp(0.1, 2.0);
                            }
                        }
                    },
                );
            }
        });
}
