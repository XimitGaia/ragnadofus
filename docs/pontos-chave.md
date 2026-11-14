# Pontos-chave → backlog de RFCs

Este documento transforma a visão num conjunto de **pontos-chave de decisão**.
Cada ponto vira (ou alimenta) uma RFC. Para cada um registramos: a decisão a tomar,
por que importa para o MVP, e as **perguntas que a segunda pesquisa precisa responder**.

Prioridade: **P0** = caminho crítico do MVP · **P1** = necessário logo após o MVP ·
**P2** = frente futura, só esboço de RFC por ora.

Legenda de eixo: **A** Motor/Render · **B** Mundo/Dados · **C** Gameplay ·
**D** Online/MMO · **E** AI Studio/Harness.

---

## A1 — Base técnica do motor · P0 · eixo A
**Decisão:** sobre o que construímos — Bevy, Macroquad ou wgpu puro — e o que fica
como módulo próprio substituível.
**Por que importa:** define produtividade, portabilidade (desktop/web) e quanto
controle temos sobre a representação compacta.
**Pesquisar:**
- Estado de maturidade de Bevy / Macroquad / wgpu em 2026 (versões, breaking changes,
  ecossistema, suporte a WebGPU).
- Qual dá melhor acesso a *palette shaders*, texture arrays e pipelines customizados
  sem reescrever janela/input/áudio.
- Casos reais de pixel-art 2.5D em cada um; custos de abstrair "o que queremos
  substituir depois".

## A2 — Pipeline de render de mundo 2.5D · P0 · eixo A
**Decisão:** como desenhar o mundo contínuo com eficiência (tilemap-as-texture,
instancing, batching, depth sorting).
**Por que importa:** é o que permite mundo grande com poucas draw calls e pouca CPU.
**Pesquisar:**
- Técnica *tilemap como textura de índices* na GPU (ref.: `wgpu_tilemap`); limites e
  variações por camada (terreno, vegetação, água, objetos, sombras).
- Depth sorting de sprites sobre geometria com elevação (o "truque" do Ragnarok).
- Como fazer câmera isométrica oblíqua com pixel-art nítida (snapping, escala inteira).

## A3 — Representação compacta de assets · P0 · eixo A
**Decisão:** como representar muito mundo com pouco dado — o coração do *50 MB
Universe Challenge*.
**Por que importa:** é o diferencial técnico e artístico declarado do projeto.
**Pesquisar:**
- Paletas indexadas + palette swapping em shader (custo/benefício, variação de cor).
- Content-addressable storage / deduplicação de assets.
- Animação composível (paper-doll): corpo+cabeça+equipamento+arma como composição —
  como MMORPGs lidam com milhares de combinações.
- O que kkrieger e afins ensinam sobre reconstrução procedural; onde procedural
  *não* compensa.

## B1 — Formato declarativo de mundo/bioma · P0 · eixo B
**Decisão:** o schema que descreve um bioma/região por regras + seed, não por imagem.
**Por que importa:** é a fonte da geração; versioná-lo bem permite reconstruir o
mundo de forma consistente entre atualizações.
**Pesquisar:**
- Padrões de PCG determinística reprodutível (seeds, versionamento de regras).
- Como outros projetos declaram biomas/mundos (formatos, DSLs, data-driven design).
- Equilíbrio procedural × autoral para não cair em repetição sem intenção.

## B2 — Streaming de chunks + persistência diferencial · P0 · eixo B
**Decisão:** como dividir o mundo em chunks/regiões, carregar sob demanda e persistir
só as *diferenças* (árvore derrubada, construção do jogador) em vez de cópias.
**Por que importa:** sustenta "mundo maior que a memória" e "dados menores que o mundo".
**Pesquisar:**
- Arquiteturas de chunk streaming 2D/2.5D (tamanho de chunk, borda, LOD).
- Persistência diferencial sobre base procedural (gerar estático da seed, guardar só o delta).

## B3 — Movimentação livre contínua + navegação/colisão · P0 · eixo B
**Decisão:** movimento espacial contínuo (não grid de células) com colisão e
caminhabilidade — e como isso coexiste com futuro combate possivelmente tático.
**Por que importa:** é a crítica central a Dofus e a sensação-alvo do mundo.
**Pesquisar:**
- Representação de caminhabilidade/colisão em mundo por tiles com movimento contínuo
  (algo como o GAT do Ragnarok, repensado).
