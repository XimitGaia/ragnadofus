# E1 — Harness Mínimo de Desenvolvimento com IA (Rust/Bevy)

> Brief de fundamentação para RFC. Data: 2026-11-14.
> Premissa: "um agente afirmar que concluiu NÃO substitui evidência de que a build funciona e a experiência visual/interativa está correta."

Convenção: cada item marcado como **[FATO]** tem fonte/URL; **[HIPÓTESE]** é inferência/recomendação de engenharia nossa, não comprovada por fonte.

---

## Problema

Em um fluxo AI-first, o modo de falha dominante não é o agente "não conseguir", é o agente **declarar sucesso sem evidência executada** ("confident completion summaries are the most likely next tokens after coding"). Para um jogo em Bevy isso é duplamente perigoso: além de "a build compila?", há "a cena renderiza? o input faz o que deveria? a simulação não quebrou?". Nada disso é observável a partir do texto do diff.

O harness mínimo precisa então fechar um loop **propor → gerar artefatos → build/testes → capturar evidência visual/estado → apresentar evidência verificável**, onde a verificação é um **portão (gate) no ambiente de execução**, não um parágrafo no prompt. As quatro frentes abaixo (verificação automatizada, exposição do runtime, loop agêntico, determinismo) são as peças desse portão.

---

## Achados (com URLs)

### 1. Verificação automatizada para jogos (Bevy)

- **[FATO] Bevy renderiza headless (sem janela).** Configura-se uma `Camera` para renderizar em um `Image` (render target) e desabilita-se o `WinitPlugin`; usado para testes automatizados / CI / geração de imagem server-side. Exemplo oficial: `examples/app/headless_renderer.rs`.
  - https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs
