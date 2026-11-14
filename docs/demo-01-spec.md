# Demo 01 — "Andar + Online" (spec consolidada)

Consolida todas as decisões de `plano-decisoes.md` numa descrição única do que vamos
construir. É o alvo concreto da primeira demo. Escopo deliberadamente pequeno; a
qualidade de engenharia (sobretudo a rede) tem metas de longo prazo registradas em
`debito-tecnico.md`.

## 1. Objetivo
Provar o **loop cliente-servidor autoritativo** cedo (o maior risco do projeto): dois
ou mais devs, em clientes desktop, **andam e se veem** numa zona 2.5D com **elevação
real**, e veem **um NPC** cujo movimento é calculado e replicado **pelo servidor**.

## 2. Escopo
**Dentro:** personagem com sprite animado; câmera isométrica; zona à mão com rampa/
platô (elevação real); movimento contínuo + colisão; multiplayer autoritativo (servidor
dono do estado); 1 NPC com A* replicado; deploy no home cluster; gate de build+teste.
**Fora (adiado):** coleta/crafting (Bloco 3); predição/rollback (DT-01); cliente web
(DT-02); procedural/streaming; combate; progressão.

## 3. Decisões consolidadas
| Área | Decisão | Ref |
|---|---|---|
| Escopo | Andar + online (sem crafting), online desde já | D0.1/D0.2 |
| Elevação | **Real (geométrica)** — 4 alturas de canto por tile | D1.1 |
| Render | `bevy_ecs_tilemap` + y-sort + câmera upscale inteiro; 4 camadas; sem sombra/água | D1.2/D1.3 · RFC 0002 |
| Zona | ~64×64 tiles à mão, com rampa/platô | D2.1 |
| Movimento | Contínuo, `avian2d`, walkmap sub-tile 4×4; sobe 1 degrau, acima bloqueia | D2.2/D2.3 · RFC 0003 |
| NPC | 1 NPC, A* hierárquico no servidor, **replicado** | D2.4 |
| Rede | **replicon puro** (sem predição) — ⚠️ DT-01 | D4.1 |
| Transporte | **renet/UDP**, exposto por **MetalLB LoadBalancer UDP** (IP de LAN) | D4.2 |
| Deploy | **Home cluster k8s** (4 nós, v1.28.5), só p/ teste entre devs | D4.5 |
| Persistência | **Postgres do cluster** atrás de repository/trait trocável; persistir seed | D4.3 |
| Determinismo | Baixo; gameplay em `FixedUpdate` + RNG semeado (higiene) | D4.4 |
| Harness | **Gate básico**: `cargo build && cargo test` headless bloqueia entrega | D5.1 |
| Bevy | Fixar **0.20.x** | D5.3 |
| Escala-alvo | **~10** (devs + 1 NPC) | D6.1 |

## 4. Arquitetura da demo
```
┌─ Cliente (Bevy 0.20, desktop) ─┐        ┌─ Servidor (Bevy headless, k8s pod) ─┐
│ render 2.5D (ecs_tilemap+y-sort)│        │ MinimalPlugins + FixedUpdate ~64Hz  │
│ input → envia INTENÇÃO de mover │──UDP──►│ autoridade: move, colide (avian2d)  │
│ mostra players + NPC (replicados)│◄─delta─│ NPC: A* na walkmap                  │
└─────────────────────────────────┘        │ replicon: replica estado            │
         ▲ MetalLB LB (UDP, IP LAN)         │ repository trait → Postgres (seed)  │
         └──────────────────────────────────┴──── Kong (HTTP) p/ health/métricas ─┘
```
Mesmo crate de ECS/componentes compartilhado cliente↔servidor (RFC 0001).

## 5. Stack concreto (a fixar na RFC 0005/0001)
- **Engine:** Bevy 0.20.x · **Tilemap:** bevy_ecs_tilemap 0.20 · **Física:** avian2d
- **Rede:** bevy_replicon 0.44.x + renet2 2.x (UDP) · **Serialização:** postcard
- **Pathfinding:** hierarchical_pathfinding · **Câmera:** bevy_pixel_camera/bevy_pixcam
- **Persistência:** Postgres (cluster) via sqlx/diesel atrás de um trait `WorldRepository`
> Spike de compatibilidade de versões Bevy↔replicon↔avian antes de cravar (RFC 0005 §7).

## 6. Deploy no cluster
- Servidor containerizado (imagem Linux); `Deployment` 1 réplica (zona única).
- `Service type=LoadBalancer` com `protocol: UDP` (MetalLB atribui IP de LAN, ex.
  `172.168.0.105`). Clientes desktop apontam para esse IP:porta.
- (Opcional) `Ingress` Kong p/ um endpoint HTTP de health/métricas.
- Conexão ao `postgres-postgresql.postgres.svc:5432` (criar db/role do jogo).

## 7. Definition of Done (critério objetivo)
1. `cargo build && cargo test` headless verdes (gate).
2. Servidor roda como pod no cluster, exposto via MetalLB-UDP.
3. **Dois clientes** conectam e **veem o personagem um do outro andar** em tempo quase-real.
4. A zona tem **rampa/platô**: colisão e ordenação visual corretas na elevação.
5. **1 NPC** anda sozinho (A* no servidor) e é visto igual por todos os clientes.
6. Seed/parâmetros da zona são lidos do Postgres (repository), reprodutíveis.

## 8. Sequência de implementação sugerida
1. **Spike de versões** + esqueleto do projeto (workspace cliente/servidor/compartilhado) + gate CI.
2. Zona à mão + render 2.5D + câmera + personagem andando **local** (sem rede).
3. Elevação real: walkmap 4×4, colisão `avian2d`, y-sort correto.
4. Servidor autoritativo + replicon: mover via intenção, replicar a 2 clientes.
5. NPC com A* no servidor, replicado.
6. Repository trait → Postgres (persistir/ler seed).
7. Containerizar + manifests k8s + MetalLB-UDP; teste entre devs.

## 9. Débitos assumidos
- **DT-01 🔴** netcode replicon-puro → elevar a estado da arte (predição/rollback).
- **DT-02 🟡** sem cliente web → QUIC/WebTransport depois.
