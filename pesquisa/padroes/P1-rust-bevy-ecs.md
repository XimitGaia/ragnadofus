# P1 — Padrões de Código e Arquitetura: Rust + Bevy ECS (brief de fundação)

> Pesquisa web multi-fonte realizada em **2026-11-14**. Cada afirmação é marcada como **[FATO]** (com fonte/URL) ou **[OPINIÃO/CONVENÇÃO]** (síntese, convenção da comunidade ou recomendação de projeto minha). Versões refletem o que as fontes reportavam na data.
>
> Convenção de marcação igual aos demais briefs de `pesquisa/`, adaptada ao pedido: usamos **[OPINIÃO/CONVENÇÃO]** no lugar de `[HIPÓTESE]`.

---

## Contexto

`ragnadofus` já decidiu (RFCs 0001/0005, briefs A1/D1): **Rust + Bevy 0.20.x**, **mesmo ECS compartilhado cliente↔servidor**, **servidor Bevy headless autoritativo**, cliente desktop, rede via `bevy_replicon` (depois `lightyear` — DT-01), física `avian2d`, persistência atrás de um trait `WorldRepository`, e **renderer próprio como módulo substituível** (não reescrever a engine).

Este brief não reabre essas decisões: ele fixa os **padrões de código e arquitetura** para executá-las bem num projeto plurianual e AI-first. Cinco frentes:

1. Organização de workspace/crates (shared/client/server), evitando dependências cruzadas, com feature flags.
2. ECS idiomático em Bevy (components/systems/resources, system sets, states, events, plugins) + data-oriented design vs OOP + anti-padrões.
3. Estilo Rust para projeto grande: erros (`thiserror`/`anyhow`), newtypes, módulos, traits, banir `unwrap` em produção.
4. Manter o **renderer substituível** sem acoplar o resto a `bevy_render`.
5. Armadilhas do Bevy pré-1.0 (churn, migração) e mitigação.

---

## Fontes de alta qualidade (com URLs)

**Oficial Bevy**
- ECS (guia oficial): <https://bevy.org/learn/quick-start/getting-started/ecs/> · `bevy_ecs` README: <https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/README.md> · exemplo `ecs_guide`: <https://github.com/bevyengine/bevy/blob/main/examples/ecs/ecs_guide.rs>
- Plugins (oficial): <https://bevy.org/learn/quick-start/getting-started/plugins/>
- **Bevy 0.17 — decoupling do renderer** (crucial p/ item 4): <https://bevy.org/news/bevy-0-17/>
- Bevy 0.20 (release) e guia de migração 0.19→0.20: <https://bevy.org/news/bevy-0-20/> · <https://bevy.org/learn/migration-guides/0-19-to-0-20/>
- Exemplo headless renderer: <https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs>
- Organização de crates do monorepo: <https://deepwiki.com/bevyengine/bevy/1.2-crate-organization>
- Discussão de estabilização / `bevy_platform` 1.0: <https://github.com/bevyengine/bevy/discussions/9789>

**Comunidade reconhecida**
- **tbillington — `bevy_best_practices`** (referência de facto): <https://github.com/tbillington/bevy_best_practices>
- **taintedcoders** — organização de plugins e código: <https://taintedcoders.com/bevy/patterns/plugin-organization> · <https://taintedcoders.com/bevy/code-organization>
- Unofficial Bevy Cheat Book — states: <https://bevy-cheatbook.github.io/programming/states.html> (NB: o Cheat Book declara-se **desatualizado**; usar com migração oficial ao lado — <https://bevy-cheatbook.github.io/>)
- Discussão oficial sobre custom renderer / modular rendering: <https://github.com/bevyengine/bevy/discussions/2265> · <https://github.com/bevyengine/bevy/pull/2831>

**Rust / estilo / erros / DoD**
- Rust API Guidelines (checklist): <https://rust-lang.github.io/api-guidelines/checklist.html>
- `anyhow` (dtolnay): <https://github.com/dtolnay/anyhow> · `thiserror`: <https://github.com/dtolnay/thiserror>
- Microsoft Rust Patterns — Error Handling: <https://microsoft.github.io/RustTraining/rust-patterns-book/ch10-error-handling-patterns.html>
- Clippy `unwrap_used`/`expect_used`: <https://users.rust-lang.org/t/how-do-i-ban-unwrap-and-expect-function/57368> · <https://rust.code-maven.com/avoid-unwrap>
- Data-Oriented Design: <https://en.wikipedia.org/wiki/Data-oriented_design> · intro (Rust): <https://news.ycombinator.com/item?id=24506744>

