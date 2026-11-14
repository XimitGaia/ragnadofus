# D1 — Arquitetura Cliente-Servidor (RFC brief)

> MMORPG 2.5D em Rust, MVP ONLINE desde o início. Servidor autoritativo dono do estado do mundo; clientes Bevy sincronizam. Loop do MVP = coleta/crafting numa zona feita à mão.
> Data: 2026-11-14. Convenção: **[FATO]** = afirmação com fonte/URL; **[HIPÓTESE]** = inferência/recomendação de projeto, não comprovada por fonte.

---

## 1. Problema

Precisamos fundamentar a RFC de arquitetura cliente-servidor para um MMORPG 2.5D com servidor **autoritativo** desde o MVP. Decisões a tomar:

- Onde fica a autoridade, como roda o tick/simulação no servidor, como o cliente reconcilia e qual anti-cheat mínimo.
- Como particionar o mundo (zonas/instâncias/sharding) e como isso mapeia para processos/servidores — mesmo que o MVP tenha **uma única zona feita à mão**.
- Qual stack Rust de networking + ECS + serialização adotar, considerando maturidade em 2026.
- Como sincronizar estado (replicação, interest management, snapshot vs delta, predição de movimento) com o **mínimo viável** para poucos jogadores numa zona.
- Como persistir estado autoritativo de forma durável **sem travar o tick**.

Restrição de projeto: o MVP é pequeno (poucos jogadores, uma zona), mas a arquitetura não deve fechar portas para crescer depois.

---

## 2. Achados (com URLs)

### 2.1 Servidor autoritativo, tick e reconciliação

- **[FATO]** No modelo servidor-autoritativo o **cliente só envia inputs/intenção** ("mover com esse input", "usar habilidade"); o servidor valida contra as regras, avança a simulação e replica o estado aceito de volta. Isso é o que previne trapaça (clientes não alteram o estado diretamente). Heroic Labs — <https://heroiclabs.com/blog/netcode-in-games/index.html> ; AccelByte — <https://accelbyte.io/blog/server-authoritative-vs-client-authoritative-architecture>
- **[FATO]** O servidor roda um **game loop em tick fixo** (passo de simulação): lê inputs, atualiza o mundo, envia o novo estado. Tick rates típicos vão de **1 a 60 Hz**; shooters rápidos precisam de updates frequentes, jogos mais lentos rodam menos ticks. Heroic Labs — <https://heroiclabs.com/blog/netcode-in-games/index.html> ; Crux — <https://crux.supercraft.host/blog/what-is-an-authoritative-game-server/>
- **[FATO]** **Client-side prediction + reconciliation**: o cliente aplica o input localmente na hora, guarda a lista de inputs não confirmados; quando chega o estado autoritativo, reseta para esse estado e **re-simula (replay)** os inputs pendentes até o tick atual — idealmente de forma invisível. O conjunto predição+rollback+re-simulação chama-se "latency compensation". Heroic Labs — <https://heroiclabs.com/blog/netcode-in-games/index.html> ; Daniel Jiménez Morales — <https://danieljimenezmorales.github.io/2025-06-20-client-side-prediction-and-server-reconciliation/>
- **[FATO]** **Entity interpolation**: entidades remotas (outros jogadores) são desenhadas "ligeiramente no passado", entre duas posições conhecidas, para suavizar. Heroic Labs — <https://heroiclabs.com/blog/netcode-in-games/index.html>
- **[FATO]** **Anti-cheat**: a autoridade de servidor impede o cliente de mudar o estado diretamente, mas **não** resolve aimbot/wallhack — estes exigem anti-cheat dedicado. Heroic Labs — <https://heroiclabs.com/blog/netcode-in-games/index.html>

### 2.2 Particionamento de mundo

