# P4 — Padrões de Teste e Validação (Determinismo)

> Brief de pesquisa para fundamentar os **padrões de teste e validação** de `ragnadofus`
> (Rust/Bevy, AI-first). Princípio inegociável do projeto (AGENTS.md §3.2, RFC 0006):
> **verificação é um gate, não uma afirmação** — um agente precisa **provar** que algo
> funciona (build+teste verdes, evidência), não afirmar.
>
> Convenção de marcação: **[FATO]** = verificável em fonte primária/autoritativa (docs
> oficiais, repo da ferramenta, docs.rs, código-fonte). **[OPINIÃO]** = consenso de
> comunidade / blog de prática / recomendação (direcional, não canônico).

---

## 1. Contexto

`ragnadofus` compartilha o **mesmo ECS entre cliente e servidor** (Bevy 0.20.x), roda o
**servidor headless**, usa **`bevy_replicon`** para rede (com `lightyear` previsto quando
o netcode subir de nível — DT-01) e **`avian2d`** para física. É um RPG online de mundo
contínuo e **economia sistêmica** — ou seja, há invariantes de domínio fortes (ouro e
itens conservados em trocas, estoques não-negativos, IDs únicos) que são exatamente o que
property-based testing sabe atacar.

Este brief complementa o **RFC 0006 (harness mínimo de IA)** e o brief `E1-harness-ia.md`.
O RFC 0006 já decidiu quatro peças: (a) gate build+teste **headless** (`cargo build &&
cargo test` com `app.update()` + `TimeUpdateStrategy`, sem `DefaultPlugins`); (b) captura
de **screenshot headless**; (c) **BRP** (Bevy Remote Protocol) em dev; (d) **núcleo
determinístico** (`FixedUpdate` + RNG semeado, gravando seed+inputs+versão). Aqui
aprofundamos **como** cada peça vira teste que *prova*, e expandimos para replay,
property-based, visual regression, netcode e CI.

Quatro perguntas abertas do RFC 0006 que este brief ajuda a fechar: fronteira do núcleo
determinístico (§ Padrão 1); flakiness de headless+GPU em CI (§ Armadilhas); visual-diff
próprio e sua tolerância (§ Padrão 4); e o gate de CI de verdade (§ Padrão 6).

---

## 2. Fontes de alta qualidade (URLs)

### Determinismo (Bevy / Rust / física)
- Bevy `Time<Fixed>` (timestep fixo, 64 Hz) — https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html **[FATO]**
- Fontes de não-determinismo em Bevy (Alice I. Cecile, maintainer) — https://github.com/bevyengine/bevy/discussions/2480 **[FATO]**
- `bevy_ecs` schedule (executor multithreaded, `.chain()`, `SystemSet`, ambiguidade) — https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/index.html **[FATO]**
- Detecção de ambiguidade por padrão (proposta, mira Bevy 0.21) — https://github.com/bevyengine/bevy/pull/26047 **[FATO]**
- `std::collections::HashMap` (SipHash 1-3, semente aleatória, ordem arbitrária) — https://doc.rust-lang.org/std/collections/struct.HashMap.html **[FATO]**
- Box2D "Determinism" (Erin Catto — FMA, fast-math, transcendentais) — https://box2d.org/posts/2024/08/determinism/ **[FATO]**
- Glenn Fiedler "Floating Point Determinism" — https://gafferongames.com/post/floating_point_determinism/ **[FATO]**
- Rapier `enhanced-determinism` — https://rapier.rs/docs/user_guides/rust/determinism **[FATO]**
- Avian determinism (libm em toda a stack) — https://docs.rs/avian3d/latest/avian3d/ **[FATO]**
- `bevy_rand` (0.16, forking de PRNG, WyRand/ChaCha) — https://docs.rs/bevy_rand/latest/bevy_rand/ · https://github.com/Bluefinger/bevy_rand **[FATO]**
- Bevy `tests/how_to_test_apps.rs` (MinimalPlugins, `app.update()`, assert no World) — https://github.com/bevyengine/bevy/blob/main/tests/how_to_test_apps.rs **[FATO]**

