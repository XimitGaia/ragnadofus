# A1 — Engine Base: Bevy como fundação para MMORPG 2.5D pixel-art (RFC de suporte)

> Pesquisa web multi-fonte realizada em **2026-11-14**. Cada afirmação é marcada como **[FATO]** (com fonte/URL) ou **[HIPÓTESE]** (síntese/inferência minha a partir das fontes). Versões e datas refletem o que as fontes reportavam na data da pesquisa.

---

## Problema

Decidimos (provisoriamente) usar **Bevy como base** (ECS, janela, input, render) para um MMORPG 2.5D pixel-art **online (cliente-servidor)**, mantendo um **módulo de render próprio compacto e substituível** como experimento. O MVP é uma fatia jogável online.

Esta RFC precisa confirmar ou qualificar essa escolha com dados atuais (2026), respondendo:

1. Maturidade de Bevy em 2026 (versão, estabilidade de API, 2D/2.5D/pixel-art, WebGPU/web, ecossistema).
2. Comparação honesta **Bevy vs Macroquad vs wgpu puro** para render 2.5D pixel-art com *palette shaders* e pipelines customizados.
3. **Networking para MMO** em Bevy (lightyear, bevy_replicon, renet, aeronet): maturidade, modelo servidor-autoritativo, limites de escala.
4. **Riscos** de depender de Bevy (churn, performance) e o que significa concretamente "abstrair as partes que queremos substituir".

As três premissas que a pesquisa precisa estressar: (a) Bevy é estável o suficiente para um projeto plurianual? (b) dá para manter um render próprio sem lutar contra a engine? (c) o networking aguenta um MMO autoritativo?

---

## Achados (com URLs)

### 1. Maturidade de Bevy em 2026

- **[FATO] Versão atual: Bevy 0.20.0 (08/out/2026), patch 0.20.1 (28/out/2026).** Ainda **pré-1.0**, sem data comprometida para 1.0. — https://bevy.org/news/bevy-0-20/ · https://docs.rs/crate/bevy/latest
- **[FATO] Cadência ~3 meses, com breaking changes a cada release.** Timeline 2026: 0.18 (13/jan), 0.19 (19/jun), 0.20 (08/out). A própria doc oficial afirma: *"A new version of Bevy containing breaking changes to the API is released approximately once every 3 months."* — https://bevy.org/news/ · https://bevy.org/learn/quick-start/introduction/
- **[FATO] A doc oficial ainda traz "Stability Warning"** e recomenda Godot para projetos grandes que precisam de estabilidade. — https://bevy.org/learn/quick-start/introduction/
- **[FATO] Cada release traz um guia de migração completo** (o 0.19→0.20 é extenso, ~10k+ palavras). Migrações são majoritariamente mecânicas ("o compilador diz o que quebrou"). — https://bevy.org/learn/migration-guides/0-19-to-0-20/
- **[FATO] Pixel-art é caminho de primeira classe:** `ImagePlugin::default_nearest()` (nearest-neighbor), texture atlas/sprite-sheet oficiais, exemplo oficial de *pixel-grid snapping*, e **sprite materials com shaders customizados** adicionados no 0.20. — https://bevy.org/examples/2d-rendering/pixel-grid-snap/ · https://bevy.org/examples/2d-rendering/sprite-sheet/ · https://bevy.org/news/bevy-0-20/
- **[FATO] WebGPU + WASM é first-class** (galeria oficial de exemplos WebGPU), com WebGL2 como fallback legado. **Caveat: o backend é escolhido em build-time** — um binário WASM não serve WebGL2 e WebGPU ao mesmo tempo; builds WebGPU não rodam em browsers sem WebGPU. WebGL2 não tem storage buffers (limita nº de luzes etc.); mesh shaders não rodam na web. — https://bevy.org/news/bevy-webgpu/ · https://bevy.org/examples-webgpu/ · https://github.com/bevyengine/bevy/issues/13168
- **[FATO] Maior ecossistema entre engines Rust:** ~501 entradas na página de Assets (ago/2026), ~23.4k membros no Discord, ~48k stars no GitHub, 6,66M downloads no crates.io. — https://bevy.org/news/bevys-sixth-birthday/ · https://github.com/bevyengine/bevy