- **[FATO]** Duas técnicas principais: **Zoning** (partição do espaço do mundo em zonas, cada uma sob responsabilidade de um recurso de computação) e **Sharding** (clonar o mundo e distribuir usuários para absorver carga). Sharding tem teto físico por shard; zoning permite mapear zona→processo/servidor. Game-Ace — <https://game-ace.com/blog/mmorpg-games-and-how-to-develop-them/> ; Medium/Nguyen — <https://medium.com/nguyen/a-single-shard-continuous-universe-one-world-no-boundaries-f9fee0c7d7f0>
- **[FATO]** **Instâncias**: a abordagem mais simples divide o mundo em zonas; quando uma zona enche, cria-se uma nova instância dela (sem visibilidade inter-instância). OneManMMO/Medium — <https://onemanmmo.com/sharding-solves-queues-and-creates-loneliness/>
- **[FATO]** **Interest management (Area of Interest / AoI)**: filtra quais entidades cada cliente recebe — só entidades dentro do raio de visibilidade/célula de zona. No modelo baseado em região, o mundo é quebrado em regiões gerenciadas por peers/servidores; ao mover-se, o avatar troca de região e passa a receber dados do servidor correspondente. GameDev.net — <https://gamedev.net/forums/topic/506375-mmo-server-cluster-architecture/> ; ResearchGate (P2P/MMO spatial locality) — <https://www.researchgate.net/publication/234111686_Using_Spacial_Locality_and_Replication_to_Increase_P2P_Network_Performance_in_MMO_Games>
- **[FATO]** Sharding resolve filas e densidade mas cria "ghost town": comunidades/guildas fragmentadas. Trade-off social vs. estabilidade técnica. OneManMMO — <https://onemanmmo.com/sharding-solves-queues-and-creates-loneliness/>
- **[FATO]** Fleet managers (ex.: Edgegap) são usados para orquestrar instâncias/zonas como processos escaláveis sob demanda. Edgegap — <https://edgegap.com/blog/how-mmo-games-architecture-scales-with-a-smart-fleet-manager>

### 2.3 Stack Rust (maturidade 2026)

- **[FATO]** **lightyear 0.31** (compatível com Bevy 0.20, Rust ≥1.97.1; ~44,5k downloads/mês): biblioteca server-client para Bevy voltada a jogos **server-autoritativos**. Features: client-side prediction com **rollback**, snapshot interpolation, input delay, bandwidth/priority, lag compensation, world replication **via bevy_replicon** com interest management. É **transport-agnostic** (UDP, e WebSocket/Steam/WebTransport **via aeronet**); serialização default **postcard**. GitHub — <https://github.com/cBournhonesque/lightyear> ; Lib.rs — <https://lib.rs/crates/lightyear_sync>
- **[FATO]** **bevy_replicon 0.44.2** (set/2026; ~22,2k downloads/mês): crate de **replicação server-autoritativa** para Bevy. Abstrai a lógica para suportar singleplayer, cliente, servidor dedicado e listen-server com o mesmo código. Suporta **visibilidade por cliente / rooms** (fog of war) para reduzir banda; backends de mensageria: `bevy_replicon_renet`, `bevy_replicon_renet2`, `aeronet_replicon`. Extensão `bevy_replicon_attributes` adiciona visibilidade por atributos. Lib.rs — <https://lib.rs/crates/bevy_replicon> ; GitHub — <https://github.com/simgine/bevy_replicon> ; attributes — <https://github.com/UkoeHB/bevy_replicon_attributes>
- **[FATO]** **renet 2.0.1** (~38k downloads recentes): lib server/client com canais **ReliableOrdered / ReliableUnordered / Unreliable**, autenticação+criptografia via `renet_netcode`, transporte **UDP** (netcode) e `renet_steam`. Tem `bevy_renet`. Existe fork **renet2** com WebTransport/WASM (usado como backend pelo replicon). GitHub — <https://github.com/lucaspoffo/renet> ; Lib.rs — <https://lib.rs/crates/bevy_replicon>
- **[FATO]** **aeronet 0.22.0** (29/out/2026, Bevy 0.20): conjunto de crates **Bevy-native** de IO/transporte, com estado de rede representado como **entidades** (queryável). Ecossistema: `aeronet_channel` (MPSC in-memory), `aeronet_websocket` (TCP), `aeronet_webtransport` (**WebTransport sobre QUIC**), `aeronet_steam`, `aeronet_iroh` (P2P), `aeronet_replicon`. É a **camada de IO** usada pela lightyear. Lib.rs — <https://lib.rs/crates/aeronet> ; docs.rs — <https://docs.rs/aeronet/latest/aeronet/> ; GitHub — <https://github.com/aecsocket/aeronet>
- **[FATO]** **quinn 0.11.19** (30/set/2026): implementação **QUIC** pura em Rust, async sobre **tokio**, estável, **production-ready e battle-tested** (>86M downloads totais). `quinn-proto` é a máquina de estados determinística sem I/O (para loops customizados). É o QUIC por baixo de WebTransport. docs.rs — <https://docs.rs/crate/quinn/latest> ; GitHub — <https://github.com/quinn-rs/quinn> ; Lib.rs — <https://lib.rs/crates/quinn-proto>
- **[FATO]** **laminar**: lib de UDP semi-confiável (reliability sobre UDP); listada no ecossistema de networking Rust, porém hoje **menos ativa** que renet/lightyear/aeronet. Are We Game Yet — <https://arewegameyet.rs/ecosystem/networking/>
- **[FATO]** **Bevy headless como servidor**: usa-se `App::new().add_plugins(MinimalPlugins)` (sem janela/render) + `LogPlugin`; `ScheduleRunnerPlugin::run_loop(...)` define a taxa; **`FixedUpdate` roda em tick fixo (default 64 Hz)**, apropriado para simulação/lockstep. bevy_replicon roda em singleplayer/cliente/servidor com o mesmo ECS. Bevy docs — <https://docs.rs/bevy/latest/bevy/prelude/struct.Time.html> ; Hank Ruiger — <https://www.hankruiger.com/posts/adding-networked-multiplayer-to-my-game-with-bevy-replicon/>