### Property-based, replay, snapshot, fuzz
- proptest (book + docs.rs + repo) — https://proptest-rs.github.io/proptest/intro.html · https://docs.rs/proptest/latest/proptest/ · https://github.com/proptest-rs/proptest **[FATO]**
- quickcheck — https://github.com/BurntSushi/quickcheck **[FATO]**
- proptest-state-machine (teste model-based/stateful) — https://proptest-rs.github.io/proptest/proptest/state-machine.html · https://docs.rs/proptest-state-machine **[FATO]**
- insta / cargo-insta (snapshot de estado serializado) — https://insta.rs/ · https://insta.rs/docs/ **[FATO]**
- insta redactions (estabilizar floats/IDs/ordem) — https://insta.rs/docs/redactions/ **[FATO]**
- cargo-fuzz (libFuzzer) — https://rust-fuzz.github.io/book/cargo-fuzz.html · https://github.com/rust-fuzz/cargo-fuzz **[FATO]**
- arbitrary (fuzzing structure-aware) — https://docs.rs/arbitrary/latest/arbitrary/ · https://github.com/rust-fuzz/arbitrary **[FATO]**
- bolero (unifica libfuzzer/afl/honggfuzz + property test) — https://crates.io/crates/bolero **[FATO]**
- "Property Testing Stateful Code in Rust" (tutorial concreto) — https://rtpg.co/2024/02/02/property-testing-with-imperative-rust/ **[OPINIÃO]**
- Debug de desync em lockstep (replay + checksum por tick) — https://bugnet.io/blog/how-to-debug-desync-in-deterministic-lockstep-games **[OPINIÃO]**

### Visual regression / screenshot
- Bevy — reescrita de screenshots (componente `Screenshot` + observer `ScreenshotCaptured`) — https://github.com/bevyengine/bevy/pull/14833 **[FATO]**
- Bevy `bevy_dev_tools::ci_testing` (`CiTestingConfig`, screenshot em frame fixo) — https://docs.rs/bevy_dev_tools/latest/bevy_dev_tools/all.html **[FATO]**
- Bevy `examples/app/headless_renderer.rs` (render-to-texture + `ImageCopier`, sem janela) — https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs **[FATO]**
- Bevy `examples/app/no_renderer.rs` (desligar render de vez) — https://github.com/bevyengine/bevy/blob/main/examples/app/no_renderer.rs **[FATO]**
- Bevy CI de exemplos (xvfb + Mesa/lavapipe + PixelEagle) — https://github.com/bevyengine/bevy/blob/main/.github/workflows/example-run.yml **[FATO]**
- wgpu `RequestAdapterOptions.force_fallback_adapter` (adapter de software) — https://docs.rs/wgpu/latest/wgpu/type.RequestAdapterOptions.html **[FATO]**
- `image-compare` (SSIM + imagem de diff por pixel) — https://docs.rs/image-compare **[FATO]**
- `dssim` (SSIM multiescala, L\*a\*b\*) — https://github.com/kornelski/dssim **[FATO]**
- `dify` (pixelmatch em Rust, detecção de AA, máscaras) — https://github.com/jihchi/dify **[FATO]**
- pixelmatch (algoritmo de referência, OKLab/HyAB, detector de AA) — https://github.com/mapbox/pixelmatch **[FATO]**
- Pixel Eagle (serviço de VR do Bevy, pixel-diff vs perceptual-diff, NVIDIA FLIP) — https://pixel-eagle.com/blog/pixel-diff-vs-perceptual-diff/ **[OPINIÃO]**
- lavapipe (Vulkan por software, sem GPU) — https://techoverflow.net/2026/09/29/software-vulkan-with-lavapipe-run-shaders-and-pipelines-without-a-gpu/ **[FATO]**

