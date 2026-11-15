# Débito técnico

Simplificações **deliberadas** feitas na demo/MVP para avançar rápido, que **devem
ser elevadas depois**. Registrar aqui preserva a distinção entre "decisão final" e
"atalho consciente" — e evita que um atalho vire, por esquecimento, a arquitetura
definitiva.

Legenda: 🔴 crítico (bloqueia qualidade-alvo) · 🟡 importante · 🟢 oportunista

| ID | Débito | Onde | Sev. | Elevar para | Gatilho p/ pagar |
|---|---|---|---|---|---|
| **DT-01** | Netcode da demo usa **replicon puro**, sem client-side prediction/rollback/lag-compensation | RFC 0005 · D4.1 | 🔴 | **Estado da arte**: predição+rollback+interpolação+lag-comp (via `lightyear` ou implementação própria de nível AAA) | Antes de avaliar seriamente a **sensação** de movimento online, ou ao passar de 2 p/ muitos jogadores |
| **DT-02** | Sem **cliente web** na demo (só desktop, via renet/UDP) | RFC 0005 · D4.2 | 🟡 | Cliente WASM no navegador via **QUIC/WebTransport** (`aeronet`+`quinn`) | Quando distribuição via navegador virar requisito |
| **DT-03** | Cliente compila com features de Bevy **curadas, sem áudio nem gamepad** (exclui `bevy_audio`/`bevy_gilrs`) para contornar `libasound2-dev`/`libudev-dev` ausentes na máquina de dev | `crates/client/Cargo.toml` · Passo 1 | 🟢 | Features default de plataforma (áudio + gamepad) de volta | Quando áudio/entrada de gamepad virarem necessários, ou ao padronizar a imagem/container de build com as libs de sistema |
| **DT-04** | Detecção de **célula de escada** por ranges hardcoded **duplicados em 3 lugares** (walkmap `build_house_floors`, clique do cliente, vãos de piso/telhado) — mudar a escada exige editar os três em sincronia | `shared/world.rs`, `client/main.rs` | 🟡 | Centralizar em `shared` (constantes/função "é célula da escada X") consumidas por todos | Ao adicionar outra casa/escada ou mudar o layout |
| **DT-05** | **Clique de nível** é heurístico: raycast num plano por altura + range de célula de escada, sujeito a parallax da geometria alta | `client/main.rs` `read_right_click` | 🟡 | **Mesh-picking** (clicar a geometria realmente visível) → subir/descer/andar 100% precisos | Ao querer andares de layout livre, ou clique "pixel-perfect" |
| **DT-06** | **Elevação discreta por waypoint** — o sprite sobe/desce 1 nível ao CHEGAR no waypoint (sem interpolar a altura no trajeto) | `shared/order.rs` `follow_move_order` | 🟢 | Interpolar `Elevation`/altura entre superfícies | Polish visual do movimento vertical |
| **DT-07** | **Queda só na mesma célula** (Enter → `surface_below` local); não cai de borda pra fora da estrutura | `server/main.rs` `on_jump` | 🟡 | Queda de borda (cair no chão ao lado), habilitável por nível de mapa | Gameplay vertical (pular do telhado pra fora) |
| **DT-08** | **FOV é 2D** (ignora elevação na linha de visão). A visão cross-nível (de baixo ver quem está na borda do telhado) funciona por o tile de parede ser o 1º hit do LoS | `client/main.rs` `compute_fov`/`apply_vision` | 🟢 | LoS ciente de altura (3D) | Mapas multi-nível complexos onde o truque 2D falhe |
| **DT-09** | `Walkmap::surfaces()` **aloca um `Vec`** a cada chamada, inclusive no laço interno do A* | `shared/walkmap.rs` | 🟢 | Iterador sem alocação / cache de superfícies | Perf em mapas grandes ou muitos agentes |

> **Princípio do usuário:** a camada de engenharia de rede **tem que ser estado da
> arte** — o atalho da demo existe só para provar o loop cliente-servidor, não para
> definir a qualidade final do netcode.