### 2. Bevy vs Macroquad vs wgpu puro (render 2.5D pixel-art, palette shaders, pipelines custom)

**Bevy**
- **[FATO] Render roda sobre wgpu → shaders WGSL nativos (e GLSL também suportado para 2D).** Material customizado via trait `Material2d` + derive `AsBindGroup` (bindings `#[uniform]`, `#[texture]`, `#[sampler]`), registrado com `Material2dPlugin`. — https://bevy.org/examples/shaders/shader-material-2d/ · https://bevy.org/examples-webgpu/shaders/shader-material-glsl/
- **[HIPÓTESE] Palette-swap (cor indexada) é direto no nível de shader** — o exemplo `Material2d` já amarra textura+sampler+uniform, que é exatamente o encanamento de um palette shader (amostrar índice → lookup numa LUT/paleta). Não há tutorial oficial *específico* de palette-swap; a técnica é agnóstica de engine. — técnica: https://pvigier.github.io/2019/10/06/palette-swapping-with-shaders.html
- **[FATO] `ExtendedMaterial2d`/`MaterialExtension2d`** permite acrescentar lógica de shader por cima do `ColorMaterial` sem reconstruir o material. — https://taintedcoders.com/bevy/rendering
- **[FATO] O antigo RenderGraph por nós foi substituído por schedules ECS;** passes de render são systems nos schedules `Core2d`/`Core3d`; pipeline Extract→Prepare→Queue→Render→Cleanup num "render world" separado. Pipelines totalmente customizados são possíveis, porém "avançados" — **para palette shader NÃO é necessário descer a esse nível.** — https://taintedcoders.com/bevy/rendering · https://bevy.org/examples/shaders/custom-render-phase/
- **[FATO] Batching automático + GPU-driven** (multi-draw-indirect). — https://bevy.org/news/bevy-0-19/

**Macroquad**
- **[FATO] Framework 2D minimalista (estilo Raylib) sobre Miniquad (OpenGL/GL-ES), por not-fl3. Último: 0.4.16 (30/jul/2026), ativo.** — https://lib.rs/crates/macroquad · https://macroquad.rs/articles/macroquad-0-4/
- **[FATO] Shaders só em GLSL ES 2.0 (`#version 100`); SEM WGSL/wgpu.** Para Metal é preciso fornecer um `metal_shader` separado à mão. Render targets / uniforms seguem o modelo GL imediato, mais limitado que wgpu. — https://macroquad.rs/articles/macroquad-0-4/ · https://github.com/GlyphForged/mq_shader_template
- **[FATO] Sem ECS, sem arquitetura imposta** — você escreve o game loop. Bom para prototipagem, risco de "spaghetti" ao crescer.
- **[FATO — RISCO IMPORTANTE] Advisory de soundness RUSTSEC-2025-0035:** uso disseminado de mutable statics permite *use-after-free em safe code*; todas as versões afetadas, **sem versão corrigida**, correção "não parece ser prioridade". — https://rustsec.org/advisories/RUSTSEC-2025-0035.html

**wgpu puro**
- **[FATO] Maduro e estável em 2026.** WebGPU chegou a Candidate Recommendation Draft (W3C, mai/2026); WebGPU por padrão em Chromium e Safari 26 (~85% de suporte global). Releases wgpu: v29 (mar), v30 (jul), v30.0.1 (ago/2026). Shaders em WGSL. — https://github.com/gfx-rs/wgpu/releases · https://en.wikipedia.org/wiki/WebGPU
- **[FATO] Controle total:** você é dono do pipeline, bind groups, WGSL, surface. Janela via winit; **sem ECS, sem asset pipeline** — tudo isso vira seu código. Batching de sprites é DIY (instancing + atlas podem render a cena em 1 draw call). — https://blog.logrocket.com/rust-wgpu-cross-platform-graphics/ · https://www.benny-wilson.com/rust_wgpu/
- **[FATO] Curva de aprendizado íngreme** (pipelines, bind groups, WGSL). — https://appscale.blog/en/blog/webgpu-production-web-graphics-compute-2026
- **[HIPÓTESE] Nota-chave:** escolher Bevy já entrega WGSL + opção de custom render systems; **wgpu puro é "o renderer do Bevy sem a engine".** Macroquad é o destoante (GL/GLSL-ES, sem wgpu).