### Netcode / multiplayer
- `tc-netem(8)` (latência/jitter/loss/reorder/rate no Linux) — https://man7.org/linux/man-pages/man8/tc-netem.8.html **[FATO]**
- toxiproxy (proxy TCP com "toxics") — https://github.com/Shopify/toxiproxy **[FATO]**
- clumsy (Windows, WinDivert) — https://jagt.github.io/clumsy/ **[FATO]**
- lightyear `LinkConditioner` + replicação determinística — https://docs.rs/lightyear_link/latest/lightyear_link/ · https://github.com/cBournhonesque/lightyear **[FATO]**
- lightyear `ClientServerStepper` / `DetStepper` (harness in-memory, crossbeam) — https://github.com/cBournhonesque/lightyear/blob/main/crates/tests/src/stepper.rs **[FATO]**
- bevy_replicon `ServerTestAppExt` (`connect_client` / `exchange_with_client`, sem socket) — https://github.com/projectharmonia/bevy_replicon/blob/master/src/test_app.rs **[FATO]**
- GGRS `SyncTestSession` (rollback 1 frame + checksum) — https://docs.rs/ggrs/latest/ggrs/struct.SyncTestSession.html **[FATO]**
- bevy_ggrs (checksum/desync, pitfalls de determinismo) — https://github.com/gschup/bevy_ggrs/blob/main/docs/pitfalls.md **[FATO]**
- "1500 Archers on a 28.8" (origem do lockstep determinístico + checksum por turno) — https://zoo.cs.yale.edu/classes/cs538/readings/papers/terrano_1500arch.pdf **[FATO]**
- turmoil (simulador determinístico de rede/tokio) — https://docs.rs/turmoil/ · https://tokio.rs/blog/2023-01-03-announcing-turmoil **[FATO]**

### CI Rust / monorepo de jogo
- Bevy `.github/workflows/ci.yml` (20+ jobs, matrix de SO, env, merge_group) — https://github.com/bevyengine/bevy/blob/main/.github/workflows/ci.yml **[FATO]**
- Bevy `tools/ci` (crate xtask: local == CI) — https://github.com/bevyengine/bevy/tree/main/tools/ci/src/commands **[FATO]**
- Bevy install-linux-deps (alsa/udev/wayland/xkb) — https://github.com/bevyengine/bevy/blob/main/.github/actions/install-linux-deps/action.yml **[FATO]**
- cargo-nextest (process-per-test, partição, retries) — https://nexte.st/ · https://nexte.st/docs/features/retries/ · https://nexte.st/docs/ci-features/partitioning/ **[FATO]**
- cargo-llvm-cov (cobertura source-based em stable, `--fail-under-*`) — https://github.com/taiki-e/cargo-llvm-cov **[FATO]**
- cargo-deny (advisories + licenças + bans) — https://crates.io/crates/cargo-audit (advisory) e EmbarkStudios/cargo-deny **[FATO]**
- Swatinem/rust-cache@v2 — https://github.com/Swatinem/rust-cache **[FATO]**
- xtask (matklad) — https://github.com/matklad/cargo-xtask **[FATO]**
- GitHub merge queue (checks precisam rodar em `merge_group`) — https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/managing-a-branch-protection-rule **[FATO]**
- corrode.dev "tips for faster CI builds" (CARGO_INCREMENTAL=0, nextest, cache) — https://corrode.dev/blog/tips-for-faster-ci-builds/ **[OPINIÃO]**

---

## 3. Padrões que adotamos

### Padrão 1 — Núcleo determinístico isolado e testável headless
A simulação de gameplay vive num **núcleo determinístico** separado da camada de
view/render. O núcleo roda em **`FixedUpdate`** (timestep fixo; Bevy usa **64 Hz** por
padrão — power-of-two que converte sem perda para f32/f64, e evita batimento com refresh de
monitor) **[FATO]** `docs.rs/.../struct.Fixed.html`. Regras do núcleo:

- **RNG sempre semeado e "forkado".** Usar `bevy_rand` (WyRand como default — mais rápido e
  mais portável; `rand_chacha` só quando precisar qualidade de entropia) e **derivar um
  stream por entidade** a partir de `(seed, EntityId)`, em vez de compartilhar um stream
  global entre sistemas paralelos **[FATO]** `docs.rs/bevy_rand`. Gravar **seed + inputs +
  versão** por sessão (já decidido no RFC 0006) torna qualquer run reproduzível.
- **Ordem de iteração estável.** Bevy **não garante** ordem estável de queries; dois
  sistemas iterando em ordens diferentes divergem **[FATO]** (discussion #2480; pitfalls do
  bevy_ggrs). Ordenar cada query por chave estável (ex.: ID de jogador/entidade) quando a
  ordem importar. **Nunca iterar `std::HashMap`/`HashSet`** em código de simulação — a
  semente SipHash é aleatória por mapa e a ordem é arbitrária **[FATO]** (docs std). Usar
  `BTreeMap`/`Vec` ordenado, ou um hasher determinístico.
