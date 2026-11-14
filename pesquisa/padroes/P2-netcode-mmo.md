# P2 — Netcode de estado da arte para MMORPG (servidor autoritativo, clientes Bevy)

> Brief de pesquisa para pagar o **débito DT-01**: na demo usamos `bevy_replicon`
> puro como atalho; o netcode precisa virar estado da arte. Marcação:
> **[FATO]** = afirmação verificável com URL · **[OPINIÃO]** = recomendação de
> engenharia/arquitetura minha, a validar no jogo.

---

## 1. Contexto

`ragnadofus` é um RPG online 2.5D de **mundo contínuo** (não lobby/arena), servidor
**Bevy headless autoritativo**, clientes **Bevy desktop**, mesmo ECS compartilhado
cliente↔servidor, física `avian2d`. Hoje o transporte é `bevy_replicon` puro
(replicação servidor→cliente sem predição, sem reconciliação, sem lag compensation,
sem interest management configurado). Isso basta para uma demo, mas tem três problemas
de MMO que o atalho não resolve:

1. **Responsividade** — movimento do próprio jogador sente o RTT inteiro (sem predição
   client-side, cada passo espera a volta do servidor).
2. **Escala por zona** — replicação ingênua envia o mundo inteiro para todo cliente; um
   mundo contínuo com muitos jogadores por área satura banda e CPU sem
   *interest management*.
3. **Justiça/anti-cheat** — sem autoridade real validando intenção, o cliente "decide"
   demais.

O alvo é um netcode que combine o modelo clássico **Source/Overwatch** (predição +
reconciliação + lag compensation, servidor autoritativo) com **state synchronization à
la Gaffer** (extrapolação por velocidade, priority accumulator, delta compression) e
**interest management espacial** para mundo contínuo — construído sobre o ecossistema
Rust/Bevy, migrando de replicon puro para `lightyear`.

Este brief dialoga com `D1-cliente-servidor.md` (arquitetura cliente-servidor) e
`B3-movimento-navegacao.md` (movimento), e deve embasar uma RFC de netcode.

---

## 2. Fontes de alta qualidade (URLs)

