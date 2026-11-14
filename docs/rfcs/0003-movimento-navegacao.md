# RFC 0003 — Movimento contínuo, colisão e navegação

- **Estado:** Draft
- **Eixo / código do backlog:** B3
- **Prioridade:** P0
- **Autor(es):** equipe + pesquisa (agente)
- **Data:** 2026-11-14
- **Pesquisa-fonte:** `../../pesquisa/B3-movimento-navegacao.md`
- **Padrões aplicáveis:** `../padroes-engenharia-e-design.md` §2
- **Depende de:** RFC 0001 · **Condiciona:** RFC 0002 (elevação), RFC 0005 (física no servidor)

## 1. Problema e contexto
A tese de design é **movimento espacial contínuo e livre** (crítica ao grid de células
do Dofus), com colisão e caminhabilidade num mundo de tiles — deixando a porta aberta
para combate tático por posições no futuro, sem conflito de sistemas.

## 2. Objetivos e não-objetivos
**Objetivos:** mover o personagem sub-tile com colisão/sliding; caminhabilidade por
terreno/altura; uma **única fonte de verdade** servível a exploração e combate.
**Não-objetivos:** definir o sistema de combate (C1, P1); pathfinding de hordas no MVP.

## 3. Pesquisa / evidências
Resumo (detalhe e URLs em `B3-movimento-navegacao.md`):
- **Camada de dados estilo GAT repensado:** por tile, as **4 alturas de canto**
  (heightfield p/ o 2.5D) + flags de caminhabilidade/terreno. Mesma verdade p/
  exploração e combate.
- **Movimento contínuo** (posição em `Vec2/Vec3`) com colisão por **swept-AABB +
  sliding** contra uma **walkmap sub-tile** de alta resolução (2×2/4×4, triângulos nas
  diagonais) — não travado em células.
- **Navegação híbrida:** A* hierárquico (`hierarchical_pathfinding`) p/ NPC com destino
  único; flow fields + evasão local p/ hordas; navmesh any-angle
  (`vleue_navigator`/Polyanya) em reserva p/ geometria orgânica.
- **Combate tático** = grid lógico **instanciado sobre** o espaço contínuo (padrão
  BG3/DOS2), não arena separada.
- **Física/colisão:** começar com **`avian2d`** (ECS nativo, API próxima a
  `bevy_rapier2d` p/ migração fácil).

## 4. Alternativas consideradas
| Tema | Escolha MVP | Alternativa |
|---|---|---|
| Caminhabilidade | Walkmap sub-tile derivada do GAT-repensado | Grid de células puro (rejeitado: trava o mundo) |
| Colisão | avian2d (swept-AABB/sliding) | bevy_rapier2d |
| Pathfinding NPC | A* hierárquico | navmesh any-angle (reserva) |
| Combate | Grid lógico sobre contínuo | Arena/instância separada |

## 5. Decisão proposta
MVP com **walkmap sub-tile derivada de dados por-tile (4 alturas de canto + flags)**,
**movimento contínuo com `avian2d`**, e **A\* hierárquico** para NPCs. Combate futuro
instancia um grid lógico sobre o mesmo espaço.

## 6. Impacto e riscos
A escolha de "alturas de canto" alimenta diretamente o y-sort da RFC 0002 e a física
autoritativa no servidor (RFC 0005). **Padrão (P2):** o **broadphase/spatial-hash do
`avian2d`** será **reusado pelo interest management** do servidor (AOI = célula + 8
vizinhas, RFC 0005) — manter a física acessível no servidor e indexada espacialmente,
não só para colisão mas como base da área de interesse. **Incertezas:** (1) física geométrica por agente
pode **não escalar** no servidor — walkmap + A* talvez escale melhor (falta benchmark
do nº-alvo de agentes/mapa); (2) maturidade de navmesh dinâmico 2D em Rust/Bevy;
(3) parâmetros abertos: resolução sub-tile, limiar de altura/diagonal intransponível,
cell size tático, snap de agentes "entre células" ao iniciar combate, multi-tamanho
de agentes (clearance maps).

## 7. Plano de validação
- Protótipo: personagem andando contínuo com colisão/sliding contra obstáculos e
  rampas; NPC indo de A a B com A* hierárquico.
- Benchmark de física no servidor: nº de agentes antes de degradar o tick (RFC 0005).
- Teste de transição exploração→combate: instanciar grid lógico e snap coerente.

## 8. Questões em aberto
Resolução da walkmap; limiares de intransponibilidade; cell size tático; escala de
física server-authoritative; multi-tamanho de agentes.