- **Ordem explícita de sistemas.** `.chain()`, `.before/.after`, `SystemSet` +
  `configure_sets`. Habilitar/vigiar a **detecção de ambiguidade** (hoje opt-in via
  `ScheduleBuildSettings`; vira padrão na proposta de Bevy 0.21) **[FATO]** (PR #26047) —
  ambiguidade = par de sistemas com acesso conflitante e sem ordem → ordem aleatória
  frame-a-frame.
- **Testável sem render.** Padrão canônico do próprio Bevy (`how_to_test_apps.rs`):
  `App::new()` com **`MinimalPlugins`** (headless, sem janela/renderer), injetar input por
  recurso, chamar `app.update()` manualmente, e assertar no `World`
  (`world.query::<&T>()`, `world.resource::<T>()`) **[FATO]**. Controlar o tempo com
  `TimeUpdateStrategy` para cada `update()` avançar um delta fixo. **[OPINIÃO]** O primeiro
  `update()` inicializa o tempo com delta zero — "aquecer" antes de contar ticks; e
  `Commands` só aparecem após um sync point, então assertar depois da barreira.

### Padrão 2 — Property-based testing para mecânicas e servidor
Para invariantes de domínio (economia, inventário, trocas, máquinas de estado), usar
**`proptest`** (preferido a quickcheck: gera/encolhe via objetos `Strategy` explícitos, em
vez de só pelo tipo — mais flexível para valores com restrição) **[FATO]** (README proptest).
Práticas:

- **Invariantes como propriedades.** Ex.: `ouro_total_antes == ouro_total_depois` após
  qualquer sequência de trocas; contagem de itens conservada em transferências; HP ≥ 0; IDs
  únicos; só transições legais na máquina de estado. (O *mecanismo* é **[FATO]**; os
  invariantes de jogo específicos são **[OPINIÃO]** de design.)
- **Teste model-based/stateful** com **`proptest-state-machine`** **[FATO]**: um
  `ReferenceStateMachine` (modelo abstrato: `State`, `Transition`, `transitions()`,
  `apply()`, `preconditions()`) dirige a mesma sequência de transições contra o
  **sistema real** (`StateMachineTest` com `check_invariants()` após **cada** transição).
  Ideal para validar o **servidor de economia**: gera sequências aleatórias de operações
  (comprar/vender/transferir) e assegura que modelo e servidor nunca divergem. O
  encolhimento reduz a falha a uma sequência mínima de transições.
- **Casos de falha viram regressão permanente.** proptest grava `proptest-regressions/`
  com o caso mínimo que falhou — **commitar no VCS**; vira semente replayada em todo run
  **[FATO]**.

### Padrão 3 — Replay testing (golden replays) + snapshot de estado
- **Golden replay.** Gravar `(seed, stream de inputs, versão)` de uma sessão; o replay
  consome os inputs/escolhas gravados e **reproduz o run bit-a-bit**. A gravação commitada é
  uma *fixture de regressão*: rodar o replay contra o código novo e assertar que o estado
  final (ou checksums por tick) bate **[OPINIÃO]** (prática de indústria; sem crate Rust
  canônico — construímos in-house). Todo bug de desync reportado — fixado em tick +
  subsistema — vira um teste que re-simula os inputs e assere os hashes **[OPINIÃO]**.
- **Snapshot de estado serializado com `insta`.** Após N ticks determinísticos,
  serializar o estado de mundo/entidade e `assert_yaml_snapshot!` / `assert_json_snapshot!`
  (recomendação oficial: YAML de valores serializáveis) **[FATO]**. Fluxo: `cargo insta
  test` → `cargo insta review` para aceitar diffs; em CI o `INSTA_UPDATE=auto` vira "no".
  Usar **redactions** para estabilizar campos não-determinísticos: `rounded_redaction(n)`
  para floats, `sorted_redaction()` para coleções sem ordem, `".id" => "[id]"` para
  timestamps/UUIDs **[FATO]**. Isso é o "estado dourado" que casa com os golden replays.

### Padrão 4 — Visual regression com tolerância perceptual (fase P1)
Capturar screenshot headless pela API pós-#14833 (componente **`Screenshot`** + observer
**`ScreenshotCaptured`**, aceita qualquer `RenderTarget`, inclusive `RenderTarget::Image`)
ou pelo padrão render-to-texture do `headless_renderer.rs` **[FATO]**. Dirigir **frame
fixo** via `bevy_dev_tools::ci_testing` (`CiTestingConfig`) **[FATO]**. Para comparar:

- **Preferir métrica perceptual a diff pixel-a-pixel** em conteúdo de GPU — diff exato é
  ruidoso (um shift de 1 pixel faz quase tudo "diferir") e gera "diff fatigue" **[OPINIÃO]**
  (Pixel Eagle). Em Rust: **`dssim`** (SSIM multiescala, L\*a\*b\*) ou **`image-compare`**
  (SSIM + imagem de diff por pixel para debugar); **`dify`** se quiser um pixelmatch com
  detecção de anti-aliasing e máscaras **[FATO]**.
- **Tolerância por projeto, ajustada ao "noise floor" do renderer**; baselines **por SO**
  (macOS Metal não tem fallback de software; Linux lavapipe e Windows WARP diferem)
  **[OPINIÃO]**. O próprio Bevy compara poucos exemplos e via serviço dedicado (Pixel Eagle
  / NVIDIA FLIP) **[FATO/OPINIÃO]**.

RFC 0006 mantém visual-diff como **P1** (não-objetivo do MVP). Confirmado: **não há crate
Rust canônico de visual-diff integrado a Bevy** → implementação própria sobre `dssim`/
`image-compare`, e o maior custo real é *plumbing de pipeline + manutenção de baseline*, não
a matemática do diff (a correção de flakiness #15894 do Bevy foi infraestrutural) **[FATO]**.

### Padrão 5 — Testar netcode pelo paradigma (replicon ≠ lockstep)
A estratégia de validação **depende do paradigma de rede**:

- **State-replication / servidor autoritativo** (nosso caso com **`bevy_replicon`**; e
  lightyear no modo default): **não** exige determinismo. Validar rodando **servidor + N
  clientes in-process, sem socket**, avançando ticks e assertando **convergência do estado
  replicado**. `bevy_replicon` já expõe `ServerTestAppExt` (`connect_client`,
  `exchange_with_client` transfere mensagens direto entre `World`s, filtrado por cliente →
  multi-cliente), tudo em memória **[FATO]**. lightyear tem `ClientServerStepper`
  (MinimalPlugins + `CrossbeamIo`, `frame_step`/`tick_step`) **[FATO]** — padrões prontos
  para copiar.
- **Rollback / lockstep determinístico** (se DT-01 trouxer rollback via lightyear/GGRS):
  correção **é** determinismo → o gate primário é **SyncTest a cada frame + checksum**.
  GGRS `SyncTestSession` roda rollback de `check_distance` frames e compara checksums
  re-simulados (usar `check_distance ≥ 2`; 0/1 desligam a comparação) **[FATO]**.
  bevy_ggrs oferece `ComponentChecksumPlugin`/`EntityChecksumPlugin`, evento
  `SyncTestMismatch` e `DesyncDetection` em P2P **[FATO]**. É a formalização do padrão de
  "1500 Archers" (AoE): checksum do mundo por turno; divergiu → out of sync **[FATO]**.
- **Simular condições de rede em 3 camadas:** in-process (lightyear `LinkConditioner` com
  modelo de perda Gilbert-Elliot; ou `turmoil`) para testes determinísticos de
  integração; **toxiproxy** para caos em TCP; **`tc`/netem** (Linux), **clumsy** (Windows),
  dummynet (macOS) para validação ponta-a-ponta realista **[FATO]**.

### Padrão 6 — Pipeline de CI como gate real (local == CI)
Espelhar a arquitetura do Bevy, adaptada ao nosso workspace (`shared/`, `client/`,
`server/`):

- **Crate xtask (`tools/ci`)** invocado como `cargo run -p ci -- <cmd>`, para que **local e
  CI rodem exatamente o mesmo código** ("funciona local, falha no CI" deixa de existir)
  **[FATO]** (matklad xtask; tools/ci do Bevy).
- **Jobs paralelos** (não um monólito): lint (`cargo fmt --all -- --check` +
  `cargo clippy --workspace --all-targets --all-features -- -Dwarnings`) **[FATO]**;
  teste em **matrix de SO** com **`cargo-nextest`** (process-per-test — isola estado global
  e contexto gráfico, ~crucial para código de jogo; + retries com `flaky-result = "fail"`
  e partição `--partition hash:m/n`) **[FATO]**; cobertura com **`cargo-llvm-cov nextest`**
  (source-based, stable, `--fail-under-lines`) **[FATO]**; segurança com **`cargo-deny`**
  (advisories RustSec + licenças + bans) **[FATO]**.
- **Separar logic de render.** Gate rápido por PR roda crates de lógica **headless /
  no-renderer** (`no_renderer.rs` / `WgpuSettings`) — sem display, sem GPU **[FATO]**. Jobs
  pesados de render/visual-regression ficam num **workflow separado** com **xvfb +
  `mesa-vulkan-drivers` (lavapipe)** — Vulkan por software, sem GPU física **[FATO]**.
  Deps de build Linux: `libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`
  **[FATO]**.
- **Velocidade.** `Swatinem/rust-cache@v2`, `CARGO_INCREMENTAL=0`, `--locked`; sccache só se
  medido (combinar sccache + rust-cache é anti-padrão conhecido) **[OPINIÃO]**.
- **Gates e merge queue.** Branch protection com checks obrigatórios; todos os checks
  precisam disparar em **`merge_group`** (um check só em `pull_request` **nunca satisfaz** a
  fila e trava o merge) **[FATO]**.

Gate mínimo do projeto (AGENTS.md §6): `cargo build && cargo test` headless **bloqueia
entrega**. Este padrão é a evolução para "nível estado-da-arte".

---

## 4. Armadilhas (flakiness, determinismo)

**Determinismo — o que silenciosamente quebra:**
- **Float cross-platform.** IEEE 754 garante arredondamento correto só para `+ - * /`,
  sqrt e fma — **não** para transcendentais (`sin`, `cos`, `exp`, `powf`), que chamam o
  libm da plataforma e diferem no último bit (glibc ≠ macOS ≠ WASM) **[FATO]** (Box2D,
  Fiedler). LLVM pode contrair para **FMA** (desligar com `-ffp-contract=off`); **x87** tem
  100+ bugs (usar SSE2); **GPU/shader não é IEEE-compliant** **[FATO]** (discussion #2480).
  Mitigação: rotear transcendentais pelo **`libm`** puro-Rust (o que `avian2d` já faz com
  `enhanced-determinism`) **[FATO]**.
- **`enhanced-determinism` da física tem condições.** Rapier: bit-exato cross-platform **só
  se** plataformas estritamente IEEE 754-2008 + **mesma versão de Rapier e do rustc** +
  construção/ordem idênticas; incompatível com `simd8`; compatível com `parallel`
  **[FATO]**. Avian: timestep fixo por padrão + libm em toda a stack **[FATO]**. **Para
  nosso caso com `avian2d`**: tratar cross-platform como *hipótese a validar*, não garantia.
- **Pitfalls de rollback (se adotarmos).** Em bevy_ggrs, só componentes registrados via
  `rollback_component_with_*` são snapshotados — **estado não-registrado desync silencioso**;
  `Local<T>`, `Messages`/eventos e `GlobalTransform` não são snapshotados; `Entity` IDs
  ficam velhos após despawn (precisa `MapEntities`) **[FATO]**.
- **Tempo em teste.** Primeiro `app.update()` tem delta zero; `Commands` só após sync point
  **[OPINIÃO]**.

**Flakiness — onde testes "piscam":**
- **Visual regression é o mais frágil.** Variação sub-pixel/anti-aliasing entre
  GPUs/drivers/SO produz diffs imperceptíveis mas pixel-nível **[OPINIÃO]**. macOS Metal
  **não tem fallback de software** → teste que força adapter de software some/falha no Mac
  **[FATO]**. Mitigação: fixar **um** renderer de software (lavapipe) sob xvfb, frame fixo,
  métrica perceptual com tolerância, baselines por SO. **E saber que boa parte da
  "flakiness de screenshot" é plumbing de pipeline, não render** — a maior correção do Bevy
  (#15894) foi de infra (paths de artefato, permissões `workflow_run`), não de tolerância
  **[FATO]**.
- **Headless+GPU (wgpu) em CI.** Pode exigir software rendering; precisa de PoC (incerteza
  (b) do RFC 0006). Para a **maioria** dos testes (lógica/ECS/sim/netcode), **desligar o
  render de vez** (`no_renderer`) é mais rápido e confiável que xvfb+lavapipe **[FATO]**.
- **Testes de rede com tempo real.** Usar os steppers in-memory determinísticos
  (`ClientServerStepper`, `ServerTestAppExt`) para a CI; deixar `tc`/clumsy para validação
  manual/E2E, não para o gate por PR **[OPINIÃO]**.
- **Retries mascaram bugs.** nextest marca `FLAKY`; configurar `flaky-result = "fail"` para
  não deixar flakiness passar batido **[FATO]**.

**Limitações honestas:**
- Cobertura de código de **render/GPU/plataforma** é subcontada em CI headless — escopar o
  gate de cobertura às crates de lógica (`shared`, sim, netcode), não ao workspace inteiro
  **[OPINIÃO]** (mecanicamente correto; não achamos fonte autoritativa específica para jogo).
- "Testar só crates alteradas" em monorepo Rust **não** tem ferramenta canônica clara;
  candidatos a validar: `paths:` do GitHub Actions, `dorny/paths-filter`. Merge queue
  interage mal com "skip se não mudou" (re-roda em branch combinada) **[OPINIÃO]**.
- Property test *convence, não prova* (rtpg.co). Determinismo não é garantido por
  subsistema — definir a **fronteira do núcleo determinístico** é decisão, não default
  (questão aberta do RFC 0006) **[OPINIÃO]**.

---

## 5. Como validamos

Pirâmide de validação, do gate rápido por PR ao job pesado separado — tudo via `cargo run
-p ci -- <cmd>` para local == CI:

1. **Lint (sempre, rápido):** `fmt --check` + `clippy --workspace --all-targets
   --all-features -- -Dwarnings`.
2. **Teste de lógica headless (gate que bloqueia entrega):** `cargo-nextest` sobre
   `shared/`+sim+netcode com `MinimalPlugins`/`no_renderer`, tempo controlado por
   `TimeUpdateStrategy`. Inclui:
   - **property tests** (proptest) dos invariantes de economia/inventário;
   - **state-machine tests** (proptest-state-machine) do servidor;
   - **snapshot de estado** (insta, com redactions) após N ticks determinísticos;
   - **golden replay**: reproduzir `(seed, inputs)` gravado e assertar estado/checksums.
3. **Convergência de netcode (in-process):** servidor + N clientes via
   `ServerTestAppExt`/`ClientServerStepper`, sem socket, assertando estado replicado
   convergente; se houver lockstep/rollback, **SyncTest + checksum** como gate obrigatório.
4. **Cobertura:** `cargo-llvm-cov nextest` com `--fail-under-lines` nas crates de lógica;
   upload p/ Codecov.
5. **Segurança:** `cargo-deny` (advisories + licenças + bans).
6. **Render/visual regression (workflow separado, P1):** xvfb + lavapipe, screenshot em
   frame fixo via `ci_testing`, comparação perceptual (`dssim`/`image-compare`) com
   tolerância por projeto e baseline por SO.
7. **Fuzzing (periódico, nightly):** `cargo-fuzz` + `arbitrary` no (de)serializador de
   protocolo/save — propriedade round-trip `decode(encode(x)) == x` e "nenhum input entra em
   pânico". Crashes viram artefato replayável.
8. **Validação de rede realista (manual/E2E):** `tc`/netem (Linux), clumsy (Windows),
   `LinkConditioner`/turmoil in-process — fora do gate por PR.

**Critério de aceite (gate, não afirmação):** uma mudança só é "pronta" quando (1) lint +
teste headless verdes; (2) property/state-machine/snapshot/replay que cobrem o invariante
tocado passam; (3) se visual/interativa, screenshot anexada e visual-diff dentro da
tolerância; (4) se de rede, convergência (ou SyncTest/checksum) verde. Qualquer PR que
quebre o gate é **bloqueado automaticamente** e o critério do gate é **revelado ao agente**
(princípio regolo.ai, RFC 0006). Determinismo tem seu próprio teste: **reproduzir um bug a
partir de seed+inputs gravados**.