### 2.4 Persistência sem travar o tick

- **[FATO]** Padrão: **estado vivo em memória do servidor**, com persistência separada por classe de dado — respostas de economia **síncronas**, estado de mundo escrito **em background num timer (write-behind)**. Assim o tick não espera o DB. HN discussion (MMO source of truth / I/O bottlenecks) — <https://news.ycombinator.com/item?id=37702632>
- **[FATO]** **Redis = dados reconstruíveis** a partir do Postgres (caches, locks, presença) — nunca estado autoritativo; **Postgres = source of truth** durável. Redis só é vantajoso quando vários processos compartilham autoridade. HN — <https://news.ycombinator.com/item?id=37702632> ; packagemain.tech — <https://packagemain.tech/p/persistent-multiplayer-state-without>
- **[FATO]** **Saving em camadas (tiered)**: crítico (inventários) vs. importante (posições) vs. periódico (estado de zona). Não dá para persistir a cada mudança — é preciso **dump periódico** ao DB (ex.: loop autoritativo a 20 Hz + "persistence tick" separado). game-ace — <https://game-ace.com/blog/mmorpg-games-and-how-to-develop-them/> ; sorceress.games — <https://sorceress.games/blog/realm-how-to-make-an-mmo-browser-server-loop-2026>
- **[FATO]** **SQLite** é discutido como design de persistência para game state em projetos MMO menores/single-process (debate em aberto sobre adequação). opensro issue — <https://github.com/opensro-dev/opensro/issues/2319>
- **[FATO]** Postgres no mesmo host adiciona processo + round-trip e **não elimina a espera** do flush durável; a "física" de commits duráveis exige um flush em algum lugar — por isso o write-behind/batching. packagemain.tech — <https://packagemain.tech/p/persistent-multiplayer-state-without>

---

## 3. Alternativas de stack (tabela)

| Camada | Opção | Versão (2026) | Papel | Maturidade | Observação |
|---|---|---|---|---|---|
| Replicação ECS | **bevy_replicon** | 0.44.2 | Replicação server-autoritativa + visibilidade/rooms | Boa, ativa | Mesmo código p/ single/cliente/servidor; base p/ lightyear |
| Framework netcode | **lightyear** | 0.31 | Prediction+rollback, interp, lag comp, interest mgmt | Boa, muito ativa | Usa replicon p/ replicação e aeronet p/ IO; server-auth "pronto de fábrica" |
| Transporte confiável UDP | **renet / renet2** | 2.0.1 | Canais confiáveis/não, netcode auth+cripto | Madura | renet2 adiciona WebTransport/WASM; backend do replicon |
| Transporte Bevy-native | **aeronet** | 0.22.0 | IO: WebTransport(QUIC)/WS/Steam/channel | Boa, ativa | Estado de rede como entidades; usado pela lightyear |
| QUIC base | **quinn** | 0.11.19 | QUIC async (sob WebTransport) | **Production-ready** | Battle-tested, tokio |
| UDP semi-confiável | laminar | — | Reliability sobre UDP | Menos ativa | Fallback/legado |
| ECS servidor | **Bevy headless** (MinimalPlugins) | Bevy 0.20 | Simulação server-side em `FixedUpdate` | Madura | 64 Hz default; mesmo ECS do cliente |
| Serialização | **postcard** (default lightyear) / bincode | — | Wire format compacto | Madura | — |
| Persistência durável | **PostgreSQL** | — | Source of truth (inventário/recursos/mundo) | Madura | Write-behind/batch fora do tick |
| Cache/estado efêmero | Redis | — | Caches/locks/presença reconstruíveis | Madura | Opcional no MVP single-process |
| Persistência embarcada | SQLite | — | Alternativa single-process | Madura | Simplicidade p/ MVP |

