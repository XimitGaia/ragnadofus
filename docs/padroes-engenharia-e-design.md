# Padrões de engenharia e design

Destilado acionável da **busca vasta** (6 trilhas) em fontes de alta qualidade (GDC,
postmortems, docs oficiais, papers, engenharia de referência). Cada seção resume os
padrões que **adotamos como regra** e aponta para o brief completo com fontes em
`../pesquisa/padroes/`. Estes padrões são a referência de qualidade do projeto —
mudanças relevantes passam por RFC.

> Princípio que atravessa tudo: **verificação é um gate, não uma afirmação**; e sempre
> **guardar a receita, não os bytes**. Veja os temas transversais no fim.

---

## 1. Código e arquitetura Rust/Bevy — `pesquisa/padroes/P1-rust-bevy-ecs.md`
- **Grafo de crates unidirecional:** `client → shared`, `server → shared`; **nunca**
  `shared → *` nem `client ↔ server`. `shared` é o ECS do jogo em Bevy puro, **sem
  `bevy_render`**. Verificar com `cargo deny`/`cargo tree` no CI.
- **ECS data-oriented:** components granulares + markers + bundles; lógica em systems,
  nunca em components; sem `struct Player` monolítico.
- **Nunca guardar `Entity` para rede/persistência** — usar newtypes (`PlayerId`, `Tick`).
- **States + SystemSets nomeados**, simulação em `FixedUpdate`. (`configure_sets` não
  atravessa schedules — cuidado.)
- **Erros:** `thiserror` em `shared`/`WorldRepository`; `anyhow` só nos binários.
  Banir `unwrap`/`expect` via Clippy `deny`.
- **Renderer isolado** num crate atrás de um trait `Renderer` + snapshot de apresentação
  (apoiado no desacoplamento de `bevy_render` desde Bevy 0.17). O servidor compila **sem
  renderer**, provando o desacoplamento a cada build.

## 2. Netcode (estado da arte) — `pesquisa/padroes/P2-netcode-mmo.md`
Alimenta o **DT-01**. Alvo: netcode **por classe de entidade**, não modelo único.
- **Avatar próprio:** client-side prediction + server reconciliation (rollback/replay
  dos inputs pendentes).
- **Outros jogadores/mobs:** snapshot interpolation (~100 ms atrás); state sync com
  extrapolação quando a banda apertar.
- **Mundo em massa:** state sync com priority accumulator + delta compression.
- **Combate direcional:** lag compensation ("rewind the world") com teto por RTT.
- **Escala:** interest management em **grid/spatial-hash** (AOI = célula + 8 vizinhas),
  reusando o broadphase do `avian2d`; vira visibility filter (também é anti-wallhack).
- **Anti-cheat:** cliente envia só **intenção**; servidor valida tudo. Tick 20–30 Hz.
- **Migração DT-01:** replicon puro → **`lightyear` 0.31** (compatível com Bevy 0.20,
  construído sobre `replicon`) em **6 fases incrementais**, cada uma com gate de
  verificação (netem p/ latência, teste rejeitando input forjado, evidência in-game).

## 3. Game design sistêmico — `pesquisa/padroes/P3-game-design-sistemico.md`
- **Os jogadores SÃO a simulação econômica.** Não simular ecologia autônoma (a de
  Ultima Online colapsou). Economia emerge de produção + **destruição** de jogadores —
  **todo item craftado precisa de razão para ser consumido ou perdido** (= o *sink* da
  RFC 0004 não é opcional).
- **Mundo vivo "trapaceado" com outputs legíveis:** timers baratos + LOD + diretor de
  eventos que lê o estado do servidor. Nunca gastar compute em simulação invisível.
- **Progressão horizontal, aprendizado primeiro** (Koster): teto de poder baixo;
  recompensas mudam *como* você joga, não só os números.
- **Exploração + tática em camadas** conectadas por transição legível: combate
  instanciado em grade derivada de onde o encontro começou (resolve Ragnarok×Dofus).
- **Poucos pilares rígidos + non-goals como firewall de escopo** (evita a diluição de
  identidade de SWG-NGE/WoW).

