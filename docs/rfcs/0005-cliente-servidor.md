# RFC 0005 — Arquitetura cliente-servidor autoritativa + persistência

- **Estado:** In Review
- **Eixo / código do backlog:** D1 (+ fatia de persistência B2)
- **Prioridade:** P0
- **Autor(es):** equipe + pesquisa (agente)
- **Data:** 2026-11-14
- **Pesquisa-fonte:** `../../pesquisa/D1-cliente-servidor.md`
- **Padrões aplicáveis:** `../padroes-engenharia-e-design.md` §2
- **Depende de:** RFC 0001 · **Condiciona:** RFC 0003 (física no servidor), RFC 0004 (autoridade)

## 1. Problema e contexto
O MVP é **online desde o início**: servidor autoritativo dono do estado do mundo,
clientes (Bevy) sincronizam. É a decisão que mais aumenta a complexidade do MVP —
precisamos do caminho **mínimo viável** (poucos jogadores, uma zona).

## 2. Objetivos e não-objetivos
**Objetivos:** autoridade do servidor; sincronização de estado; persistência durável
do inventário/recursos; reuso do mesmo ECS cliente/servidor (RFC 0001).
**Não-objetivos:** sharding multi-zona; milhares de jogadores; cliente web no MVP
(caminho mantido aberto, não exigido).

## 3. Pesquisa / evidências
Resumo (detalhe e URLs em `D1-cliente-servidor.md`):
- **Servidor = Bevy headless** (`MinimalPlugins` + `FixedUpdate`, ~64 Hz), **1 processo =
  a única zona do MVP**.
- **Replicação server-autoritativa** com **`bevy_replicon` 0.44.x** (mesmo ECS
  cliente/servidor; visibilidade/rooms embutida).
- **`lightyear` 0.31** por cima → client-side prediction+rollback, interpolation, lag
  compensation de fábrica.
- **Transporte:** `renet`/`renet2` 2.x (UDP/netcode) p/ desktop; `aeronet` 0.22 +
  `quinn` 0.11 (WebTransport/QUIC) como caminho p/ cliente web.
- **Serialização:** `postcard`. Cliente envia só **intenção**; servidor valida e
  replica delta por componente alterado.
- **Persistência:** **PostgreSQL** como source of truth com **write-behind** fora do
  tick (ou SQLite enquanto single-process).

## 4. Alternativas de stack
| Camada | Escolha MVP | Alternativa / nota |
|---|---|---|
| Replicação | bevy_replicon | replicon puro + predição à mão (menos dep.) |
| Predição/rollback | lightyear | escrever à mão sobre replicon |
| Transporte | renet (UDP) | aeronet+QUIC se WASM entrar cedo |
| Persistência | PostgreSQL write-behind | SQLite enquanto single-process |

## 5. Arquitetura mínima para o MVP
```
Cliente (Bevy)  ──intenções──►  Servidor (Bevy headless, FixedUpdate ~64Hz)
     ▲                                  │ valida (RFC 0004) + simula (RFC 0003)
     └──────delta de estado────────────┤
                                        └── write-behind ──► PostgreSQL (durável)
```

### Padrões de netcode a mirar (P2 — alvo do DT-01, além da demo)
- **Netcode por classe de entidade:** avatar próprio = prediction + reconciliation;
  outros jogadores/mobs = snapshot interpolation (~100 ms); mundo em massa = state sync
  com priority accumulator + delta compression.
- **Dois ticks distintos:** simulação em `FixedUpdate` (~64 Hz) ≠ **tick de rede**
  (~20–30 Hz). Não confundir.
- **Interest management** em grid/spatial-hash (AOI = célula + 8 vizinhas), **reusando o
  broadphase do `avian2d`** (RFC 0003); vira visibility filter e também é anti-wallhack.
- **Migração DT-01** replicon → `lightyear` 0.31 (compatível com Bevy 0.20, construído
  sobre replicon) em **6 fases incrementais**, cada uma com gate de verificação.
- **Contrato de determinismo** `(seed, rules_version, engine_version)` persistido junto
  ao estado — necessário para rollback e reprodutibilidade (cruza com RFC 0006 e P6).

## 6. Impacto e riscos
É a espinha dorsal: RFC 0003 (física autoritativa) e 0004 (transações) rodam aqui.
**3 incertezas:** (1) adotar `lightyear` (churn de API 0.x) **vs** replicon puro +
predição à mão; (2) UDP/renet **vs** QUIC/WebTransport (define se WASM entra cedo);
(3) **determinismo** da simulação Bevy para rollback, e compatibilidade de versões
Bevy ↔ lightyear ↔ aeronet ↔ replicon.

## 7. Plano de validação
- **Benchmark de escala cedo** (também pedido pela RFC 0001/0003): nº de agentes/
  jogadores por zona antes de o tick degradar.
- Spike de compatibilidade de versões antes de fixar o stack.
- Teste de durabilidade: matar o servidor e recuperar o estado do Postgres.

## 8. Questões em aberto
Parâmetros de interest management (tamanho de célula, histerese na transição); tick de
rede exato (20–30 Hz); UDP vs QUIC (WASM — DT-02); matriz de versões compatíveis
Bevy ↔ replicon ↔ lightyear ↔ avian. _`lightyear` vs predição própria_ foi resolvido
por P2: **lightyear** é o caminho (construído sobre replicon, 6 fases).