---

## 4. Arquitetura mínima para o MVP

**[HIPÓTESE / recomendação de projeto]** — Para "poucos jogadores numa zona feita à mão, coleta/crafting":

**Stack recomendada:**

- **Servidor = Bevy headless** (`MinimalPlugins` + `ScheduleRunnerPlugin`), simulação em `FixedUpdate`. **Um único processo = a única zona do MVP.** [FATO que viabiliza: Bevy headless + FixedUpdate 64 Hz — docs/Hank Ruiger]
- **Replicação = bevy_replicon** (server-autoritativo), para ter o mesmo código ECS no cliente e no servidor e visibilidade por cliente já embutida. [FATO: replicon abstrai single/cliente/servidor]
- **Netcode = lightyear 0.31** por cima (se quiser prediction+rollback+interpolation de fábrica) **ou** apenas replicon+renet se quisermos o mínimo absoluto. [FATO: lightyear fornece prediction/interp; usa replicon+aeronet]
- **Transporte = renet2 (UDP/netcode) ou aeronet_webtransport (QUIC via quinn)**. Para desktop-first, renet/UDP é o caminho mais simples; WebTransport só se precisarmos de cliente web. [FATO: renet UDP maduro; quinn production-ready]
- **Serialização = postcard** (default) ou bincode. [FATO]
- **Persistência = PostgreSQL como source of truth**; estado vivo em memória do servidor; **save write-behind em timer** (ex.: a cada N s) + save síncrono em eventos críticos (craft concluído, item coletado commitado). SQLite é alternativa aceitável enquanto for single-process. [FATO: padrão memória+write-behind; SQLite para single-process]

**Loop lógico mínimo (coleta/crafting):**
1. Cliente envia **intenção** ("coletar nó X", "craftar receita Y"), nunca o resultado. [FATO: só inputs]
2. Servidor valida (posição, cooldown, recursos no inventário), aplica no ECS no próximo tick, replica estado. [FATO: servidor valida+avança+replica]
3. **Movimento**: client-side prediction + reconciliation para o próprio avatar; interpolation para os outros. [FATO]
4. **Interest management**: mínimo = replicar tudo da única zona para todos (poucos jogadores). Deixar o hook de visibilidade/rooms do replicon pronto, mas não ativar AoI por raio ainda. [FATO: replicon tem visibilidade/rooms; HIPÓTESE: desnecessário ativar no MVP]
5. **Snapshot vs delta**: replicon faz replicação por mudança de componente (delta/changed) automaticamente — suficiente; snapshots completos só no join. [FATO: replicon detecta componentes alterados]
6. **Anti-cheat mínimo**: validação server-side de toda ação + rate-limit de inputs. Sem anti-cheat de cliente no MVP. [FATO: server-auth cobre state, não aimbot — irrelevante p/ coleta/crafting]

**Mapeamento zona→processo:** no MVP, 1 zona = 1 processo. A arquitetura mantém a porta aberta para **zoning** (cada zona futura = um processo/servidor, handoff na borda) sem precisar de sharding no curto prazo. [FATO: zoning mapeia zona→recurso; HIPÓTESE: handoff fora de escopo do MVP]

---

## 5. Questões em aberto

1. **lightyear vs. replicon "puro"**: adotar lightyear 0.31 (prediction/rollback/interp de fábrica, mais acoplamento e superfície de API) ou começar só com bevy_replicon+renet e implementar predição de movimento à mão? Risco: lightyear acelera, mas é a peça de maior churn de API (0.x, segue Bevy de perto).
2. **Transporte UDP vs QUIC/WebTransport**: desktop-only (renet/UDP, mais simples) ou já mirar cliente web (aeronet_webtransport/quinn)? Define se WASM entra cedo.
3. **Determinismo da simulação**: prediction+rollback assume simulação determinística o suficiente. Coleta/crafting é quase trivial, mas física/movimento em Bevy precisa ser confirmada como determinística (ou usar reconciliação por correção de posição, não lockstep). [HIPÓTESE]
4. **Postgres vs SQLite no MVP**: SQLite single-process é mais simples agora, mas migrar para Postgres quando houver múltiplos processos/zonas tem custo. Decidir cedo se o eixo de crescimento (multi-processo) justifica já começar em Postgres.
5. **Cadência de persistência vs. durabilidade**: definir janela de perda aceitável (ex.: até N s de progresso) para calibrar o write-behind; o que é "crítico-síncrono" (inventário pós-craft) vs. "periódico" (posição/estado de nós).
6. **Versões Bevy/ecossistema**: lightyear 0.31↔Bevy 0.20, aeronet 0.22↔Bevy 0.20, replicon 0.44↔? — travar um conjunto de versões compatível entre si é pré-requisito e muda a cada release do Bevy.
7. **Redis sim/não no MVP**: provavelmente desnecessário single-process; só entra quando houver múltiplos processos compartilhando autoridade. [FATO sustenta; decisão em aberto]