---

## Padrões que adotamos (lista acionável)

### 1. Workspace / crates (shared / client / server)

- **[FATO]** O próprio Bevy é um Cargo workspace com **80+ crates em `crates/`**, cada um com uma responsabilidade, e o crate guarda-chuva `bevy` liga/desliga-os por **feature flags** — modelo de referência de workspace modular. — <https://deepwiki.com/bevyengine/bevy/1.2-crate-organization>
- **[FATO]** Bibliotecas de netcode já estabelecem o padrão de três alvos com protocolo compartilhado: `bevy_quinnet` vem com *"headless server, terminal client, and a shared protocol"*; **naia** usa `naia-shared` como dependência de path consumida por `*-server` e `*-client`. — <https://lib.rs/crates/bevy_quinnet> · <https://lib.rs/crates/naia-bevy-shared>
- **[OPINIÃO/CONVENÇÃO]** Comece com **1 crate** e só fatie em crates quando houver ganho real (build/isolamento). *"Start with a single crate—avoid premature optimization… Break logic into separate crates later as the project grows."* — <https://taintedcoders.com/bevy/patterns/plugin-organization>
- **Layout alvo** (quando fatiar): workspace com três crates + opcionais.
  - `shared/` — **o ECS do jogo**: components, eventos de rede, enums de estado do mundo, regras determinísticas de simulação, o protocolo de replicação e os *traits* de borda (ex.: `WorldRepository`). **Não depende de `client` nem de `server`.** É onde mora a lógica autoritativa que roda idêntica dos dois lados.
  - `server/` — binário Bevy **headless** (`MinimalPlugins` + plugins de simulação/rede/persistência). Depende de `shared`.
  - `client/` — binário desktop (`DefaultPlugins` + render + input + UI + predição). Depende de `shared`.
  - opcionais: `render/` (nosso renderer substituível, ver item 4), `persistence/` (impls do `WorldRepository`).
- **[OPINIÃO/CONVENÇÃO] Regra de ouro contra dependências cruzadas:** o grafo é estritamente **`client → shared`** e **`server → shared`**; **nunca** `shared → client/server`, e **nunca** `client ↔ server`. `shared` é Bevy-ECS puro (idealmente sem `bevy_render`). Isso é o que torna "mesmo ECS compartilhado" verdadeiro e barato de verificar.
- **[OPINIÃO/CONVENÇÃO] Feature flags** no crate guarda-chuva/local, não `#[cfg]` espalhado: `shared` expõe `render` (liga tipos de render só no cliente), `dev` (dynamic_linking, diagnostics), `headless`. O servidor compila `shared` **sem** a feature `render` e sem `bevy_render`. Casa com o item 4 e corta compile-time do servidor.

### 2. ECS idiomático em Bevy (e DoD vs OOP)

