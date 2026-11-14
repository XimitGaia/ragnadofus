# Plano de decisões — rumo a uma demo simples

Consolida **todos os pontos de análise em aberto** (das 6 RFCs P0) numa sequência que
vamos percorrer **uma pergunta de cada vez, com opções**. Cada decisão tomada é
registrada aqui (vira o log de decisões da demo).

**Ordem é por dependência:** respostas de cima podam/alteram as de baixo (ex.: se a
demo for local, pulamos por ora todas as perguntas de rede).

Legenda de status: ⬜ pendente · 🔵 em discussão · ✅ decidido

---

## Bloco 0 — Escopo da demo
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D0.1 | **O que a demo simples precisa mostrar?** | ✅ | **Andar + online** (2 clientes se vendo numa zona, servidor autoritativo; sem crafting) |
| D0.2 | Demo **local** ou **online**? | ✅ | **Online** (é o ponto da demo e o maior risco) |

## Bloco 1 — Visual e câmera (RFC 0002)
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D1.1 | Elevação real vs fake height | ✅ | **Elevação real (geométrica)** — 4 alturas de canto por tile; altura afeta colisão e sorting |
| D1.2 | Quantas **camadas** de tilemap | 🔵 | _default provisório:_ 4 (terreno, objetos/elevação, decoração, efeitos) |
| D1.3 | Transparência/halos no depth buffer | 🔵 | _default provisório:_ adiar (sem sombra/água na demo) |

## Bloco 2 — Mundo e movimento (RFC 0003)
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D2.1 | **Tamanho da zona** à mão | 🔵 | _default provisório:_ pequena (~64×64 tiles), com uma rampa/platô para exercitar elevação |
| D2.2 | Resolução da **walkmap** sub-tile | 🔵 | _default provisório:_ 4×4 (elevação real pede granularidade) |
| D2.3 | Limiar de altura/diagonal **intransponível** | 🔵 | _default provisório:_ 1 degrau de altura sobe; acima disso bloqueia (ajustar no protótipo) |
| D2.4 | NPC com pathfinding ou só jogadores? | ✅ | **1 NPC replicado**: movimento calculado no servidor (A* hierárquico sobre a walkmap) e replicado a todos — prova pathfinding **e** autoridade do servidor sobre entidades |

## Bloco 3 — Loop de jogo (RFC 0004) — ⏭️ FORA DA DEMO (demo = andar + online, sem crafting)
_Reativar quando a demo evoluir para incluir coleta/crafting._
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D3.1 | Loop: só colher / colher→craftar / cadeia cruzada | ⏭️ | adiado |
| D3.2 | Nó compartilhado-contestado vs instanciado | ⏭️ | adiado |
| D3.3 | Sink de item (durabilidade) | ⏭️ | adiado |
| D3.4 | Mercado/troca vs só inventário | ⏭️ | adiado |

## Bloco 4 — Rede e persistência (RFC 0005) — só se a demo for online
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D4.1 | `lightyear` vs `replicon` puro | ✅ | **replicon puro** na demo (simples, põe a rede de pé). ⚠️ **Débito técnico [DT-01]**: netcode deve virar **estado da arte** (predição/rollback/lag-comp) antes de avaliar seriamente a sensação de movimento online |
| D4.2 | Transporte: renet/UDP vs QUIC/WebTransport | ✅ | **renet/UDP**. Exposição via **MetalLB `Service type=LoadBalancer protocol=UDP`** com IP de LAN próprio (Kong é L7/HTTP, não serve UDP). Cliente web = débito [DT-02] |
| D4.5 | **Deploy alvo** da demo | ✅ | **Home cluster k8s** (4 nós, v1.28.5, LAN 172.168.0.0/24, containerd). Servidor containerizado, exposto via MetalLB-UDP **só p/ teste entre devs**. Kong disponível p/ side-channels HTTP (health/métricas/BRP) |
| D4.3 | Persistência: SQLite vs PostgreSQL | ✅ | **Postgres do cluster** (`postgres` ns, `postgres-postgresql:5432`; criar db/schema do jogo), atrás de uma **camada repository/trait bem abstraída** (backend trocável). Já persistir **seed/parâmetros do gerador**. Escrita direta na demo; write-behind (RFC 0005) vira otimização posterior |
| D4.4 | Grau de **determinismo** exigido | ✅ | **Baixo na demo** (replicon puro, sem rollback — ver DT-01). Mas gameplay em **FixedUpdate + RNG semeado** por higiene (alimenta harness e o futuro rollback) |

## Bloco 5 — Harness de verificação (RFC 0006)
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D5.1 | Quais gates **já na demo** | ✅ | **Gate básico**: `cargo build && cargo test` headless bloqueia entrega que não compile/passe. Screenshot/BRP/seed ficam para fases seguintes (RFC 0006) |
| D5.2 | Visual-diff próprio agora ou só screenshot | 🔵 | _default provisório:_ só screenshot-como-artefato na demo; visual-diff depois |
| D5.3 | Versão de **Bevy** a fixar | 🔵 | _default provisório:_ fixar **Bevy 0.20.x**; spike de atualização orçado por release |

## Bloco 6 — Engine / infra (RFC 0001)
| # | Ponto | Status | Decisão |
|---|---|---|---|
| D6.1 | **Nº-alvo de jogadores/agentes por zona** | ✅ | **~10** (devs + 1 NPC) — alvo da demo; benchmark revisita p/ o MMO depois |
| D6.2 | Critério para **extrair o renderer próprio** | 🔵 | _default provisório:_ adiar (pós-MVP); só com evidência de gargalo mensurável |

---

### Como vamos tocar
Pergunto **uma de cada vez**, com opções e uma recomendação. Você escolhe (ou diz
"outro"), eu registro aqui a decisão e sigo para a próxima — começando pelo Bloco 0,
porque ele define quais blocos abaixo são relevantes para a **demo simples**.
