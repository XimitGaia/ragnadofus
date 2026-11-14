# Arranque da Demo 01 — prompt para nova sessão

> **Como usar:** abra uma nova sessão do Claude Code na raiz do repositório
> `ragnadofus` e **cole o bloco abaixo** como primeira mensagem. Ele carrega as regras,
> aponta a documentação e define o primeiro passo de forma controlada (um passo por vez,
> com gate de verificação).

---

```
Você vai iniciar a implementação da DEMO 01 do projeto ragnadofus, de forma CONTROLADA
e seguindo as regras do repositório. Antes de escrever qualquer código, leia nesta
ordem e confirme que entendeu:

1. AGENTS.md  (regras inegociáveis de trabalho IA-first)
2. docs/demo-01-spec.md  (o que a Demo 01 é — escopo, decisões, Definition of Done)
3. docs/padroes-engenharia-e-design.md  (padrões de código/arquitetura a seguir)
4. docs/plano-decisoes.md  (todas as decisões já tomadas — não reabrir sem pedir)
5. docs/debito-tecnico.md  (DT-01 netcode, DT-02 web — atalhos conscientes)
6. docs/rfcs/0001, 0005, 0006  (engine, cliente-servidor, harness)

REGRAS INEGOCIÁVEIS (detalhe em AGENTS.md):
- code-intel (MCP) é o CARRO-CHEFE para TODA operação de código. grep/sed só para texto
  literal. Em Rust, edições com verify_build. blast_radius antes de edição ampla.
- Verificação é um GATE, não uma afirmação: nada é "pronto" sem `cargo build && cargo
  test` verdes, rodados pelo xtask (local == CI). O agente não edita o que o avalia
  (forbidden-path).
- Trabalhe UM PASSO POR VEZ. Ao fim de cada passo, PARE e apresente evidência (saída do
  gate) e espere minha aprovação antes do próximo. Não feche hipóteses; não dê push.

AMBIENTE (já verificado): rustc/cargo/rust-analyzer 1.96.1; Docker 26; cluster k8s
(4 nós, MetalLB para LoadBalancer UDP; Postgres em postgres-postgresql.postgres:5432;
Kong = ingress HTTP). Deploy só para teste entre devs.

PASSO 1 (e SOMENTE o passo 1 por enquanto):
Criar o workspace Cargo com o GRAFO DE CRATES UNIDIRECIONAL (padrão P1):
  - crates/shared  (ECS do jogo em Bevy puro, SEM bevy_render; tipos compartilhados:
    PlayerId, Tick, componentes, mensagens de rede)
  - crates/server  (Bevy headless; depende de shared)
  - crates/client  (Bevy + render; depende de shared)
  - crates/xtask   (runner do gate: build + test; `cargo run -p xtask ci`)
Regras do passo: fixar Bevy 0.20.x; client→shared e server→shared, NUNCA shared→* nem
client↔server; adicionar cargo-deny/cargo-tree checando o grafo; Clippy negando
unwrap/expect; um teste trivial em shared para o gate ter o que rodar; o server deve
COMPILAR SEM renderer (prova do desacoplamento).
Antes de codar: rode `code-intel doctor` com smoke=true no projeto para validar que o
rust-analyzer indexa o workspace (o code-intel é nosso carro-chefe — garanta que está
funcionando ANTES de prosseguir). Se não indexar, pare e me avise.
Entregue como evidência: a árvore de crates, a saída de `cargo run -p xtask ci` verde, e
a confirmação do code-intel funcionando. Depois PARE e aguarde minha aprovação para o
Passo 2 (zona + render + personagem andando local).
```

---

## Sequência completa (referência — só o Passo 1 vai no prompt acima)
1. Workspace + grafo de crates + xtask gate + validar code-intel.  ← **arranque**
2. Zona à mão + render 2.5D + câmera + personagem andando **local**.
3. Elevação real: walkmap 4×4, colisão `avian2d`, y-sort.
4. Servidor autoritativo + replicon: mover por intenção, replicar a 2 clientes.
5. NPC com A* no servidor, replicado.
6. `WorldRepository` (trait) → Postgres; persistir seed (delta-sobre-baseline).
7. Containerizar + manifests k8s + MetalLB-UDP; teste entre devs.

## Estado do code-intel (validado nesta sessão)
- Servidor **de pé** (`doctor`: 0 problemas na checagem estática).
- `rust-analyzer` 1.96.1 presente → suporte Rust funcionará assim que houver código.
- Validação **end-to-end** (`find_references` real) fica para o Passo 1, após `cargo
  init` — por isso o prompt pede `doctor smoke=true` antes de prosseguir.