- **[FATO]** Components são **structs normais** com `#[derive(Component)]`; Systems são **funções normais** cujos parâmetros declaram o acesso a dados (o scheduler infere paralelismo); Resource é o singleton global identificado pelo tipo. — <https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/README.md> · <https://bevy.org/learn/quick-start/getting-started/ecs/>
- **[FATO] Não coloque lógica em componentes — crie um system.** Coloque só a informação mais específica em cada component (reutilizando `Transform` etc.); evite `struct Player` monolítico (*"this is considered bad practice"*). Prefira **componentes granulares** + **marker components** (tags) + **bundles** como templates de spawn. — <https://bevy.org/learn/quick-start/getting-started/ecs/> (seção de data-oriented) · <https://bevy-cheatbook.github.io/programming/intro-data.html>
- **[FATO] Data-Oriented Design:** componentes do mesmo tipo ficam **contíguos na memória** (SoA), organizados por **arquétipo**, para iteração cache-friendly; é o oposto do pointer-chasing de OOP. DoD = projetar para *data locality*. — <https://en.wikipedia.org/wiki/Data-oriented_design> · <https://deepwiki.com/bevyengine/bevy/1.2-crate-organization> (arquétipos)
- **[FATO] Eventos para comunicação entre subsistemas.** *"Use Events to structure logic and communication between subsystems"*; leitores devem ser **ordenados depois** dos escritores no frame (`.before()/.after()/.chain()`); systems que só reagem a evento devem usar `.run_if(on_event::<E>())`. — <https://github.com/tbillington/bevy_best_practices>
- **[FATO] States + SystemSet como regra:** *"All systems added to the `Update` schedule should be bound by run conditions on `State` and `SystemSet`."* Pares setup/cleanup de transição têm `OnEnter`/`OnExit` **co-localizados**. — <https://github.com/tbillington/bevy_best_practices>
- **[FATO] `configure_sets` é por schedule** — a config de sets em `Update` **não** vale em `FixedUpdate`/`OnEnter`/`OnExit`. Reconfigurar por schedule. — <https://bevy-cheatbook.github.io/programming/states.html>
- **[FATO] `.chain()` vs `chain_weak`:** `.chain()` força toda a ordem (todo X antes de qualquer Y, mesmo sem dado em comum); `chain_weak` (0.20) relaxa quando não há dependência real de dados — usar só nesse caso. — <https://bevy.org/learn/migration-guides/0-19-to-0-20/>
- **[FATO] Simulação em `FixedUpdate` (tick fixo autoritativo); render/input em `Update`.** Casa com o modelo servidor-autoritativo de D1 (tick fixo + reconciliação). — D1 (`pesquisa/D1-cliente-servidor.md`); scheduler: <https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/README.md>
- **[OPINIÃO/CONVENÇÃO] Nossos SystemSets de projeto** (nomeados e documentados num único lugar — *"record exactly what each of these system sets do in a single place"*, tbillington): ex. `Input → Intent → Simulation → Replication → Presentation`. Documentar o contrato de ordenação num `prelude`/módulo central.
- **Anti-padrões a evitar (lista):**
  - **[FATO]** Lógica/métodos "gordos" em components (OOP disfarçado). — <https://bevy.org/learn/quick-start/getting-started/ecs/>
  - **[FATO]** Guardar `Entity` para referência persistente/de rede — *"Entity is more akin to a pointer; not to be relied upon across sessions or over the network"*. Usar **IDs fortes** (newtype próprio). — <https://github.com/tbillington/bevy_best_practices>
  - **[OPINIÃO/CONVENÇÃO]** `.chain()` em schedules inteiros "por segurança" (mata paralelismo); systems gigantes com muitas queries acopladas; `Query` sem filtros `With/Without/Changed` quando dá para restringir.

### 3. Estilo Rust para projeto grande

- **[FATO] Erros — regra de ouro:** **`thiserror` em bibliotecas** (erros tipados, o consumidor dá `match` nas variantes) e **`anyhow` em binários/aplicação** (erro opaco com contexto). *"Using anyhow in library crates is a mistake."* — <https://microsoft.github.io/RustTraining/rust-patterns-book/ch10-error-handling-patterns.html> · <https://github.com/dtolnay/anyhow> · <https://github.com/dtolnay/thiserror>
  - **[OPINIÃO/CONVENÇÃO] Para nós:** `shared` (lib do jogo) e o trait `WorldRepository` expõem **erros tipados com `thiserror`** (ex.: `enum PersistenceError { NotFound, Backend(#[from] …) }`), para a simulação poder reagir. Os binários `client`/`server`, na borda (`main`, setup, loaders), usam **`anyhow`** com `.context(...)`.
- **[FATO] Banir `unwrap`/`expect` em produção** via Clippy no workspace: `[lints.clippy] unwrap_used = "deny"` (e `expect_used`), com `#[allow(..., reason="…")]` escopado onde houver invariante provado. Legítimo em testes/setup/config. — <https://rust.code-maven.com/avoid-unwrap> · <https://users.rust-lang.org/t/how-do-i-ban-unwrap-and-expect-function/57368>
- **[FATO] Newtypes** para dar significado no nível de tipo (evitar `bool`/`u32` ambíguos); **structs com campos privados** (evolução de API sem quebra); **builders** para construção complexa; implementar **`Debug`/`Clone`/`Default`** cedo; convenções `as_`/`to_`/`into_`; tipos **`Send + Sync`** onde possível; funções **validam argumentos**. — <https://rust-lang.github.io/api-guidelines/checklist.html>
  - **[OPINIÃO/CONVENÇÃO] Aplicação direta:** `PlayerId(u64)`, `ZoneId(u32)`, `Tick(u64)`, `ItemId` como newtypes (IDs de rede/persistência — ver anti-padrão do `Entity` no item 2).