## 4. Teste, validação e determinismo — `pesquisa/padroes/P4-teste-validacao-determinismo.md`
- **Pirâmide de validação onde verificação é gate**, tudo via crate **xtask**
  (`cargo run -p ci`) para "local == CI".
- **Núcleo determinístico isolado do render:** `FixedUpdate` 64 Hz, RNG semeado e
  *forkado por entidade* (`bevy_rand`/WyRand), ordem estável de queries (sem iterar
  `HashMap`), sistemas em `.chain()`.
- **Testes headless** (`MinimalPlugins`, `cargo-nextest`): **property-based** (proptest)
  para invariantes de economia/servidor; **snapshot de estado** (`insta`); **golden
  replay** de `(seed, inputs)`.
- **Netcode:** convergência in-process (`ServerTestAppExt`); se vier rollback, SyncTest +
  checksum vira gate.
- **Visual regression** em workflow separado (xvfb+lavapipe, métrica perceptual
  `dssim`/`image-compare`, baseline por SO).

## 5. Harness agêntico (AI-first) — `pesquisa/padroes/P5-harness-agentico.md`
- **Loop Plan → Build → Verify → Fix** com verificação como **gate estrutural à prova de
  bypass** (execução → gate → só então entrega), nunca texto de prompt.
- **Runtime exposto a agentes** via **BRP** (JSON-RPC sobre o ECS) + fachada **MCP**
  (`bevy_mcp_harness`), dev-only/localhost.
- **Critérios do gate revelados ao agente**; caminho honesto de falha.
- **⚠️ Reward-hacking:** "agentes entregam o que você verifica, não o que você pediu".
  Proteger testes/baselines com **forbidden-path** (o agente não edita o que o avalia).
- **Papéis** (design/engenharia/arte/QA), cada um entregando artefato verificável,
  orquestração orchestrator-worker. Orquestração paralela tem **ROI duvidoso em código
  acoplado do engine** — usar com critério.

## 6. Representação compacta / PCG (a alma técnica) — `pesquisa/padroes/P6-representacao-compacta-pcg.md`
P1-de-prioridade (pós-demo), mas **não inviabilizar agora**:
- **Guardar a receita, não os bytes:** seed + regras versionadas + grafo de operações
  (validado por No Man's Sky e `.kkrieger`).
- **Mundo como índices:** tiles + paleta indexada (casa com os palette shaders de A2).
- **Content-addressable storage** (dedup + integridade de graça); **animação paper-doll**.
- **WFC** para layout local com **âncoras autorais** contra repetição (é NP-hard, pode
  dar contradição — restringir).
- **Persistência diferencial** (baseline por seed + delta de ações) **embutida na forma
  do `WorldRepository`** desde já (cruza com D4.3).
- **Medir 4 budgets separados:** executável / assets / mundo-estático / mundo-dinâmico,
  com a RAM de runtime à vista.

---

## Temas transversais (aparecem em várias trilhas)
1. **Contrato de determinismo `(seed, rules_version, engine_version)`** — aparece em P4
   e P6. Precisa ser **testado**, não assumido (float cross-platform, ordem de iteração,
   churn do Bevy quebram seeds silenciosamente). É a maior incerteza técnica repetida.
2. **Verificação como gate** — P4 e P5. O gate co-evolui e precisa endurecer contra
   reward-hacking.
3. **Sink econômico obrigatório** — P3 confirma e eleva a questão aberta da RFC 0004.
4. **Interest management reusa o broadphase do `avian2d`** — P2 cruza com a física da
   RFC 0003.

## Impacto nas decisões já tomadas
- **RFC 0003/0005:** adotar interest management em grid reusando `avian2d`; tick 20–30 Hz.
- **RFC 0004:** o *sink* deixa de ser "questão aberta" e vira **requisito de design**.
- **RFC 0006:** adicionar **forbidden-path** (anti reward-hacking) e o crate **xtask**
  como forma canônica de rodar o gate; definir a fronteira do núcleo determinístico.
- **D4.3 (`WorldRepository`):** já modelar **delta sobre baseline por seed**.
- **DT-01:** o caminho de migração para `lightyear` tem **6 fases** com gate por fase.
