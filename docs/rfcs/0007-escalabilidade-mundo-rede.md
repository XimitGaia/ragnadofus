# RFC 0007 — Escalabilidade do mundo e da rede (chunking, interesse, culling)

- **Estado:** Draft
- **Eixo / código do backlog:** Escala (render + rede + simulação)
- **Prioridade:** P1 (fundação; pagar por gatilho, não tudo de uma vez)
- **Autor(es):** Claude (par do usuário)
- **Data:** 2026-11-15
- **Supersede / Superseded by:** —

## 1. Problema e contexto

A Demo 01 evoluiu para render 3D (câmera estilo RO), FOV e multi-nível. O código
**funciona e é conceitualmente correto**, mas foi escrito para **um mapa pequeno (64×64)
carregado inteiro** e **poucos players**. Antes de empilhar features, queremos garantir que
a base **aguenta escala** (mapas grandes com partes pesadas e outras não; muitos players;
renderizar só o que o player espera ver) com **conceitos robustos**, sem reescrever.

Gatilho: o usuário sentiu travamento com 2 clientes em debug. A causa imediata (deps não
otimizadas + spam de clique) foi resolvida, mas a investigação expôs limites estruturais
que esta RFC registra e prioriza.

## 2. Objetivos e não-objetivos

**Objetivos:**
- Render e simulação com custo proporcional ao que está **perto/visível** (não ao mundo
  inteiro).
- Rede com banda proporcional ao que cada cliente **precisa ver** (não O(players²)).
- Caminho incremental sobre a arquitetura atual (ECS data-oriented, shared sem render,
  servidor autoritativo) — **sem reescrever**.

**Não-objetivos:**
- Netcode estado-da-arte (predição/rollback) — segue em [DT-01] / RFC 0005.
- Cliente web — RFC 0005 / DT-02.
- Definir tamanho final de mundo ou nº de players alvo (parametrizável).

## 3. Pesquisa / evidências

Padrões consolidados da indústria (MMO/open-world 2.5D-3D):
- **Chunking + streaming**: mundo em blocos N×N; carrega/descarrega por proximidade da
  câmera; terreno = 1 malha por chunk (não 1 entidade por tile). Base de culling eficiente.
- **Frustum/occlusion culling**: o Bevy já faz frustum culling por entidade; chunks reduzem
  o nº de entidades testadas.
- **Interest management / Area of Interest (AoI)**: servidor replica a cada cliente só as
  entidades na sua região (grid de interesse / rooms). `bevy_replicon` tem *visibility*.
- **Particionamento espacial** (grid/quadtree) para consultas "quem está perto" / FOV /
  colisão em O(vizinhança), não O(N).
- **Fog-of-war via shader** (máscara amostrada no material) em vez de trocar material por
  célula — custo O(1) no fragment, independe do tamanho do mapa.

Estado atual (fatos): mundo inteiro spawnado como entidades por célula; `replicate` manda
tudo a todos; FOV troca material célula a célula na mudança de tile; consultas ECS globais.

## 4. Alternativas consideradas

| Alternativa | Prós | Contras | Custo |
|---|---|---|---|
| **Manter mundo inteiro em entidades** | simples; funciona na demo | memória/spawn O(mundo); FOV O(tiles); não escala | 0 agora, alto depois |
| **Chunking + streaming (proposto)** | custo ~visível; base de culling/stream | precisa gerência de carga/descarga + malha por chunk | médio, incremental |
| **Um único mega-mesh do mapa** | poucos draw calls | sem streaming; recarrega tudo ao mudar | médio |
| **Rede: replicar tudo** | trivial | banda O(players²) | 0 agora, inviável com muitos |
| **Rede: AoI/visibility (proposto)** | banda ~o que se vê | lógica de interesse + troca de estado ao entrar/sair | médio |
| **Fog via troca de material/tile** (atual) | fácil | O(tiles) por mudança | baixo p/ mapa pequeno |
| **Fog via shader (proposto)** | O(1) no shader; escala | shader custom + textura de máscara | médio |

## 5. Decisão proposta

Adotar, **incrementalmente e por gatilho**, quatro evoluções sobre a base atual:

1. **Mundo em chunks** (render + walkmap) com streaming por proximidade e culling. Terreno
   vira malha por chunk.
2. **Interest management** no `replicon` (grid de AoI): cada cliente recebe só as entidades
   da sua vizinhança. Casa com o FOV ("render o que o player espera").
3. **Índice espacial de entidades** (grid) para FOV/colisão/consultas de vizinhança.
4. **Fog-of-war via shader** (substitui a troca de material por célula do FOV atual).

Validação por **protótipo + benchmark** (ver §7): medir frame time e banda antes/depois.

Já aplicado como fundação imediata (não é esta RFC, mas habilita medir): `opt-level=3` nas
deps em dev, throttle do clique, e centralização das escadas ([DT-04]).

## 6. Impacto e riscos

- **Mexe em:** RFC 0002 (render 2.5D — vira chunked), RFC 0005 (cliente-servidor — AoI),
  `shared/walkmap` (chunking), cliente (streaming/culling/fog shader).
- **Riscos:** complexidade de borda entre chunks (pathfinding/visão atravessando chunks);
  "pop-in" ao streamar; estado ao entrar/sair da AoI (replicon). **Mitigação:** começar com
  chunk grande + margem de pré-carga; medir antes de otimizar.
- **Débito atacado:** [DT-08] (FOV 2D → shader/3D), e reduz [DT-09].

## 7. Plano de validação

- **Protótipo 1 (chunk):** mapa 256×256 em chunks 32×32, só vizinhos da câmera ativos.
  Métrica: frame time estável e memória ~constante ao andar (vs. crescer com o mundo).
- **Protótipo 2 (AoI):** N bots conectados; medir banda/cliente com e sem interest
  management. Critério: banda por cliente ~constante com o nº de players distantes.
- **Aceitação:** 60 fps com mundo >> tela e dezenas de players simulados, em release.

## 8. Questões em aberto

- Tamanho de chunk e de célula de AoI (parametrizar e medir).
- Pathfinding/visão **cross-chunk** (A* precisa dos chunks vizinhos carregados).
- Fonte do mundo grande (procedural por seed, como hoje, vs. autorado) e como isso casa com
  o `WorldRepository` (RFC 0002/0005).
- Ordem exata de pagamento (provável: chunk → AoI → índice/fog), confirmada pelos gatilhos
  reais (mapa grande primeiro, ou muitos players primeiro).