- **[FATO] Traits de borda = pontos de troca.** `WorldRepository` como **trait** com impls intercambiáveis (memória/arquivo/DB) é exatamente o padrão repositório; declarar `Send + Sync` para rodar no ECS/async. — guideline de object-safety e Send+Sync: <https://rust-lang.github.io/api-guidelines/checklist.html>
- **[FATO] Módulos / `prelude`:** manter um módulo `prelude` central que reexporta os tipos comuns, reduzindo manutenção de imports e facilitando refactor. — <https://github.com/tbillington/bevy_best_practices>
- **[FATO] Wrap de tipos de 3º (orphan rule):** usar tipo de terceiro como `Resource`/`Event` → **envolver em newtype/tuple struct**, com `Deref`/`DerefMut` para ergonomia. — <https://github.com/sanisoclem/bevy/blob/main/docs/plugins_guidelines.md>

### 4. Renderer como módulo SUBSTITUÍVEL (sem acoplar a `bevy_render`)

- **[FATO] A base técnica já existe no Bevy 0.17:** *"decoupled most of the user-facing renderer API from `bevy_render`"* — dá para usar **cameras, lights, shaders, images, meshes, sprites, text, UI, picking, animation e scenes sem depender de `bevy_render`**; e *"it is now possible for 3rd party custom renderers to act as drop-in replacements for rendering Bevy scenes, without the need to pull in `bevy_render`"*, podendo **omitir os plugins `bevy_render`/`bevy_wgpu`** e escrever o próprio. — <https://bevy.org/news/bevy-0-17/>
- **[OPINIÃO/CONVENÇÃO] Como isolar no nosso projeto:**
  1. **O resto do código nunca importa `bevy_render` diretamente.** Toda a cena é descrita em termos dos tipos "front-end" desacoplados (camera/sprite/mesh/transform) e **tipos de apresentação próprios** no `shared`/`client`.
  2. **O renderer vive num crate `render/`** que implementa um **trait de fronteira nosso** (ex.: `trait Renderer { fn draw(&mut self, scene: &PresentationSnapshot); }` + plugin). A escolha do backend (Bevy padrão ou custom) é um **plugin trocável** no `client`, atrás de feature flag.
  3. **Um `system` de "extract" nosso** converte o estado ECS em `PresentationSnapshot` (dados planos), análogo ao pipeline Extract→Prepare→Queue→Render do Bevy — assim a troca de renderer não toca a simulação. — pipeline/RenderCommand: <https://bevy.org/examples/shaders/custom-phase-item/> · <https://github.com/bevyengine/bevy/discussions/2265>
  4. **Servidor compila sem renderer** (feature `render` off, `MinimalPlugins`), provando o desacoplamento a cada build — o próprio exemplo `headless_renderer` do Bevy mostra como isolar. — <https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs>
- **[OPINIÃO/CONVENÇÃO]** "Abstrair só o que se quer trocar": a fronteira é o **snapshot de apresentação + trait `Renderer`**, não a engine inteira. Não envolver `bevy_ecs`/`bevy_app` em wrappers (não vamos trocá-los) — isso seria over-abstraction cara.

### 5. Como mitigamos o churn pré-1.0 (ver também A1)

- **[FATO]** Bevy quebra API ~a cada 3 meses até 1.0 (sem data); cada release traz guia de migração; não dá para "congelar" limpo a versão. — <https://bevy.org/news/bevy-0-20/> · <https://github.com/bevyengine/bevy/discussions/9789> · A1 (`pesquisa/A1-base-engine.md`)
- **[FATO] `bevy_platform`-core estabiliza primeiro:** a discussão oficial de 1.0 prevê um núcleo (`bevy_ecs`, `bevy_app`, `bevy_reflect`, `bevy_time`, util) que **para de quebrar antes** do resto — concentrar nosso código nesse núcleo reduz churn. — <https://github.com/bevyengine/bevy/discussions/9789>
- **[FATO] Isolamento 100% de plugins:** *"No plugin should import anything from any other plugin ever"* — plugins só importam de si mesmos ou de structs independentes. Isso **limita o raio de uma migração** a plugins pontuais. — <https://github.com/tbillington/bevy_best_practices> · <https://taintedcoders.com/bevy/patterns/plugin-organization>
- **[OPINIÃO/CONVENÇÃO]** Além disso: (a) **pinar versões exatas** de Bevy e plugins no `Cargo.toml`/lockfile e subir em lotes planejados (A1); (b) manter a **superfície de contato com `bevy_render` em um só crate** (item 4) — a parte mais volátil do Bevy; (c) orçar **~1–3 dias de migração a cada release** (A1); (d) `prelude` central para reduzir o nº de arquivos tocados por renames de API.