---

## 6. Fontes

- Heroic Labs — Netcode in Games: <https://heroiclabs.com/blog/netcode-in-games/index.html>
- Heroic Labs — How to Make an Online Multiplayer Game: <https://heroiclabs.com/blog/how-to-make-an-online-multiplayer-game/>
- AccelByte — Server- vs Client-Authoritative: <https://accelbyte.io/blog/server-authoritative-vs-client-authoritative-architecture>
- Crux — What Is an Authoritative Game Server: <https://crux.supercraft.host/blog/what-is-an-authoritative-game-server/>
- Daniel Jiménez Morales — Client-Side Prediction & Server Reconciliation: <https://danieljimenezmorales.github.io/2025-06-20-client-side-prediction-and-server-reconciliation/>
- lightyear (GitHub): <https://github.com/cBournhonesque/lightyear>
- lightyear_sync (Lib.rs): <https://lib.rs/crates/lightyear_sync>
- lightyear_replication (Lib.rs): <https://lib.rs/crates/lightyear_replication>
- bevy_replicon (Lib.rs): <https://lib.rs/crates/bevy_replicon>
- bevy_replicon (GitHub): <https://github.com/simgine/bevy_replicon>
- bevy_replicon_attributes: <https://github.com/UkoeHB/bevy_replicon_attributes>
- renet (GitHub): <https://github.com/lucaspoffo/renet>
- aeronet (Lib.rs): <https://lib.rs/crates/aeronet>
- aeronet (docs.rs): <https://docs.rs/aeronet/latest/aeronet/>
- aeronet (GitHub): <https://github.com/aecsocket/aeronet>
- quinn (docs.rs): <https://docs.rs/crate/quinn/latest>
- quinn (GitHub): <https://github.com/quinn-rs/quinn>
- quinn-proto (Lib.rs): <https://lib.rs/crates/quinn-proto>
- Are We Game Yet — Networking: <https://arewegameyet.rs/ecosystem/networking/>
- Bevy Time docs: <https://docs.rs/bevy/latest/bevy/prelude/struct.Time.html>
- Hank Ruiger — Bevy Replicon multiplayer: <https://www.hankruiger.com/posts/adding-networked-multiplayer-to-my-game-with-bevy-replicon/>
- Game-Ace — MMORPG development: <https://game-ace.com/blog/mmorpg-games-and-how-to-develop-them/>
- OneManMMO — Sharding Solves Queues and Creates Loneliness: <https://onemanmmo.com/sharding-solves-queues-and-creates-loneliness/>
- Medium/Nguyen — Single Shard Continuous Universe: <https://medium.com/nguyen/a-single-shard-continuous-universe-one-world-no-boundaries-f9fee0c7d7f0>
- GameDev.net — MMO server cluster architecture: <https://gamedev.net/forums/topic/506375-mmo-server-cluster-architecture/>
- Edgegap — MMO architecture & Fleet Manager: <https://edgegap.com/blog/how-mmo-games-architecture-scales-with-a-smart-fleet-manager>
- ResearchGate — Spatial locality & replication in P2P MMO: <https://www.researchgate.net/publication/234111686_Using_Spacial_Locality_and_Replication_to_Increase_P2P_Network_Performance_in_MMO_Games>
- HN — MMO Architecture: Source of truth, Dataflows, I/O bottlenecks: <https://news.ycombinator.com/item?id=37702632>
- packagemain.tech — Persistent multiplayer state without chaos: <https://packagemain.tech/p/persistent-multiplayer-state-without>
- sorceress.games — How to Make an MMO (Server Loop 2026): <https://sorceress.games/blog/realm-how-to-make-an-mmo-browser-server-loop-2026>
- opensro — game state persistence / SQLite design: <https://github.com/opensro-dev/opensro/issues/2319>

> Nota: a lib.rs/crates.io e algumas páginas (Medium, scs.stanford PDF) retornaram 403/binário em fetch automatizado; os dados de versão foram extraídos de Lib.rs/docs.rs/GitHub acessíveis e dos snippets de busca. Confirmar as versões exatas contra crates.io antes de fixar o Cargo.toml.
