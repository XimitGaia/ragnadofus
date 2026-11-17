# Handoff — Modularização do `client/main.rs` (feature-plugins)

> **Para a próxima sessão.** Este doc é o briefing completo pra refatorar o cliente num
> conjunto de **feature-plugins** bem organizados, sem mudar comportamento. Leia `AGENTS.md`
> antes de agir. Use **code-intel** como carro-chefe (grep/sed só pra texto literal).

## Objetivo

Quebrar `crates/client/src/main.rs` (hoje ~1200 linhas, tudo num arquivo) em **módulos por
feature**, cada um um **função-plugin** (`pub(crate) fn plugin(app: &mut App)`), seguindo o
padrão recomendado pela comunidade Bevy (plugin-por-feature, `main.rs` mínimo, agrupar por
feature e não por camada ECS, ordenar com `SystemSet` em vez de `.chain()`).

**Meta de manutenção:** mexer só no desenho → `sprites.rs`; só no andar → `animate_directional`
em `sprites.rs`; só na câmera → `camera.rs`; etc. Sem precisar ler o arquivo inteiro.

## Estado atual (ponto de partida)

- Branch: `feature/camera-3d` (base GitFlow: `develop`).
- Último commit: `7f07d22 refactor(client): unifica animação direcional num sistema data-driven`.
- **Keystone já feito:** `animate_knight` + `animate_wizard` já foram unificados num único
  `animate_directional` guiado por dados (componente `DirectionalSprite`, montado na hidratação
  via `DirectionalSprite::knight(..)` / `::wizard(..)`). NÃO desfazer isso.
- Gate verde (`cargo run -p xtask ci`), tree limpo exceto `assets-review/` e
  `docs/proposta-licenciamento-gravity.*` (NÃO commitar esses dois).

## Regras do refactor (inegociáveis)

1. **Sem mudar comportamento.** É reorganização pura. Rodar o cliente e confirmar que tudo
   funciona igual (mago/cavaleiro andando em 8 direções, câmera, visão, pulo).
2. **Verde a cada passo:** após CADA módulo extraído, `validate_build` (code-intel) + relançar
   server+client pra smoke test (sem panic). `cargo run -p xtask ci` antes de cada commit.
3. **Um commit por módulo** (`refactor(client): extrai <modulo>.rs`), pra nunca quebrar o checkpoint.
4. **code-intel primeiro:** tente `move_symbol`/`move_file` (na sessão passada o rust-analyzer
   não tinha "mover pra novo arquivo" — se já tiver, use). Se ainda não tiver, mova o bloco
   **verbatim** (Read + Write no módulo novo + remover do `main.rs`) e use code-intel para a
   parte semântica: `find_references` (mapear uso), `organize_imports`, `safe_delete` (código
   morto), `validate_build`. **Nunca grep/sed pra renomear/mover símbolo.**

## Truque de visibilidade (Rust)

- **Módulos-filho enxergam itens privados do crate-root** via `crate::Nome`. Então tipos/consts
  que ficarem no `common.rs` (ou mesmo soltos) podem ser acessados por qualquer módulo sem virar
  `pub`. Só precisa ser `pub(crate)` o que o **pai** (ou outro módulo) acessa do filho — na
  prática, só a `pub(crate) fn plugin(app)` de cada módulo (os sistemas podem ficar privados).

## Estrutura-alvo

```
crates/client/src/
├── main.rs      # mínimo: lê RAGNADOFUS_SERVER, App::new().add_plugins(AppPlugin).run()
├── app.rs       # AppPlugin: DefaultPlugins curado + WinitSettings + ClearColor + ProtocolPlugin +
│                #   RepliconRenetPlugins + MeshPickingPlugin; define e ORDENA os SystemSet;
│                #   add_plugins((net, world, sprites, camera, vision, input, ui)::plugin)
├── common.rs    # tipos/consts compartilhados por ≥2 módulos
├── net.rs       # conexão/netcode
├── world.rs     # construção do mundo 3D
├── sprites.rs   # billboards + animação direcional ("desenho" + "andar")
├── camera.rs    # câmera orbital + rotação + auto-rotação anti-oclusão
├── vision.rs    # FOV / cutaway
├── input.rs     # input de ações (clique-mover, pulo, hop)
└── ui.rs        # UI (zoom)
```

### Mapa de símbolos → módulo (do `document_symbols` atual)

**common.rs** — `MyPlayer`, `Billboard`, `SpriteHalf`, `Cell`, `Platform`, `HouseWall`, `DimPair`;
consts `LEVEL_HEIGHT`, `BILLBOARD_HALF`, `BILLBOARD_W`.

**net.rs** — const `DEFAULT_SERVER`; `PendingConnect`; fns `connect_when_ready`, `make_netcode`,
`on_assigned_player`. `plugin`: insere `PendingConnect` (precisa do `server_addr`), `init_resource::<MyPlayer>()`,
`add_systems(Update, connect_when_ready)`, `add_observer(on_assigned_player)`.

**world.rs** — consts `WALL_LEVELS`, `FLOOR_HEIGHT`; `ZoneBuilt`; fns `setup_scene`, `on_zone_received`,
`dim_color`. `plugin`: `init_resource::<ZoneBuilt>()`, `add_systems(Startup, setup_scene)`,
`add_observer(on_zone_received)`. (Spawna `Cell/Platform/HouseWall/DimPair/Billboard` de `common`.)