---

## Armadilhas

- **[FATO]** **`configure_sets` não atravessa schedules** — bug clássico: configurar sets em `Update` e esperar que valham em `FixedUpdate`/`OnEnter`. — <https://bevy-cheatbook.github.io/programming/states.html>
- **[FATO]** **Guardar `Entity` para persistir/rede** quebra entre sessões; usar IDs fortes próprios. — <https://github.com/tbillington/bevy_best_practices>
- **[FATO]** **`.chain()` em schedule inteiro** serializa tudo e mata o paralelismo do ECS; preferir ordenação explícita mínima ou `chain_weak` quando não há dado em comum. — <https://bevy.org/learn/migration-guides/0-19-to-0-20/>
- **[FATO]** **Cheat Book está oficialmente desatualizado** — cruzar sempre com guia de migração e release notes oficiais. — <https://bevy-cheatbook.github.io/>
- **[FATO]** **`anyhow` numa lib** impede o consumidor de distinguir erros — manter `anyhow` só nos binários; `thiserror` em `shared`/traits. — <https://microsoft.github.io/RustTraining/rust-patterns-book/ch10-error-handling-patterns.html>
- **[OPINIÃO/CONVENÇÃO]** **OOP disfarçado em ECS** (lógica em components, `struct Player` monolítico, `Entity` como "ponteiro de objeto") — o anti-padrão nº 1 de quem vem de engines OO.
- **[OPINIÃO/CONVENÇÃO]** **Over-abstraction do renderer:** envolver a engine toda em traits (não só a fronteira de apresentação) custa caro e não paga — abstrair **apenas** o que se troca.
- **[OPINIÃO/CONVENÇÃO]** **Fatiar em muitos crates cedo demais** — o custo de dependências/ergonomia supera o ganho enquanto o projeto é pequeno; fatie quando doer.

---

## Como validamos

- **Grafo de dependências:** CI roda `cargo tree`/`cargo deny` para garantir que **`server`/`client` nunca dependem um do outro** e que **`shared` não importa `bevy_render`** (nem `client`/`server`). Build do `server` com feature `render` off é o teste vivo do desacoplamento (item 4).
- **Lints como gate:** workspace com `unwrap_used = "deny"`, `expect_used = "warn"`, `clippy::all`; `cargo clippy -- -D warnings` no gate de CI (ver AGENTS.md §6: `cargo build && cargo test` headless bloqueia entrega).
- **Erros tipados:** o trait `WorldRepository` e a API de `shared` expõem `enum` de erro com `thiserror`; teste garante que variantes relevantes são distinguíveis (não `anyhow`).
- **Renderer trocável:** um teste de fumaça sobe o `App` do cliente com o plugin de renderer **custom** e com o **Bevy padrão**, provando que a simulação (ECS/`shared`) roda idêntica; servidor headless roda sem nenhum.
- **Churn:** a cada bump de Bevy, medir o raio pelo nº de plugins/crates tocados (meta: migração concentrada no crate `render/` e em plugins isolados, ~1–3 dias — A1).
- **code-intel (MCP)** para toda refatoração de símbolo (renomear newtypes, mover systems entre plugins) com `verify_build` — nunca grep/sed (AGENTS.md §3).

---

*FATO = afirmação com fonte direta (URL). OPINIÃO/CONVENÇÃO = convenção da comunidade Bevy/Rust ou recomendação de projeto minha, não um fato verificável numa única fonte autoritativa. Versões: Bevy 0.20.x (renderer decoupling desde 0.17), tbillington/bevy_best_practices e taintedcoders como referências de comunidade, Rust API Guidelines e anyhow/thiserror (dtolnay) como canônicos de Rust.*