### 3. Networking para MMO em Bevy

A stack consolidou em camadas claras em 2026:

- **[FATO] aeronet 0.22 (Bevy 0.20)** — camada de IO/transporte (abstração). Implementações: channel (in-memory), websocket, **webtransport (QUIC, nativo + WASM)**, steam. Replicação/rollback/prediction são *não-objetivos* explícitos. lightyear usa aeronet para a maior parte do IO. — https://github.com/aecsocket/aeronet
- **[FATO] renet 2.0 / bevy_renet 5.0 (Bevy 0.19)** — camada de netcode/transporte reliable-UDP (canais ReliableOrdered/Unordered/Unreliable), auth + conexão, transporte Steam. Consumido pelo replicon via `bevy_replicon_renet`. — https://github.com/lucaspoffo/renet
- **[FATO] bevy_replicon 0.45 (Bevy 0.20)** — **replicação servidor-autoritativa** de baixo nível, backend-agnóstica ("No builtin I/O"). Componentes replicam **só servidor→cliente** (anti-cheat), um codebase serve singleplayer/cliente/dedicated-server/listen-server; visibilidade por cliente. **NÃO traz prediction/interpolation/rollback embutidos.** — https://github.com/simgine/bevy_replicon
- **[FATO] lightyear 0.31 (Bevy 0.20)** — camada alta *batteries-included*: client-side prediction + rollback, interpolação por snapshot, input buffering, lag compensation, interest management por **"Rooms"**. Servidor-autoritativo (também suporta P2P/host-client). **Mudança arquitetural: lightyear agora constrói a replicação SOBRE o bevy_replicon** (o antigo `lightyear_replication` foi depreciado). — https://github.com/cBournhonesque/lightyear · https://github.com/cBournhonesque/lightyear/issues/1350
- **[FATO] ECS do Bevy é usado no lado servidor na prática.** Ex. real mais ambicioso de MMO: **l2r** — emulador de servidor de Lineage 2 (High Five) em Rust sobre ECS do Bevy (combat, movement, inventory, AI, pathfinding, visibility). **Explicitamente early-development, NÃO production-ready.** — https://github.com/l2r-dev/l2r
- **[FATO] Limite de escala conhecido:** o interest management por Rooms do lightyear dá visibilidade correta e reduz banda, **mas não reduz o custo de travessia entidade×cliente** — gargalo real em escala de MMO. (Fonte: livro do lightyear, via snippets indexados; fetch direto deu 403 — fonte única, tratar com cautela.) — https://cbournhonesque.github.io/lightyear/book/
- **[HIPÓTESE] Não há MMO comercial grande comprovadamente shipado em Bevy.** Exemplos de produção reais em networking Bevy são indie/jam (lightyear: Lumina, cycles.io). Nenhum cap de jogadores concorrentes publicado/benchmarkado.

### 4. Riscos de depender de Bevy + "abstrair o que queremos substituir"