**sprites.rs** — consts `HERO_H/W/HALF`, `WIZ_H/W/LIFT/HALF`, `WIZ_ROW`, `MOVE_HOLD`; tipos
`KnightSprites`, `WizardSprites`, `DirFrames`, `DirectionalSprite`(+impl), `DirAnim`; fns
`dir8_to_base`, `dir8_from_velocity`, `load_assets`, `hydrate_players`, `hydrate_npcs`,
`billboard_material`, `sync_transforms`, `face_trees`, `animate_directional`. `plugin`:
`add_systems(Startup, load_assets)` + os sistemas de Update nos sets (ver ordenação).
Depende de `crate::camera::CamOrbit` (billboards encaram a câmera).

**camera.rs** — `CamOrbit`(+`Default`), const `CAM_DIST`; fns `rotate_input`, `house_center_xz`,
`house_cone`, `wrap_angle`, `auto_rotate_camera`, `camera_orbit`. `plugin`: `init_resource::<CamOrbit>()`
+ sistemas nos sets.

**vision.rs** — `VisionState`, const `FOV_RADIUS`; fns `in_house`, `los_clear`, `compute_fov`,
`apply_vision`. `plugin`: `init_resource::<VisionState>()` + `apply_vision` no set `Vision`.

**input.rs** — `HopAnim`, consts `HOP_DUR`, `HOP_HEIGHT`; fns `read_right_click`, `read_jump`,
`read_hop`, `hop_arc`. `plugin`: `init_resource::<HopAnim>()` + sistemas nos sets.

**ui.rs** — `setup_ui`. `plugin`: `add_systems(Startup, setup_ui)`.

## Tradução do `.chain()` → `SystemSet` (CRÍTICO pra não mudar comportamento)

Hoje, no `main.rs`, todos os sistemas de `Update` rodam num único `.chain()` nesta ordem:

```
read_right_click, read_jump, read_hop, hydrate_players, hydrate_npcs, rotate_input,
sync_transforms, hop_arc, face_trees, animate_directional, auto_rotate_camera,
camera_orbit, apply_vision
```

Ao fatiar em plugins, defina em `app.rs` um `SystemSet` ordenado que **preserva** essa ordem:

```rust
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ClientSet { Input, Spawn, Sync, Anim, Camera, Vision }
// em app.rs:
app.configure_sets(Update, (
    ClientSet::Input, ClientSet::Spawn, ClientSet::Sync,
    ClientSet::Anim, ClientSet::Camera, ClientSet::Vision,
).chain());
```

Cada módulo registra seus sistemas no set certo (`.in_set(ClientSet::X)`):

| Set | Sistemas (módulo) |
|---|---|
| **Input** | `read_right_click`, `read_jump`, `read_hop` (input) · `rotate_input` (camera) |
| **Spawn** | `hydrate_players`, `hydrate_npcs` (sprites) |
| **Sync** | `sync_transforms` (sprites) |
| **Anim** | `hop_arc`, `face_trees`, `animate_directional` (sprites/input) |
| **Camera** | `auto_rotate_camera`, `camera_orbit` (camera) |
| **Vision** | `apply_vision` (vision) |

Dependências que a ordem acima **garante** (não quebrar): `Spawn` antes de `Sync` (entidade nova
já com Transform); `Sync` antes de `Anim` (anim/hop compõem POR CIMA do Transform); `Camera` depois
de `Anim`; `Vision` depois de `Camera` (`apply_vision` lê o `Transform` da câmera). `rotate_input`
fica em `Input` (yaw atualizado antes de Sync/Anim usarem — igual hoje).

> `connect_when_ready` e os observers (`on_zone_received`, `on_assigned_player`) **não** entram
> nos sets (não são ordem-crítica); cada plugin os adiciona direto.

## Não quebrar (gotchas do build curado 3D)

No `app.rs` preserve EXATAMENTE a config de hoje do `main()`:
- feature/plugins de render 3D já ligados; `PresentMode::AutoNoVsync`; conectar só pós-warmup
  (`connect_when_ready`); `Tonemapping::None` + `DebandDither::Disabled` na câmera;
  `WinitSettings` contínuo (focused/unfocused) senão o netcode cai; `ImagePlugin::default_nearest()`
  (pixel art crisp). Ver memórias `bevy-3d-gotchas-build-curado` e `camera-3d-migracao`.

## Verificação (gate)

- Build/check: code-intel `validate_build` após cada módulo.
- Gate completo antes de commitar: `cargo run -p xtask ci` (grafo, renderless, build, 18 testes,
  clippy `-D warnings`). Clippy pega `doc_lazy_continuation` em doc-comments com lista — cuide
  da indentação/linha em branco ao mover doc-comments.
- Runtime: subir `./target/debug/server` e `./target/debug/client` (background) e conferir no log
  "mundo 3D construído" + ausência de `panic`. Confirmar visualmente mago/cavaleiro andando.

## Ordem sugerida de extração (do mais autocontido pro mais acoplado)

1. `ui.rs` (trivial — prova o mecanismo) → 2. `vision.rs` → 3. `input.rs` → 4. `net.rs` →
5. `camera.rs` → 6. `common.rs` (tipos compartilhados) → 7. `sprites.rs` → 8. `world.rs` →
9. `app.rs` + enxugar `main.rs`. Commit por módulo. Smoke test no fim.

## Referências (pesquisa já feita)

- [taintedcoders — Plugin organization](https://taintedcoders.com/bevy/patterns/plugin-organization)
- [taintedcoders — Code organization](https://taintedcoders.com/bevy/code-organization)
- [vladbat00 — Organising Bevy projects](https://vladbat00.github.io/blog/001-organising-bevy-projects/)
- [TheBevyFlock/bevy_new_2d — design.md](https://github.com/TheBevyFlock/bevy_new_2d/blob/main/docs/design.md)
- [omnidotdev/template-bevy](https://github.com/omnidotdev/template-bevy)