- Como separar "exploração contínua" de "combate tático por posições" sem conflito.

## C1 — Combate · P1 · eixo C
**Decisão:** tempo real, tático por turnos, ou híbrido configurável.
**Pesquisar (esboço):** padrões de arquitetura de combate; como alternar modo
exploração↔combate; o que preserva profundidade tática de Dofus com mundo livre.

## C2 — Economia, crafting e profissões · P1 · eixo C
**Decisão:** modelo de produção interdependente (matéria-prima → receita → item),
mercado e especialização entre jogadores.
**Pesquisar (esboço):** design sistêmico de economias de MMO; sinks/sources;
interdependência sem tédio.

## C3 — Progressão · P1 · eixo C
**Decisão:** classes/builds, progressão horizontal vs. vertical.
**Pesquisar (esboço):** modelos de progressão modular que não dependam só de "números maiores".

## D1 — Arquitetura MMO (servidor autoritativo) · P2 · eixo D
**Decisão:** servidores, autoridade do mundo, sincronização, áreas/instâncias, escala.
**Pesquisar (esboço):** arquiteturas de MMORPG; onde fica a autoridade; como regiões/
chunks mapeiam para processos/servidores. *Fora do MVP, mas decisões de dados (B1–B3)
não podem inviabilizá-la.*

## D2 — Mundo vivo / simulação · P2 · eixo D
**Decisão:** ecossistema, NPCs, recursos e eventos que evoluem mesmo sem jogadores.
**Pesquisar (esboço):** simulação de mundo persistente de baixo custo.

## E1 — Harness de desenvolvimento com IA · P0 (mínimo) / P1 (completo) · eixo E
**Decisão:** o loop em que um agente propõe mudança → gera artefatos → build/testes →
screenshot/logs → evidência. Um **mínimo** entra no MVP (como provamos que algo funciona).
**Pesquisar:**
- Padrões de harness para dev de jogo com IA: inspeção de runtime, comandos
  controláveis, sessões reproduzíveis, verificação visual automatizada.

## E2 — AI Game Studio (produto) · P2 · eixo E
**Decisão:** plataforma onde "descreva uma floresta" vira bioma + assets + testes +
versão jogável.
**Pesquisar (esboço):** geração de conteúdo por IA com artefatos verificáveis;
pipeline designer→agente→jogo.

---

## Resumo de prioridades para a segunda pesquisa

> **Reprioritizado em 2026-11-14** após decidir o MVP como *fatia jogável + online*
> (ver `sintese.md §4`). O caminho crítico deslocou-se do "motor+dados compacto" para
> "loop jogável online sobre Bevy". A representação compacta (A3), o bioma procedural
> (B1) e o streaming (B2) continuam sendo a **alma técnica do projeto**, mas passam a
> **P1**: não bloqueiam a primeira fatia jogável. As decisões de dados do MVP devem
> ser tomadas para **não inviabilizá-las**.

| RFC | Título | Prio | Eixo | No MVP? |
|---|---|---|---|---|
| A1 | Base técnica do motor (→ Bevy, decidido) | P0 | A | sim — confirmar via RFC |
| A2 | Pipeline de render 2.5D | P0 | A | sim |
| B3 | Movimentação contínua + navegação/colisão | P0 | B | sim |
| C2 | Economia/crafting (loop de coleta→craft) | P0 | C | sim — o loop jogável |
| D1 | Arquitetura cliente-servidor autoritativa | P0 | D | sim — online desde já |
| E1 | Harness de dev com IA (mínimo) | P0 | E | sim |
| B2 | Persistência de estado (fatia de D1/B2) | P0 | B/D | sim — só persistência; streaming fica P1 |
| A3 | Representação compacta de assets | P1 | A | não — alma técnica, pós-MVP |
| B1 | Bioma declarativo + procedural | P1 | B | não — zona à mão primeiro |
| B2 | Streaming de chunks | P1 | B | não — zona pequena cabe em memória |
| C1 | Combate | P1 | C | não — alternativa ao loop de crafting |
| C3 | Progressão | P1 | C | não |
| D2 | Mundo vivo/simulação | P2 | D | não |
| E2 | AI Game Studio (produto) | P2 | E | não |

**Caminho crítico do MVP = A1, A2, B3, C2, D1, E1 + a fatia de persistência.**
A segunda pesquisa (em andamento) ataca exatamente esse conjunto.