**Churn de API**
- **[FATO] Bevy quebra API a cada release até o 1.0 (sem data).** Thread de "revolta" dos devs (#21838, out/2025) com relatos diretos: *"always find myself chasing the next version, never get to stabilize"*, *"porting is hell"*, *"only suitable for hobby project"*. Mantenedores respondem que a migração *"typically takes 20 minutes to a couple hours"*. — https://github.com/bevyengine/bevy/discussions/21838
- **[FATO] Não dá para "congelar" limpo numa versão antiga:** o crate guarda-chuva `bevy` não fixa seus sub-crates com `=0.x.y`, então o Cargo puxa sub-crates mais novos mesmo pedindo versão antiga; e plugins de terceiros sobem em lockstep a cada release. — https://github.com/bevyengine/bevy/discussions/18586
- **[FATO] Estimativa de esforço (fonte única):** orçar ~1–3 dias de migração a cada 3–4 meses para se manter atual. — https://www.strayspark.studio/blog/bevy-rust-game-engine-2026-indie-guide

**Performance e compile time**
- **[FATO] ECS é rápido** (milhões de entidades; ~2,1 ms/milhão citado). Renderer competitivo (benchmarks informais vencendo Godot em 2D). **Performance NÃO é o risco primário.** — https://engineranked.com/article/bevy-engine-review-2026/
- **[FATO] Compile time é o atrito real.** Mitigações oficiais: `dynamic_linking` (iteração ~6s→~0,6s, mas não para ship), cranelift (build rápido/binário mais lento). 0.17 desacoplou o renderer para reduzir tempo de compilação de crates de terceiros. — https://bevy.org/learn/quick-start/getting-started/setup/ · https://bevy.org/news/bevy-0-17/

**Governança / sustentabilidade**
- **[FATO] Bevy Foundation** (sem fins lucrativos, WA-state; 501(c)(3) em set/2024) é dona da IP; board de 5 membros; primeiro funcionário pago full-time (~$150k/ano). Reduz risco de bus-factor/rug-pull vs. engine de dev solo. **[HIPÓTESE] Risco: base de staff pago é fina**, financiamento depende de doações. — https://bevy.org/news/bevy-foundation/ · https://bevy.org/news/bevy-foundation-501c3/

**"Abstrair as partes que queremos substituir" — o achado mais importante para a nossa estratégia**
- **[FATO] Desde o Bevy 0.17 o renderer é substituível como drop-in.** A API de render foi desacoplada do `bevy_render`: cameras, luzes, shaders, meshes, sprites, text, UI, picking, animation, scenes podem ser usados **sem `bevy_render`**. Citação oficial: *"It is now possible for 3rd party custom renderers to act as drop-in replacements for rendering Bevy scenes, without the need to pull in `bevy_render`."* — https://bevy.org/news/bevy-0-17/
- **[FATO] Bevy é plugin-modular; o renderer é um plugin que dá para desabilitar** (`.disable::<RenderPlugin>()`) e substituir. Precedentes reais: bevy_vello_renderer, bevy_pixels, PR #21287. — https://bevy.org/learn/quick-start/getting-started/plugins/ · https://github.com/bevyengine/bevy/pull/21287
- **[FATO] Prova comercial do padrão "ECS-only + renderer próprio": Tiny Glade** usa **só o ECS do Bevy** e construiu seu próprio renderer (Vulkan customizado); 600k+ vendas no 1º mês. **Tunnet** idem ("custom renderer on top of bevy ecs"). — https://80.lv/articles/exclusive-tiny-glade-developers-discuss-bevy-proceduralism-publishers-cozy-games · https://github.com/ThierryBerger/bevy_awesome_prod
- **[HIPÓTESE] Padrão arquitetural recomendado:** depender dos crates mais estáveis (`bevy_ecs`, `bevy_app`), manter a lógica de jogo como systems ECS/Rust puro atrás de traits (hexagonal/ports-and-adapters), e tratar renderer/UI como *adapters* substituíveis. **Caveat:** `bevy_ecs`/`bevy_app` ainda quebram entre releases — desacoplar o renderer reduz o blast radius, não elimina o churn do core que você mantém.

### Ferramentas 2.5D/pixel-art concretas (maturidade em 2026)

- **[FATO] Câmera pixel-perfect:** exemplo oficial de *pixel-grid snap* (render-to-low-res + upscale); **bevy_smooth_pixel_camera** 0.4.x (abr/2026) para scroll sub-pixel suave; **bevy_pixcam** 0.19 (jun/2026) como sucessor mantido do bevy_pixel_camera (este último está parado). — https://github.com/doonv/bevy_smooth_pixel_camera · https://lib.rs/crates/bevy_pixcam
- **[FATO] Tilemaps/isométrico:** **bevy_ecs_tilemap** (iso diamond + staggered, hex; tracks Bevy), **bevy_ecs_ldtk** 0.16 (out/2026, Bevy 0.20), **bevy_ecs_tiled** (ortho/hex/iso diamond — **sem iso staggered**). — https://github.com/StarArawn/bevy_ecs_tilemap · https://github.com/Trouv/bevy_ecs_ldtk/releases/tag/v0.16.0 · https://adrien-bon.github.io/bevy_ecs_tiled/FAQ.html
- **[FATO] 2.5D (sprites em cena 3D): bevy_sprite3d v11.1 (Bevy 0.20)** — sprites 2D no mundo 3D (billboards, perspectiva, luz; depth-buffer real). **bevy_mod_billboard** para elementos que encaram a câmera. — https://github.com/FraserLee/bevy_sprite3d
- **[FATO] Y-sort (2.5D painter's algorithm):** **extol_sprite_layer** (y-sort automático desacoplado do Z) e **bevy_sorting**. — https://lib.rs/crates/extol_sprite_layer
- **[FATO] Animação de sprites:** **benimator foi arquivado (fev/2025)** — não usar em projeto novo. Ecossistema vivo consolidou em **bevy_aseprite_ultra** (lê `.aseprite` com hot-reload) e **bevy_mod_aseprite** 0.14 (jan/2026). — https://github.com/jcornaz/benimator · https://github.com/lommix/bevy_aseprite_ultra
- **[FATO] Correções de atribuição:** *Gestalt: Steam & Cinder* e *Megabonk* são **Unity**, não Bevy. Tiny Glade usa só o ECS (renderer próprio). — https://en.wikipedia.org/wiki/Gestalt:_Steam_and_Cinder · https://en.wikipedia.org/wiki/Megabonk

---

## Alternativas (prós / contras / custo)

### Stack de render

| Opção | Prós | Contras | Custo/esforço | Adequação ao nosso caso |
|---|---|---|---|---|
| **Bevy (base + render próprio opcional)** | ECS maduro e rápido; WGSL + GLSL; `Material2d` cobre palette shaders sem descer ao render graph; batching GPU-driven; pixel-art e tilemaps de 1ª classe; **renderer drop-in substituível desde 0.17**; maior ecossistema Rust; governança institucional | Pré-1.0, breaking changes a cada ~3 meses; não dá para congelar versão; compile time alto; boilerplate/ceremônia | **Médio.** ~1–3 dias de migração a cada ~3–4 meses [HIPÓTESE, fonte única] + build inicial lento | **Alta.** É a única que entrega ECS+janela+input+asset pipeline e ainda permite render próprio sem abandonar a engine |
| **Macroquad** | Simples, compila rápido (<5s), ótimo para prototipar o *look*; quase sem dependências | **RUSTSEC-2025-0035 (soundness, sem fix)**; só GLSL ES 2.0, sem WGSL/wgpu; render targets/pipelines limitados; **sem ECS nem arquitetura**; risco de spaghetti em projeto grande | **Baixo p/ protótipo, alto p/ produção** (você reconstrói arquitetura) | **Baixa** como fundação de MMO plurianual; **útil** só para protótipo de arte/shader |
| **wgpu puro** | Controle total de pipeline, palette shaders, multi-pass 2.5D, batching; WGSL; wgpu maduro/estável (2026) | Sem ECS, sem asset pipeline, sem input/janela (winit DIY); curva íngreme; você constrói *tudo* | **Muito alto.** Reimplementa o que Bevy dá de graça | **Baixa** para o MVP (reinventa a engine); **é exatamente o que viraria nosso "módulo de render próprio" se/quando trocarmos o renderer do Bevy** |

### Stack de networking (todas sobre Bevy)

| Opção | Papel | Prós | Contras | Custo |
|---|---|---|---|---|
| **bevy_replicon** (0.45) | Replicação servidor-autoritativa de baixo nível | Anti-cheat (server→client only); backend-agnóstico; 1 codebase p/ todos os modos; visibilidade por cliente; otimizado/testado | Sem prediction/interpolation/rollback embutidos | Médio |
| **lightyear** (0.31) | Camada alta batteries-included (agora sobre replicon) | Prediction+rollback, interpolação, lag comp, Rooms; servidor-autoritativo | Mais opinativo; caveat de travessia entidade×cliente em escala MMO | Médio-baixo (traz muito pronto) |
| **aeronet** (0.22) | Abstração de transporte/IO | WebTransport (QUIC, WASM), WebSocket, Steam; usado pelo lightyear | Só IO (não replica) | Baixo |
| **renet/bevy_renet** (2.0/5.0) | Transporte reliable-UDP | Maduro, Steam, canais confiáveis | Só transporte; versão trilha Bevy 0.19 (checar p/ 0.20) | Baixo |

---

## Recomendação

**Confirmar Bevy como base, com qualificações explícitas.** Os achados sustentam a decisão: Bevy em 2026 (0.20.1) é a engine Rust mais madura em *capacidade* e a única que entrega ECS + janela + input + asset pipeline + render WGSL numa stack coesa, com pixel-art e tilemaps isométricos de primeira classe e governança institucional (Foundation 501c3). O risco central — churn de API pré-1.0 — é real, porém mecânico e mitigável.

Diretrizes concretas para a RFC:

1. **Adotar o padrão "ECS-only + renderer substituível", que o Bevy 0.17+ torna first-class** (renderer drop-in sem `bevy_render`) e que **Tiny Glade e Tunnet provam em produção**. Manter a lógica de jogo como systems ECS/Rust puro atrás de traits; tratar renderer e UI como *adapters*. Nosso "módulo de render compacto próprio" é, na prática, wgpu puro plugado no lugar do `bevy_render` quando quisermos — não um fork da engine.
2. **Networking: bevy_replicon (servidor-autoritativo) + aeronet (transporte, com WebTransport p/ web) como base; adicionar lightyear se precisarmos de prediction/interpolation/lag-comp.** Para pixel-art online em grid/tick, começar pela replicação autoritativa do replicon e medir cedo o custo de interest management em escala.
3. **Pixel-art/2.5D:** `ImagePlugin::default_nearest()` + exemplo oficial de pixel-grid snap (ou bevy_smooth_pixel_camera p/ scroll suave); tilemaps via bevy_ecs_tilemap/bevy_ecs_ldtk; para 2.5D real avaliar bevy_sprite3d; y-sort via extol_sprite_layer; animação via bevy_aseprite_ultra (**não** benimator).
4. **Palette shaders:** começar com `Material2d` (WGSL) — não precisa de custom render pipeline. Só descer ao render graph se multi-pass 2.5D/lighting exigir.
5. **Gestão de churn:** fixar versão, commitar `Cargo.lock`, orçar uma janela de migração a cada release (~trimestral), isolar dependências de plugins de terceiros atrás das nossas traits.
6. **Rejeitar Macroquad como fundação** (soundness RUSTSEC-2025-0035 sem fix, sem ECS, shaders GLSL-ES limitados) — no máximo como sandbox descartável de arte/shader. **Rejeitar wgpu puro como ponto de partida do MVP** (reinventaria a engine), reservando-o como o backend do nosso render próprio dentro do Bevy.

---

## Questões em aberto

1. **Escala de MMO não comprovada em Bevy.** Não achamos MMO comercial grande shipado em Bevy; o caso mais próximo (l2r, Lineage 2 emu) é pré-produção. **Precisamos de um benchmark próprio** de concorrência (N jogadores/entidades por tick) cedo no projeto. O caveat de travessia entidade×cliente do interest management do lightyear vem de fonte única (livro, fetch 403) — validar na prática.
2. **Custo real de manter um render próprio acoplado ao Bevy ECS.** O desacoplamento 0.17 é oficial, mas os precedentes (bevy_pixels, bevy_vello) são menores que um MMO. Qual o esforço de manter nosso renderer alinhado a cada release do `bevy_render`/`bevy_ecs`?
3. **Web (WebGPU) para o cliente online.** Backend é build-time (WebGPU xor WebGL2); suporte de browser ~85%. Precisamos decidir se o cliente web é alvo do MVP e qual backend — e medir se o pixel-art/palette shader se comporta igual em WebGPU vs nativo.
4. **Compatibilidade de versões da stack de networking.** replicon/lightyear/aeronet já em Bevy 0.20; bevy_renet reportado em 0.19 — confirmar trilha para 0.20 antes de fixar a árvore de deps.
5. **Churn do core que mantemos.** Mesmo isolando o renderer, `bevy_ecs`/`bevy_app` quebram entre releases; medir o blast radius real na nossa camada de lógica.

---

## Fontes

**Maturidade / versões**
- https://bevy.org/news/bevy-0-20/ · https://bevy.org/news/ · https://docs.rs/crate/bevy/latest
- https://bevy.org/learn/quick-start/introduction/ · https://bevy.org/learn/migration-guides/0-19-to-0-20/
- https://bevy.org/news/bevys-sixth-birthday/ · https://github.com/bevyengine/bevy

**Pixel-art / 2.5D / render**
- https://bevy.org/examples/2d-rendering/pixel-grid-snap/ · https://bevy.org/examples/2d-rendering/sprite-sheet/
- https://bevy.org/examples/shaders/shader-material-2d/ · https://bevy.org/examples/shaders/custom-render-phase/ · https://taintedcoders.com/bevy/rendering
- https://bevy.org/news/bevy-webgpu/ · https://bevy.org/examples-webgpu/ · https://github.com/bevyengine/bevy/issues/13168
- https://github.com/doonv/bevy_smooth_pixel_camera · https://lib.rs/crates/bevy_pixcam
- https://github.com/StarArawn/bevy_ecs_tilemap · https://github.com/Trouv/bevy_ecs_ldtk/releases/tag/v0.16.0 · https://adrien-bon.github.io/bevy_ecs_tiled/FAQ.html
- https://github.com/FraserLee/bevy_sprite3d · https://github.com/kulkalkul/bevy_mod_billboard · https://lib.rs/crates/extol_sprite_layer
- https://github.com/lommix/bevy_aseprite_ultra · https://github.com/jcornaz/benimator
- https://pvigier.github.io/2019/10/06/palette-swapping-with-shaders.html

**Macroquad / wgpu**
- https://macroquad.rs/articles/macroquad-0-4/ · https://lib.rs/crates/macroquad · https://rustsec.org/advisories/RUSTSEC-2025-0035.html
- https://github.com/gfx-rs/wgpu/releases · https://en.wikipedia.org/wiki/WebGPU · https://blog.logrocket.com/rust-wgpu-cross-platform-graphics/ · https://www.benny-wilson.com/rust_wgpu/

**Networking**
- https://github.com/cBournhonesque/lightyear · https://github.com/cBournhonesque/lightyear/issues/1350 · https://cbournhonesque.github.io/lightyear/book/
- https://github.com/simgine/bevy_replicon · https://github.com/lucaspoffo/renet · https://github.com/aecsocket/aeronet
- https://github.com/l2r-dev/l2r

**Riscos / churn / governança / renderer substituível**
- https://github.com/bevyengine/bevy/discussions/21838 · https://github.com/bevyengine/bevy/discussions/18586
- https://www.strayspark.studio/blog/bevy-rust-game-engine-2026-indie-guide
- https://bevy.org/learn/quick-start/getting-started/setup/ · https://bevy.org/news/bevy-0-17/
- https://bevy.org/news/bevy-foundation/ · https://bevy.org/news/bevy-foundation-501c3/
- https://bevy.org/learn/quick-start/getting-started/plugins/ · https://github.com/bevyengine/bevy/pull/21287
- https://80.lv/articles/exclusive-tiny-glade-developers-discuss-bevy-proceduralism-publishers-cozy-games · https://github.com/ThierryBerger/bevy_awesome_prod
- https://engineranked.com/article/bevy-engine-review-2026/

---

*FATO = afirmação com fonte direta. HIPÓTESE = síntese/inferência a partir das fontes. Caveats de confiança indicados inline (ex.: fonte única, fetch 403, benchmark informal).*
