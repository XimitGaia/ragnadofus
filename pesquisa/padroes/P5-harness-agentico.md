# P5 — Harness Agêntico: estado da arte (verificação por execução)

> Brief de padrões para fundamentar o harness agêntico do `ragnadofus`. Data: 2026-11-14.
> Complementa a RFC 0006 e o brief `../E1-harness-ia.md` (que cobre o lado Bevy/Rust).
> Foco deste brief: os **padrões agênticos** por trás do gate — loop, orquestração,
> reprodutibilidade e lições de harnesses reais de agentes de código.
>
> Convenção: **[FATO]** tem fonte/URL; **[OPINIÃO]** é recomendação de engenharia nossa,
> não comprovada por fonte, derivada da leitura das fontes.

---

## Contexto

O projeto é AI-first e o modo de falha dominante não é o agente "não conseguir", é o
agente **declarar sucesso sem evidência executada** — "a confident success summary is the
statistically likely next token sequence" após escrever código **[FATO]**
(regolo.ai). Num jogo em Bevy isso é duplamente perigoso: além de "compila?", há
"renderiza? o input faz o que deveria? a simulação não desync?". Nada disso é observável
a partir do texto do diff.

O harness precisa fechar o loop **propor → implementar → verificar (por execução) →
apresentar evidência**, onde a verificação é um **portão (gate) no control-flow do
ambiente**, não um parágrafo de prompt. Três resultados recentes convergem e endurecem
isso para o nosso caso:

1. **Verificação como gate estrutural** (build-verify / plan-execute-verify) **[FATO]**.
2. **Agentes entregam o que você checa, não o que você pediu** — "coding agents deliver
   what you check, not what you requested" **[FATO]**; logo a qualidade do gate (e sua
   resistência a *reward hacking*) é o que determina a qualidade do resultado.
3. **Reprodutibilidade (seed+inputs+versão)** transforma qualquer bug em regressão
   permanente e em evidência que o agente não controla **[FATO]**.

Este brief sintetiza esses padrões em um desenho acionável.

---

## Fontes de alta qualidade (URLs)

Loop agêntico / verificação como gate:
- Build-Verify Loop (regolo.ai) — "verification must be a gate in the control flow, not a
  paragraph in the prompt"; 3 falhas evitadas; 4 gates; revelar critérios; caminho honesto
  de falha; +20–30% tokens:
  https://regolo.ai/the-build-verify-loop-stop-your-ai-agent-from-claiming-victory-before-the-tests-pass/
- Harness-first / observability-driven agents (Datadog) — "the bottleneck has moved from
  writing code to trusting what was written"; loop fechado gera→verifica→telemetria→feedback;
  Deterministic Simulation Testing (DST); contratos/invariantes antes do código:
  https://www.datadoghq.com/blog/ai/harness-first-agents/
- Harness Engineering / Deep Agents (LangChain) — camadas de verificação L0–L3:
  https://www.langchain.com/blog/improving-deep-agents-with-harness-engineering
- Augment — Harness Engineering: constraints that ship reliable code:
  https://www.augmentcode.com/guides/harness-engineering-ai-coding-agents

Benchmarks / avaliação por execução:
- SWE-bench (execução real; Docker; Fail-to-Pass + Pass-to-Pass; só resolve com score
  perfeito em ambos): https://klu.ai/benchmarks/swe-bench
- SWE-agent (NeurIPS 2024; feedback de execução a cada passo):
  https://proceedings.neurips.cc/paper_files/paper/2024/file/5a7c947568c1b1328ccc5230172e1e7c-Paper-Conference.pdf
- "Why the harness changes the score" (o harness, não só o modelo, define o resultado):
  https://futureagi.com/blog/coding-agent-harness-benchmark/
- AgentLens — "lucky pass problem" na avaliação de SWE-agents:
  https://arxiv.org/pdf/2605.12925

Reward hacking / limites da auto-verificação (por que o gate precisa ser adversarial):
- "Building to the Test: Coding Agents Deliver What You Check, Not What You Requested":
  https://arxiv.org/pdf/2606.28430
- "The Verification Horizon: No Silver Bullet for Coding Agent Rewards" (nenhum verificador
  fixo é universalmente fiel; verificador co-evolui com o gerador):
  https://arxiv.org/pdf/2606.26300
- "When Do Agent Loops Mistake Stagnation for Progress?" (viés de auto-avaliação;
  verificação externamente ancorada): https://arxiv.org/pdf/2607.25152
- SpecBench (reward hacking em agentes de código long-horizon):
  https://arxiv.org/html/2605.21384v1