### Clássicos fundadores
- **[FATO]** Valve — *Source Multiplayer Networking* (predição client-side,
  interpolação de entidades, lag compensation por "rewind the world", tickrate):
  https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking
  (resumo/mirror de terceiros: https://heroiclabs.com/blog/netcode-in-games/)
- **[FATO]** Gaffer On Games (Glenn Fiedler) — série *Networked Physics*, índice:
  https://gafferongames.com/categories/networked-physics/
  - *Deterministic Lockstep*: https://gafferongames.com/post/deterministic_lockstep/
  - *Snapshot Interpolation*: https://gafferongames.com/post/snapshot_interpolation/
  - *State Synchronization*: https://gafferongames.com/post/state_synchronization/
  - *Snapshot Compression*: https://gafferongames.com/post/snapshot_compression/
  - *Fix Your Timestep!* (timestep fixo/determinismo):
    https://gafferongames.com/post/fix_your_timestep/

### GDC / estudos de caso AAA
- **[FATO]** Overwatch — *Gameplay Architecture and Netcode* (Tim Ford, GDC 2017);
  deep-dive escrito com números (command frames, rollback, favor-the-shooter):
  https://edgegap.com/blog/game-backend-deep-dive-overwatch-2016-netcode-architecture-rollback
  (vídeo GDC: https://www.gdcvault.com/ — busca "Overwatch Gameplay Architecture and Netcode")
- **[FATO]** Rocket League — *"It IS Rocket Science!"* (Jared Cone, GDC 2018): física
  determinística + rollback para carros. (GDC Vault / YouTube)
- **[FATO]** For Honor — *Working with Deterministic Simulation in For Honor*
  (Jennifer Henry, GDC 2019): lockstep determinístico em produção. (GDC Vault)
- **[FATO]** GGPO (rollback clássico de fighting games), visão geral:
  https://en.wikipedia.org/wiki/GGPO · https://words.infil.net/w02-netcode-p5.html

### Escala / interest management
- **[FATO]** J.-S. Boulanger — *Interest Management for Massively Multiplayer Games*
  (tese McGill; AOI, grid, aura/nimbus, custo):
  https://www.cs.mcgill.ca/~jboula2/thesis.pdf
- **[FATO]** Dynetis Games — *Interest management for MOG* (grid/cell + space map,
  "update many, fetch one", efeito de borda):
  https://www.dynetisgames.com/2017/04/05/interest-management-mog/

### Serialização / compressão
- **[FATO]** pvigier — *Beginner's Guide to Game Networking* (bit packing, quantização,
  delta compression): https://pvigier.github.io/2019/09/08/beginner-guide-game-networking.html
- **[FATO]** snapnet.dev — *Netcode Architectures Part 3: Snapshot Interpolation*:
  https://snapnet.dev/blog/netcode-architectures-part-3-snapshot-interpolation/
- **[FATO]** Choosing the right network model (mas-bandwidth / Fiedler):
  https://mas-bandwidth.com/choosing-the-right-network-model-for-your-multiplayer-game/

### Ecossistema Rust / Bevy
- **[FATO]** `lightyear` — repo e docs (predição+rollback, interpolação, lag
  compensation, inputs, interest management, delta compression, transports):
  https://github.com/cBournhonesque/lightyear · https://docs.rs/lightyear/latest/lightyear/
- **[FATO]** `bevy_replicon` — replicação servidor-autoritativa, *visibility filters*:
  https://docs.rs/bevy_replicon/latest/bevy_replicon/ ·
  https://github.com/projectharmonia/bevy_replicon
- **[FATO]** `bevy_replicon_snap` — extensão com snapshot interpolation + predição
  (referência de que replicon sozinho **não** traz isso de fábrica).

---

## 3. Padrões / estado da arte que miramos

### 3.1 Modelos de sincronização — quando usar cada um

| Modelo | O que trafega | Melhor para | Prós | Contras | Para nosso MMO |
|---|---|---|---|---|---|
| **Deterministic lockstep** | só **inputs** | RTS, fighting 2–4 jogadores | banda ~constante (1 M objetos = banda de 1) **[FATO]** gafferongames.com/post/deterministic_lockstep | exige determinismo total; float é quase impossível de garantir entre máquinas; trava esperando input de todos; não escala >~4 **[FATO]** (mesma fonte) | **Não** como transporte do mundo. Útil só em sistemas fechados e determinísticos (ex.: resolução de combate por turno/tick isolado). |
| **Snapshot interpolation** | estado **visual** completo, interpolado com atraso | entidades remotas (outros jogadores, mobs) | simples, robusto a perda, não precisa simular no cliente | banda alta (ex.: 900 cubos @10pps ≈ 25 KB/snapshot, ~2 Mbit/s) e **atraso** no buffer (~2–3× o intervalo de envio) **[FATO]** gafferongames.com/post/snapshot_interpolation | **Sim**, para **entidades que o jogador não controla** (interp. ~100 ms atrás do servidor). |
| **Client-side prediction + server reconciliation** | inputs do cliente + correções do servidor | **entidade controlada** pelo jogador | input instantâneo; corrige por *replay* dos inputs não confirmados; servidor manda a verdade | "rubber-banding" em misprediction; exige ECS com rollback | **Sim**, para o **próprio avatar** (e habilidades previsíveis). Núcleo do estado da arte. |
| **State synchronization** (Gaffer) | estado **+ velocidade**, extrapolado, com priority accumulator | **muitos objetos** em mundo contínuo | banda controlada por orçamento (ex.: ≤64 updates/pacote em 256 kbit/s); delta encoding dá +1 ordem de grandeza **[FATO]** gafferongames.com/post/state_synchronization | precisa simular nos dois lados; "pops" se velocidade errar; delta exige ack bidirecional + baseline por objeto | **Sim**, como estratégia de envio do mundo (prioriza por relevância/distância). |
| **Rollback / GGPO** | inputs, com *replay* especulativo | peer-to-peer twitchy, 2 jogadores | sensação offline; esconde latência | determinismo + custo de re-simulação por frame; p2p | **Parcial**: a **ideia de rollback+replay** é o motor da reconciliação; o modelo p2p do GGPO não, por sermos servidor-autoritativo. |

**[OPINIÃO]** Para um MMO 2.5D de mundo contínuo, o estado da arte **não é um único
modelo**, é uma **composição por classe de entidade**:
- **Avatar do próprio jogador** → *client-side prediction + server reconciliation*
  (predição + rollback/replay dos inputs pendentes, ao estilo Source/Overwatch).
- **Outros jogadores e mobs (remotos)** → *snapshot interpolation* (renderizados ~X ms
  no passado para suavizar) **ou** *state synchronization* com extrapolação quando a
  banda apertar.
- **Mundo/objetos em massa** → *state synchronization* com **priority accumulator** +
  **delta compression** contra baseline.
- **Combate/hit** (se houver mira) → *lag compensation* no servidor (rewind do mundo
  para o instante do cliente).

### 3.2 Lag compensation — "rewind the world"
**[FATO]** O servidor Source mantém histórico de ~1 s das posições dos jogadores e
**rebobina o mundo** para a perspectiva do atirador na hora de validar o acerto
(https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking).
**[FATO]** Overwatch otimiza isso checando primeiro *bounding volumes* espaciais e só
rebobinando candidatos cujo volume intersecta o raio; acima de ~220 ms de RTT troca por
*dead reckoning* em vez de rebobinar longe demais
(https://edgegap.com/blog/game-backend-deep-dive-overwatch-2016-netcode-architecture-rollback).
**[OPINIÃO]** Em RPG 2.5D muito da ação pode ser "tab-target"/área, reduzindo a
exigência de lag compensation fino; ainda assim precisamos de **histórico de estado por
tick** no servidor para validar skills direcionais e projéteis.

### 3.3 Interest management / AOI / particionamento espacial
**[FATO]** Interest Management limita o tráfego servidor→cliente ao que está na *Area
of Interest* de cada jogador; estratégia comum é **zoneamento/grid**
(https://www.cs.mcgill.ca/~jboula2/thesis.pdf).
**[FATO]** Implementação concreta (Phaser Quest / Dynetis): mundo dividido em células
retangulares (ex.: 34×20 tiles); updates propagam para a célula atual **e vizinhas**
para evitar "efeito de borda"; *space map* (dict de coord→objetos) + conjunto de AOIs
"sujas" para não iterar o grid inteiro; padrão "update many, fetch one"
(https://www.dynetisgames.com/2017/04/05/interest-management-mog/).
**[FATO]** Spatial hashing em grid 2D permite rastrear vizinhos e escalar a dezenas de
milhares de entidades no mesmo mundo
(https://www.cs.mcgill.ca/~jboula2/thesis.pdf; discussão prática em Game Networking
Resources).
**[OPINIÃO]** Alvo: **grid uniforme / spatial hash** (célula ≈ 1–1,5× o raio de visão —
a pesquisa sugere FOV ≈ 1,5× o tamanho da partição como ótimo
https://www.ripublication.com/ijaer17/ijaerv12n19_24.pdf [FATO]), com AOI = célula do
jogador + 8 vizinhas. Em `avian2d` já temos *broadphase* espacial; dá para **reusar a
mesma estrutura** (ou um grid paralelo) para o AOI em vez de manter dois índices.
Mundo contínuo = **sem fronteiras de mapa**; AOI é por célula, não por "mapa".

### 3.4 Anti-cheat por autoridade do servidor
**[FATO]** Predição + reconciliação só funciona porque o servidor dedicado é a "ground
truth"; "não pode existir em p2p/relay" (Overwatch, edgegap.com).
**[OPINIÃO]** Regras de ouro (a codificar na RFC):
- Cliente envia **intenção**, nunca resultado: "quero andar para NE", "quero usar skill
  X no alvo Y" — jamais "estou na posição (x,y)" nem "causei N de dano".
- Servidor **valida tudo**: velocidade máxima, colisão, cooldown, alcance, linha de
  visão, recursos (mana/itens), e **reconcilia** divergências (o cliente prevê, mas o
  servidor corrige).
- **Lag comp = favor-the-shooter com teto** (rewind limitado por RTT; acima do teto,
  descarta — evita que latência vire exploit).
- Validação de taxa: inputs por tick limitados (o servidor processa input do tick N
  **no** tick N — base para rejeitar flood). **[FATO]** lightyear garante que o input do
  cliente para o tick N é processado no tick N no servidor
  (https://github.com/cBournhonesque/lightyear).
- Nunca replicar para o cliente estado que ele não deveria "ver" (fog/AOI) — *visibility
  filters* do replicon também são **anti-wallhack**, não só banda.

### 3.5 Tick rate, delta compression, serialização
**[FATO]** Overwatch roda **command frames de 16 ms (~60 Hz)**, caindo a ~7 ms
(~128 Hz) em modo torneio; o relógio do cliente lidera o servidor por ~½ RTT + 1 frame
(~96 ms a 160 ms de RTT); *time dilation* ajusta o timestep do cliente (16→~15,2 ms)
para encher o buffer de input sob perda; inputs são reenviados em janela deslizante
desde o último ack
(https://edgegap.com/blog/game-backend-deep-dive-overwatch-2016-netcode-architecture-rollback).
**[FATO]** Técnicas de banda (pvigier / Gaffer): **bit packing** (ex.: HP 0–100 em 7
bits, não 32), **quantização** (truncar float para N valores por metro), **delta
compression** (só o que mudou vs. baseline conhecida pelo cliente)
(https://pvigier.github.io/2019/09/08/beginner-guide-game-networking.html;
https://gafferongames.com/post/snapshot_compression/).
**[FATO]** `lightyear` usa **postcard** como serializador default (trocável) e já traz
**delta compression** e **priority-based bandwidth management**
(https://docs.rs/lightyear/latest/lightyear/).
**[OPINIÃO]** Alvo inicial: **tick de simulação 20–30 Hz** (RPG 2.5D não é FPS; 60 Hz é
custo desnecessário por ora, revisável), **send rate ≤ tick**, *snapshot/state sync* com
delta + quantização de posição por tile, interpolação de remotos ~100 ms atrás. Começar
com o default do lightyear e só partir para serialização custom se o profiling exigir.

---

## 4. Caminho de evolução (replicon puro → estado da arte)

**Premissa [FATO]:** `bevy_replicon` sozinho entrega **replicação servidor→cliente** e
**visibility filters**, mas **não** traz predição, reconciliação, interpolação nem lag
compensation — essas vêm de extensões (`bevy_replicon_snap`) ou de `lightyear`
(https://docs.rs/bevy_replicon/; https://github.com/cBournhonesque/lightyear).
**Premissa [FATO]:** `lightyear` 0.31 (Bevy 0.20, Rust ≥1.97) habilita client-prediction
com rollback e snapshot interpolation "com mudança de uma linha", lag compensation,
input buffering resistente a perda, interest management, delta compression e múltiplos
transports (UDP/WebSocket/WebTransport/Steam); e **replica mundo via `bevy_replicon`**
por baixo (https://github.com/cBournhonesque/lightyear; docs.rs/lightyear).

### Fases (incrementais, cada uma com gate de verificação)

**Fase 0 — hoje:** replicon puro, servidor autoritativo, sem predição. *Mantém a demo
andando.*

**Fase 1 — Migrar o transporte para `lightyear`.**
Como ele é construído sobre replicon, a migração preserva o modelo de replicação e
**destrava** predição/interp/lag-comp sem reescrever a camada de replicação. Transport
UDP via MetalLB (já previsto no AGENTS). Verificação: build+teste headless, dois clientes
se enxergam, servidor continua autoritativo.

**Fase 2 — Predição do avatar + reconciliação.**
Ligar client-side prediction no componente de movimento do próprio jogador; servidor
reconcilia por replay de inputs. Verificação visual: input instantâneo local, correção
suave sob latência injetada (tc/netem), sem rubber-band grosseiro.

**Fase 3 — Interpolação de remotos.**
Outros jogadores/mobs renderizados ~100 ms atrás (snapshot interpolation). Verificação:
movimento remoto suave sob jitter.

**Fase 4 — Interest management espacial.**
Grid/spatial-hash (reusando broadphase do `avian2d` quando possível); AOI = célula +
vizinhas; mapear para *visibility filters* do replicon / rooms do lightyear.
Verificação: cliente só recebe entidades da sua AOI (log + banda cai com densidade), e
não há vazamento fora da AOI (anti-wallhack).

**Fase 5 — Lag compensation + endurecimento anti-cheat.**
Histórico de estado por tick no servidor; rewind com teto por RTT; validação de intenção
(velocidade/cooldown/alcance/LoS/recursos) centralizada. Verificação: testes de
servidor rejeitando inputs forjados (teleporte, speed-hack, skill fora de alcance).

**Fase 6 — Otimização de banda.**
Quantização de posição por tile, delta compression afinado, priority accumulator por
relevância (distância/importância), tuning de tick/send rate por profiling.
Verificação: banda por cliente dentro de orçamento sob carga de N jogadores/zona.

**[OPINIÃO]** Ordem deliberada: **responsividade antes de escala antes de micro-banda.**
Predição/reconciliação dá o salto de sensação; interest management dá o salto de escala;
compressão fina é a última milha e só com profiling. Cada fase é um débito pago do DT-01.

---

## 5. Como validamos

- **Gate de build/teste headless** (`cargo build && cargo test`) em cada fase — regra
  inegociável do repo.
- **Latência/perda injetadas** (`tc qdisc netem`) para ver predição, reconciliação e
  interpolação sob 50/100/200 ms + 1–5% loss. Critério: avatar responsivo, remotos
  suaves, correções sem rubber-band grosseiro.
- **Testes de autoridade/anti-cheat**: cliente de teste enviando intenções inválidas
  (teleporte, velocidade acima do máximo, skill fora de alcance/cooldown) → servidor
  **rejeita** e reconcilia. Isto é teste automatizado de servidor, não só manual.
- **Banda e CPU por cliente** medidos com N jogadores por zona (carga sintética):
  interest management reduz tráfego proporcional à densidade; delta/quantização dentro
  do orçamento alvo.
- **Verificação visual in-game** (RFC 0006): screenshot/clip de movimento próprio e
  remoto sob latência — o gate exige evidência, não afirmação.
- **Determinismo onde usado**: se algum subsistema adotar lockstep/replay, teste de
  divergência (mesmos inputs → mesmo estado) em CI, dado o risco de float
  (https://gafferongames.com/post/deterministic_lockstep/).

---

### Nota sobre marcação de confiança
Os **[FATO]** acima vêm de fontes primárias (Valve, Gaffer/Fiedler, deep-dive de
Overwatch, docs do lightyear/replicon, tese McGill). As escolhas de **composição por
classe de entidade**, **ordem das fases**, **tick 20–30 Hz** e **reuso do broadphase
avian2d para AOI** são **[OPINIÃO]** de engenharia deste autor, a confirmar por profiling
e pela RFC de netcode. Nenhuma delas fecha escopo — são recomendações com gatilho de
validação.