- **[FATO] API de screenshot programático (padrão componente + observer).** Spawna-se uma entidade com o componente `Screenshot` (ex.: `Screenshot::primary_window()` ou `Screenshot::image(render_target)`), com um observer que dispara `ScreenshotCaptured` quando a imagem está pronta. Há helper `save_to_disk(path)`. A captura é **assíncrona** — não disponível no mesmo frame em que o componente é spawnado; deve-se usar o observer. (Reescrita da API no PR #14833.)
  - https://docs.rs/bevy/latest/bevy/render/view/window/screenshot/struct.Screenshot.html
  - https://docs.rs/bevy/latest/bevy/render/view/window/screenshot/fn.save_to_disk.html
  - https://github.com/bevyengine/bevy/pull/14833
- **[FATO] Crate `bevy_capture` v0.6.0 (10/nov/2026), requer Bevy ^0.19.** Captura frames em modo headless: desabilita `WinitPlugin`, usa `RenderTarget::target_headless()`, anexa `CaptureBundle` a uma câmera e chama `capture.start()` com um encoder. Encoders: `FramesEncoder` (imagens individuais), `GifEncoder` (feature `gif`), MP4 via OpenH264 / FFmpeg. Útil para evidência em vídeo/gif de sequências interativas.
  - https://docs.rs/bevy_capture — https://github.com/jannik4/bevy_capture
- **[FATO] Padrões de teste de integração Bevy (skill "bevy-testing", Bevy 0.20).** Em vez de `App::run()`, os testes chamam `app.update()` repetidamente controlando o tempo; o primeiro `update()` inicializa o clock com delta zero ("aqueça antes de contar ticks de FixedUpdate"). `TimeUpdateStrategy` oferece modos determinísticos (`ManualDuration`/`ManualInstant`, e avanço por timestep fixo). Testes evitam `DefaultPlugins` e compõem só o necessário (`TimePlugin`, `AssetPlugin`, `TaskPoolPlugin`). Regra para async: nunca "sleep 100ms e torça" — usar `block_on(poll_once(&mut task))` ou avançar o app até um predicado de conclusão observável com deadline máximo.
  - https://github.com/chrisgliddon/bevy-skills/blob/main/skills/bevy-testing/SKILL.md
- **[FATO] Visual regression testing é um padrão consolidado**: renderizar → capturar screenshot → comparar pixel-a-pixel contra uma *baseline* versionada, falhando acima de um *threshold* de tolerância, preservando as imagens *actual/expected/diff*. Algoritmo comum: `pixelmatch`. Determinismo de ambiente é essencial ("pin the OS, disable animations, wait for fonts"). Ferramentas de referência (web): Playwright `toHaveScreenshot()`, Vitest 4.x `toMatchScreenshot()`.
  - https://testdino.com/blog/playwright-visual-testing
  - https://keploy.io/blog/community/visual-regression-testing
  - Aplicação direta ao Bevy (render acceptance): capturar `Screenshot`, esperar `ScreenshotCaptured`, comparar contra baseline com tolerância documentada — ver skill bevy-testing acima.

### 2. Exposição do runtime para controle por agentes

- **[FATO] Bevy Remote Protocol (BRP) é oficial (crate `bevy_remote`), Bevy 0.20.1 (28/out/2026).** JSON-RPC 2.0; `RemotePlugin` monta o núcleo sem iniciar transporte, `RemoteHttpPlugin` habilita HTTP (há suporte WASM). Métodos built-in cobrem o ECS refletido: `world.query`, `world.get_components`, `world.list_components`, `world.spawn_entity`, `world.despawn_entity`, `world.insert_components`, `world.mutate_components`, `world.remove_components`, `world.reparent_entities`, recursos (`world.get_resources`, `world.insert_resources`, `world.mutate_resources`, `world.list_resources`), eventos (`world.trigger_event`), descoberta (`registry.schema`, `rpc.discover`) e variantes **`+watch`** (`world.get_components+watch`, `world.list_components+watch`) para observar mudanças. (Porta comum citada por terceiros: `127.0.0.1:15702`.)
  - https://docs.rs/bevy/latest/bevy/remote/index.html
  - https://github.com/bevyengine/bevy/pull/14880 (implementação inicial adotada)
  - https://deepwiki.com/bevyengine/bevy/9.7-remote-protocol-(brp)
- **[FATO] `bevy_mcp_harness` v0.4 (Bevy 0.20) — harness pronto para agentes LLM** inspecionarem/jogarem via MCP. Arquitetura em 3 camadas: (1) BRP (JSON-RPC/HTTP :15702), (2) métodos de jogo customizados sobre BRP, (3) servidor MCP stateless (HTTP :15710) com tools "agent-friendly". Capacidades:
  - Percepção: screenshot com crop/seleção de câmera/overlays de debug; projeção de entidades no espaço de pixels com ordem de profundidade; dump da *accessibility tree* de UI; inspeção do estado de input (teclado/gamepad/mouse).
  - Interação: injeção de input sintético (gamepad passando pela resolução de bindings real), mock de teclado por `KeyCode`, mouse (move/click/wheel) integrado ao pipeline de picking, sequências pré-definidas.
  - Inspeção: queries ECS/componentes via BRP, snapshots de estado de jogo configuráveis, posição/orientação de câmera.
  - Deps: `bevy_remote` (núcleo); opcional `bevy_dev_tools`, `bevy_core_pipeline`, `bevy_pbr`; camada MCP usa `rmcp`, `serde`, `schemars`. **Aviso explícito: Dev/QA only, nunca em builds de jogador; ambos os surfaces só em localhost.**
  - https://github.com/nchashch/bevy_mcp_harness — https://docs.rs/bevy_mcp_harness/latest/bevy_mcp_harness/
  - Alternativa: `bevy_brp_mcp` (natepiano) — servidor MCP sobre BRP: https://mcpservers.org/servers/natepiano/bevy_brp_mcp
- **[FATO] Dev tools para inspeção humana (complementam o agente):**
  - `bevy-inspector-egui` — editor visual ao vivo de componentes via egui. Última versão **0.37.0 (20/jun/2026)**; 0.35.0 (22/out/2025). https://lib.rs/crates/bevy-inspector-egui
  - `bevy_dev_tools` (oficial) — inspeção de entidade/mundo, overlays de debug de render, picking debug. https://docs.rs/bevy/latest/bevy/dev_tools/index.html
  - `bevy_mod_debugdump` — visualizadores de render graph / schedule graph.

### 3. Padrões de harness agêntico (loop propor→implementar→verificar)

- **[FATO] "Build-Verify Loop" como padrão de harness.** Quatro estágios obrigatórios de control-flow: **Plan/discover → Build → Verify (contra evidência executada) → Fix** — e re-verifica. A verificação **deve ser um gate no control-flow, não um parágrafo no prompt** ("verification must be a gate in the control flow, not a paragraph in the prompt"), porque prompt sozinho permite 3 falhas: terminação prematura, auto-interpretação favorável e *spec drift*.
  - Mecanismos concretos: script de CI que roda ANTES de abrir PR (suíte completa; exit≠0 bloqueia PR), forbidden-path checks, secret scanning, exigir que arquivos fonte modificados tenham teste correspondente. Divulgar os critérios de avaliação no system prompt muda o comportamento de "produzir algo razoável" para "sobreviver a um check externo". Deve haver **caminho honesto de falha** (comentário diagnóstico em vez de falso sucesso). Custo: +20–30% tokens por tarefa, compensado por eliminar debug humano de PRs quebrados.
  - https://regolo.ai/the-build-verify-loop-stop-your-ai-agent-from-claiming-victory-before-the-tests-pass/
- **[FATO] Verificação em camadas (L0–L3).** L0 instruções iniciais, L1 lembretes em tempo de execução, L2 completion gates (reabrem o trabalho para auto-check), L3 revisor independente (crítico separado) antes de aceitar.
  - https://www.langchain.com/blog/improving-deep-agents-with-harness-engineering
- **[FATO] Evidência por execução é o padrão de benchmark de agentes de código.** SWE-bench julga correção **rodando testes reais** (aplica o patch, roda suítes "Fail-to-Pass" e "Pass-to-Pass"; só conta como resolvido com score perfeito em ambas) — não análise estática. SWE-agent incorpora feedback de execução a cada passo. "SWE-Agent+" (executar os testes gerados antes de submeter) subiu de 15,9%→18,5%.
  - https://proceedings.neurips.cc/paper_files/paper/2024/file/5a7c947568c1b1328ccc5230172e1e7c-Paper-Conference.pdf
  - https://klu.ai/benchmarks/swe-bench
  - https://arxiv.org/html/2603.17973v1 (TDAD: Test-Driven Agentic Development)

### 4. Determinismo e reprodutibilidade de sessões

- **[FATO] Receita de simulação determinística em Bevy:** sistemas de estado numa ordem fixa em `FixedUpdate` a uma taxa fixa (ex.: 60 Hz), consumindo inputs por tick; `FixedUpdate` permite numerar cada tick ("mesmo período de tempo em todo lugar"). RNG determinístico: um único `GameRng` (ex.: Xoshiro256++) semeado de um `RunSeed`. Para reproduzir: capturar **seed + stream de input indexado por tick + versão de engine/conteúdo + tick onde divergiu**. "Sem fixed timestep, PRNG semeado e ordem de iteração estável, um replay desvia em segundos e deixa de servir como evidência."
  - https://bugnet.io/blog/bug-reporting-for-games-with-replays-and-spectating
  - https://github.com/chrisgliddon/bevy-skills/blob/main/skills/bevy-testing/SKILL.md
- **[FATO] "Bevy Agent": torna o jogo numa simulação determinística expondo `reset`, `step`, `snapshot`, `restore`, `branch`;** sistemas de gameplay rodam num schedule dedicado (`AgentTick`) e consomem input frames gravados.
  - https://briansunter.com/projects/bevy-agent
- **[FATO] Template com verificação de replay determinístico** (sim determinística + replays + verifier wasm) existe como referência de arquitetura.
  - https://github.com/Bread-Heads-Studios/gamebient-bevy-template/pull/9
- **[FATO] Determinismo não é grátis em crates de terceiros:** física (`bevy_rapier`) e partículas (`bevy_hanabi`) têm issues abertas de divergência mesmo com seed fixa — o determinismo precisa ser verificado por subsistema, não assumido.
  - https://github.com/dimforge/bevy_rapier/issues/79 — https://github.com/djeedai/bevy_hanabi/issues/575

---

## Componentes do harness mínimo

Cada item produz um **artefato verificável** (arquivo/log/código de saída) que acompanha a afirmação do agente.

1. **Portão de build/teste (gate, não prompt).** [FATO que o padrão funciona]
   `cargo build` + `cargo test` headless (composição mínima de plugins, `app.update()` com `TimeUpdateStrategy`, sem `DefaultPlugins`). Exit≠0 bloqueia a entrega. Artefato: log de build + relatório de testes.
2. **Captura de evidência visual.** [FATO: APIs existem]
   App headless (Winit off, render→`Image`) + componente `Screenshot`/observer `ScreenshotCaptured` → `save_to_disk`, ou `bevy_capture` para gif/vídeo de uma sequência interativa. Artefato: PNG/GIF por cenário.
3. **Comparação visual contra baseline.** [FATO: padrão web; HIPÓTESE quanto ao tooling exato em Rust]
   Diff pixel-a-pixel (pixelmatch-equivalente) com tolerância documentada; guarda actual/expected/diff. Baselines versionadas no repo. Artefato: diff + pass/fail.
4. **Superfície de controle do runtime.** [FATO]
   `RemotePlugin` + `RemoteHttpPlugin` (BRP) para query/mutação de ECS e `+watch`. Opcionalmente `bevy_mcp_harness`/`bevy_brp_mcp` para o agente injetar input e tirar screenshot via MCP. **Somente localhost, somente builds de dev.** Artefato: snapshots de estado ECS (JSON).
5. **Reprodutibilidade de sessão.** [FATO]
   Gameplay em `FixedUpdate` ordenado + `GameRng` semeado; gravação de `seed + inputs por tick + versão`. Um bug vira um "replay file" que o agente (ou CI) re-executa deterministicamente. Artefato: replay file + tick de divergência.
6. **Loop + caminho honesto de falha.** [FATO]
   Orquestração Plan→Build→Verify→Fix; se bloqueado, gerar diagnóstico em vez de falso sucesso; opcional revisor L3 independente.

---

## Recomendação

Começar pelo **esqueleto executável mínimo**, que já quebra a premissa "o agente disse que terminou":

1. **Gate de build+teste headless** (`cargo build && cargo test`, Bevy ~0.20, `TimeUpdateStrategy` + `app.update()`), bloqueando entrega em falha. [maior ROI, menor custo]
2. **Captura de screenshot headless** via API nativa `Screenshot`/`ScreenshotCaptured` → `save_to_disk`, anexada como artefato a toda mudança que afete render/UI.
3. **BRP ligado em dev** (`RemotePlugin` + `RemoteHttpPlugin`) para o agente consultar/mutar estado ECS e snapshotar — base para asserts de estado além do visual.
4. **Determinismo mínimo**: mover gameplay para `FixedUpdate` + `GameRng` semeado e gravar `seed+inputs+versão`, para que qualquer bug seja reproduzível.

Fases seguintes (não no mínimo): comparação visual contra baseline com tolerância; `bevy_mcp_harness`/MCP para injeção de input e playtest autônomo; revisor L3. Regra transversal: **divulgar ao agente os critérios do gate** e exigir que mudança em fonte venha com teste/evidência correspondente — isso muda o comportamento de "parecer pronto" para "sobreviver ao check".

---

## Questões em aberto

- **[HIPÓTESE] Ferramenta de visual-diff em Rust/CI**: não confirmei um crate Rust canônico e mantido para pixel-diff com tolerância integrado a Bevy (o padrão vem do mundo web/Playwright/Vitest). Pode exigir `image` + implementação própria de pixelmatch, ou chamada a ferramenta externa. **Risco de flakiness**: GPU/driver/headless podem produzir diffs não-determinísticos entre máquinas e CI.
- **[HIPÓTESE] Headless + GPU em CI**: `headless_renderer.rs` ainda usa o render backend (wgpu); rodar em CI pode exigir GPU/`llvmpipe`/software rendering, com impacto em fidelidade e estabilidade das baselines. Precisa de PoC.
- **[FATO→risco] Determinismo de subsistemas**: física/partículas têm divergência conhecida mesmo com seed; precisamos decidir quais subsistemas entram no "núcleo determinístico" verificável.
- **[HIPÓTESE] Versões/compat**: `bevy_mcp_harness` v0.4 mira Bevy 0.20; `bevy_capture` 0.6 mira ^0.19. Alinhar a versão de Bevy do projeto com o ecossistema de tooling é um pré-requisito e muda rápido.
- **[HIPÓTESE] Custo**: +20–30% de tokens por tarefa no loop de verificação (número do artigo build-verify, não medido no nosso contexto).

---

## Fontes

- Bevy headless renderer (exemplo oficial): https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs
- Bevy `Screenshot` (docs): https://docs.rs/bevy/latest/bevy/render/view/window/screenshot/struct.Screenshot.html
- Bevy `save_to_disk` (docs): https://docs.rs/bevy/latest/bevy/render/view/window/screenshot/fn.save_to_disk.html
- Bevy screenshots rewrite (PR #14833): https://github.com/bevyengine/bevy/pull/14833
- bevy_capture (docs): https://docs.rs/bevy_capture — repo: https://github.com/jannik4/bevy_capture
- bevy-skills / bevy-testing (Bevy 0.20): https://github.com/chrisgliddon/bevy-skills/blob/main/skills/bevy-testing/SKILL.md
- Visual regression (Playwright): https://testdino.com/blog/playwright-visual-testing
- Visual regression (Keploy): https://keploy.io/blog/community/visual-regression-testing
- Bevy Remote (docs, 0.20.1): https://docs.rs/bevy/latest/bevy/remote/index.html
- BRP implementação inicial (PR #14880): https://github.com/bevyengine/bevy/pull/14880
- BRP (deepwiki): https://deepwiki.com/bevyengine/bevy/9.7-remote-protocol-(brp)
- bevy_mcp_harness: https://github.com/nchashch/bevy_mcp_harness — docs: https://docs.rs/bevy_mcp_harness/latest/bevy_mcp_harness/
- bevy_brp_mcp: https://mcpservers.org/servers/natepiano/bevy_brp_mcp
- bevy-inspector-egui: https://lib.rs/crates/bevy-inspector-egui
- bevy_dev_tools (docs): https://docs.rs/bevy/latest/bevy/dev_tools/index.html
- Build-Verify Loop (regolo.ai): https://regolo.ai/the-build-verify-loop-stop-your-ai-agent-from-claiming-victory-before-the-tests-pass/
- Harness engineering (LangChain): https://www.langchain.com/blog/improving-deep-agents-with-harness-engineering
- SWE-agent (NeurIPS 2024): https://proceedings.neurips.cc/paper_files/paper/2024/file/5a7c947568c1b1328ccc5230172e1e7c-Paper-Conference.pdf
- SWE-bench: https://klu.ai/benchmarks/swe-bench
- TDAD (Test-Driven Agentic Development): https://arxiv.org/html/2603.17973v1
- Replays como evidência / bug reporting: https://bugnet.io/blog/bug-reporting-for-games-with-replays-and-spectating
- Bevy Agent (reset/step/snapshot/restore/branch): https://briansunter.com/projects/bevy-agent
- Template replay determinístico: https://github.com/Bread-Heads-Studios/gamebient-bevy-template/pull/9
- Determinismo física (bevy_rapier #79): https://github.com/dimforge/bevy_rapier/issues/79
- Determinismo partículas (bevy_hanabi #575): https://github.com/djeedai/bevy_hanabi/issues/575