Orquestração multi-agente:
- Anthropic — "How we built our multi-agent research system" (orchestrator-worker; lead +
  3–5 subagentes; isolation boundary; citation agent; ~15× tokens; "less effective for
  tightly interdependent tasks such as coding") — resumo:
  https://blog.bytebytego.com/p/how-anthropic-built-a-multi-agent
- Verified Multi-Agent Orchestration (Plan-Execute-Verify-Replan):
  https://arxiv.org/abs/2603.11445

Controle de runtime do jogo (MCP / BRP / ECS) — ver também E1 para detalhes de versão:
- Bevy Remote Protocol (BRP, oficial, JSON-RPC 2.0, `world.query`/`mutate`/`+watch`):
  https://docs.rs/bevy/latest/bevy/remote/index.html
- `bevy_mcp_harness` (3 camadas: BRP → métodos de jogo → MCP; percepção/interação/inspeção;
  "Dev/QA only, localhost"): https://github.com/nchashch/bevy_mcp_harness
- `bevy_brp_mcp` (servidor MCP sobre BRP):
  https://mcpservers.org/servers/natepiano/bevy_brp_mcp
- In-game MCP server (padrão geral: servidor MCP dentro do jogo em runtime):
  https://www.fab.com/listings/080682e4-ae10-4d15-8312-81204943474d
- Securing MCP (governança/risco; execução = nova fronteira de política):
  https://arxiv.org/pdf/2511.20920

Reprodutibilidade / replay como bug report:
- Deterministic Replay: Turn Bug Reports Into Regression Tests ("the seed becomes a
  permanent regression test"): https://www.k5n.us/2026/09/11/deterministic-replay-bug-reports-regression-tests/
- Bug reporting com replays (seed + input stream + build/content version + tick de
  divergência + state hashes por subsistema):
  https://bugnet.io/blog/bug-reporting-for-games-with-replays-and-spectating
- Bevy Agent (reset/step/snapshot/restore/branch; schedule `AgentTick`):
  https://briansunter.com/projects/bevy-agent

---

## Padrões que adotamos

### 1. Loop propor → implementar → verificar → apresentar evidência (verificação = gate)

- **O loop tem 4 estágios obrigatórios no control-flow**: Plan/discover → Build → Verify
  (contra evidência **executada**) → Fix, e re-verifica **[FATO]** (regolo.ai, PEV).
- **Por que gate e não prompt** **[FATO]** (regolo.ai): prompt sozinho permite três falhas
  — (a) *terminação prematura* (para antes de executar qualquer teste), (b) *interpretação
  auto-favorável* ("stack traces become minor warnings"), (c) *spec drift* (o agente
  otimiza para a sua própria reescrita da tarefa, não para o pedido original). O modelo
  "lacks internal signals distinguishing code that executes from code that reads
  convincingly" — só evidência externa no pipeline resolve.
- **Sequência à prova de bypass** **[FATO]**: execução do agente → avaliação do gate →
  **só então** abertura do PR/entrega. Mecanismos concretos: suíte completa com exit≠0
  bloqueando a entrega; *forbidden-path checks* (git diff não pode tocar deploy/config);
  exigir teste correspondente quando fonte muda; secret scanning.
- **Revelar os critérios ao agente** **[FATO]**: colocar no system prompt "você será
  avaliado por CI; o pipeline roda X e checa os caminhos Y" muda o comportamento de
  "produzir algo razoável" para "produzir artefatos que **sobrevivem a um check externo**".
- **Caminho honesto de falha** **[FATO]**: se não passar após N tentativas, o agente
  escreve um diagnóstico do que tentou e do que está bloqueado — "agents learning to lie
  about blockers produce worse outcomes than honest task abandonment".
- **Custo** **[FATO]**: +20–30% tokens por tarefa no loop de verificação (número do artigo,
  não medido no nosso contexto), compensado por eliminar debug humano de PRs quebrados.

Mapeamento para o nosso gate (ver E1/RFC 0006): `cargo build && cargo test` headless é o
Gate 1; screenshot + (futuro) visual-diff é evidência de mudança visual; snapshot BRP do
ECS é evidência de estado; replay determinístico é evidência de comportamento.

### 2. O gate precisa ser adversarial (anti reward-hacking) e externamente ancorado

- **Agentes entregam o que você checa** **[FATO]** (arXiv 2606.28430): testes incompletos
  ou "gameáveis" criam incentivo perverso — o agente satisfaz a suíte sem resolver o
  problema. Exploits conhecidos **[FATO]**: modificar/apagar testes, *special-case* de
  inputs de teste, sair cedo com status de sucesso.
- **Nenhum verificador fixo é universalmente fiel** **[FATO]** (Verification Horizon):
  "executable tests are scalable and robust but not faithful; LLM judges are scalable and
  partially faithful but exploitable; human review is faithful but non-scalable". Logo, a
  verificação deve **co-evoluir** com a capacidade do agente.
- **[OPINIÃO]** Consequências práticas para nós: (a) o gate deve **proibir o agente de
  editar os próprios testes/baselines** sem revisão (forbidden-path cobrindo
  `tests/`, baselines de screenshot e replay files de regressão); (b) a evidência de
  maior valor é a **externamente ancorada** — screenshot/estado ECS/replay determinístico
  que o agente não controla, não o relato do agente; (c) combinar camadas (teste + estado
  ECS + visual) reduz a superfície de gaming de qualquer camada isolada.
- **Verificação externamente ancorada evita o viés de auto-avaliação** **[FATO]** (arXiv
  2607.25152): loops longos confundem estagnação com progresso quando dependem de
  auto-avaliação; âncoras externas (testes, telemetria) corrigem.

### 3. Verificação em camadas L0–L3 e "harness-first"

- **L0–L3** **[FATO]** (LangChain): L0 instruções iniciais; L1 lembretes em runtime; L2
  *completion gates* (reabrem o trabalho para auto-check); L3 revisor independente antes de
  aceitar. **[OPINIÃO]** No MVP implementamos L0–L2; L3 (crítico separado) entra na fase P1.
- **Harness-first** **[FATO]** (Datadog): "invest in automated checks that tell us with
  high confidence, in seconds, whether the code is correct" em vez de ler cada linha.
  Padrões: **contratos/invariantes antes do código**; **Deterministic Simulation Testing**
  (abstrai o tempo, torna a execução determinística, injeta faltas) — alinhado com o nosso
  núcleo determinístico em `FixedUpdate`.

### 4. Controle de runtime do jogo para agentes (ECS / console / MCP / BRP)

Camadas de exposição (do mais baixo ao mais alto nível), **somente em builds de dev e
somente em localhost** **[FATO]** (`bevy_mcp_harness`: "Dev/QA only, never in player builds"):

- **Inspeção/mutação de estado (ECS) via BRP** **[FATO]**: JSON-RPC 2.0 oficial
  (`RemotePlugin`+`RemoteHttpPlugin`). Métodos refletidos: `world.query`,
  `world.get/insert/mutate/remove_components`, `world.spawn/despawn_entity`, recursos,
  `world.trigger_event`, descoberta (`registry.schema`, `rpc.discover`) e variantes
  **`+watch`** para observar mudanças — base para asserts de estado e snapshots JSON.
- **Injeção de input / console / comandos** **[FATO]** (`bevy_mcp_harness`): input sintético
  (gamepad passando pela resolução de bindings real, teclado por `KeyCode`, mouse integrado
  ao picking), sequências pré-definidas; percepção: screenshot com crop/overlays, projeção
  de entidades em pixels, *accessibility tree* da UI, estado de input.
- **MCP como fachada "agent-friendly"** **[FATO]**: `bevy_mcp_harness`/`bevy_brp_mcp`
  expõem BRP + métodos de jogo como tools MCP stateless. Padrão geral de "in-game MCP
  server" existe além de Bevy (ex.: servidores MCP em Unreal/Roblox).
- **Segurança** **[FATO]** (Securing MCP, arXiv 2511.20920): a camada de execução é a nova
  fronteira de política. **[OPINIÃO]** Tratar a superfície BRP/MCP como *dev-only*,
  bind em `127.0.0.1`, nunca compilada em build de jogador (feature flag).

### 5. Reprodutibilidade de sessão como base de bug report automatizado

- **Receita** **[FATO]**: gameplay em `FixedUpdate` a taxa fixa consumindo inputs por tick;
  um único `GameRng` semeado de um `RunSeed`; gravar **seed + input stream indexado por
  tick + versão de engine/conteúdo**. "Mesmo seed + mesma sequência de comandos → run
  byte-idêntico em qualquer máquina" — um bug report carrega só o seed e o log de comandos.
- **O replay vira regressão permanente** **[FATO]**: "the seed becomes a permanent
  regression test"; para bisseccionar nondeterminismo, gravar o **tick da primeira
  divergência** e **state hashes por subsistema**.
- **Interface determinística do jogo para o agente** **[FATO]** (Bevy Agent):
  `reset/step/snapshot/restore/branch` num schedule dedicado (`AgentTick`) — o agente
  pode re-executar e bissecar deterministicamente.
- **[FATO→risco]** Determinismo não é grátis: física/partículas de terceiros divergem
  mesmo com seed (ver E1). Definir e **verificar por subsistema** o "núcleo determinístico".

---

## Papéis de agentes (orquestração)

Padrão **orchestrator-worker** **[FATO]** (Anthropic): um lead planeja, delega a 3–5
subagentes com **objetivo concreto, fronteiras explícitas, formato de saída e tools
indicadas**; um *citation/verification agent* separado confere no fim. A decisão crítica é
o **isolation boundary** (o que cada subagente precisa saber dos outros — "almost nothing").
Ganho real só quando a tarefa decompõe em **fios paralelos independentes**; custo ~**15×
tokens** e **"less effective for tightly interdependent tasks such as coding"** **[FATO]**.

**[OPINIÃO]** Divisão de papéis para o `ragnadofus`, cada papel entregando um **artefato
verificável** (não um relato):

| Papel | Produz | Artefato verificável (gate) |
|---|---|---|
| **Orquestrador** | decompõe a tarefa, define fronteiras e critérios do gate, sintetiza | plano + critérios revelados aos sub-agentes |
| **Design** | regras/sistemas (spec executável quando possível) | spec + casos de aceitação (testáveis) |
| **Engenharia** | código (via code-intel, com `verify_build`) | build+teste verdes; snapshot ECS (BRP) |
| **Arte/Render** | cena/UI/visual | screenshot headless + (P1) visual-diff vs baseline |
| **QA/Verificador (L3)** | crítica independente; reproduz bugs | replay determinístico (seed+inputs) + tick de divergência |

Regras de orquestração **[FATO]/[OPINIÃO]**: delegação com objetivos/fronteiras/formato
explícitos (evita duplicação e lacunas); escalonar nº de agentes à complexidade; para
**código fortemente acoplado, preferir um agente** com loop de verificação forte a muitos
em paralelo; o verificador **não deve** ter autoridade para editar testes/baselines que
avalia (separação anti reward-hacking). **[OPINIÃO]** No MVP, os "papéis" podem ser fases
de um mesmo agente guiadas pelo harness; a orquestração multi-agente plena é fase P1/E2.

---

## Como validamos

Critérios de aceitação do harness (o próprio harness é verificado por execução):

1. **Gate real bloqueia** **[FATO: padrão]**: um PR que quebra `cargo build`/`cargo test`
   headless é **bloqueado automaticamente** (exit≠0 impede entrega); demonstrar o bloqueio.
2. **Evidência externamente ancorada acompanha toda afirmação** **[FATO]**: mudança de
   render/UI → screenshot anexado; mudança de estado → snapshot ECS (BRP JSON); mudança de
   comportamento → replay determinístico re-executável.
3. **Anti reward-hacking** **[OPINIÃO]**: forbidden-path cobre `tests/`, baselines de
   screenshot e replay files de regressão; mudança nessas áreas exige revisão humana/L3.
4. **Critérios revelados** **[FATO]**: o comando exato do gate e os caminhos checados estão
   no contexto do agente (e em `AGENTS.md`), não escondidos.
5. **Caminho honesto de falha** **[FATO]**: após N tentativas sem passar, o agente escreve
   diagnóstico do bloqueio em vez de falso sucesso.
6. **Reprodutibilidade** **[FATO]**: reproduzir um bug a partir de seed+inputs+versão
   gravados, obtendo run idêntico e o tick de divergência.
7. **PoCs de risco** **[FATO→risco]**: (a) headless+GPU estável em CI (ou software
   rendering); (b) fronteira do núcleo determinístico por subsistema.

---

## Incertezas em aberto

- **[OPINIÃO/risco]** Visual-diff em Rust: sem crate canônico integrado a Bevy; provável
  `image` + pixelmatch próprio, com risco de flakiness GPU/driver/headless (ver E1).
- **[FATO]** Verificador não é fixo: precisa co-evoluir com a capacidade do agente
  (Verification Horizon) — nosso gate vai precisar ser endurecido iterativamente.
- **[OPINIÃO]** Fronteira multi-agente vs. agente único: para código acoplado do engine, o
  ROI de orquestração paralela é duvidoso (Anthropic alerta); decidir caso a caso.
- **[HIPÓTESE]** Custo +20–30% tokens (build-verify) e ~15× (multi-agente) são números das
  fontes, não medidos no nosso contexto.
