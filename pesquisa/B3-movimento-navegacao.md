# B3 — Movimentação e Navegação (MMORPG 2.5D em Rust/Bevy)

> Brief de pesquisa para fundamentar a RFC de movimento espacial contínuo + colisão/caminhabilidade + navegação, com porta aberta para combate tático por posições.
> Data: 2026-11-14. Convenção: **[FATO]** = afirmação com fonte/URL verificável; **[HIPÓTESE]** = inferência/recomendação de design não diretamente atestada.

---

## Problema

A decisão central de design é ter **movimento espacial contínuo e livre** (crítica explícita ao grid de células do Dofus/Wakfu), com **colisão e caminhabilidade**, num mundo feito de **tiles**. Simultaneamente, queremos manter a opção futura de **combate tático por posições/células** sem reescrever os sistemas de mundo.

Isso gera quatro perguntas de engenharia:

1. Como representar caminhabilidade/colisão num mundo de tiles quando o movimento é **sub-tile / contínuo** (sem travar o agente no centro da célula)?
2. Que padrão de navegação/pathfinding usar num mundo contínuo 2.5D com **muitos agentes/NPCs** (navmesh vs grid A* vs flow fields)?
3. Como **separar** "exploração contínua" de "combate tático por células" sem conflito de sistemas?
4. Quais **crates Rust/Bevy** maduros existem para colisão 2D, física e pathfinding?

A tensão de fundo: o modelo de **célula** torna caminhabilidade/colisão/pathfinding simples e barato (e é natural para combate tático), mas restringe o movimento; o modelo **contínuo** dá a sensação desejada, porém exige colisão geométrica e navegação por malha. A RFC precisa escolher a camada de dados que serve aos dois mundos.

---

## Achados (com URLs)

### 1. GAT (Ragnarok Online): caminhabilidade por célula com altura — o precedente histórico

**[FATO]** O formato **GAT** ("GroundAltitude"/GAT) descreve o mundo como uma **grade 2D de tiles**; após um header de 6 bytes vêm Width/Height e então blocos de ~20 bytes por célula. Cada célula armazena **quatro alturas de canto** (SW, SE, NW, NE) como floats e um campo de **tipo de terreno**. É o GAT (não o GND) que o **servidor** usa para caminhabilidade; o cliente usa GAT para posicionar unidades na altura correta.
Fontes: <https://ragnarokresearchlab.github.io/file-formats/gat/>, <https://openkore.com/wiki/Field_file_format>, <https://github.com/open-ragnarok/roint/blob/master/gat.c>

**[FATO]** Códigos de tipo de terreno documentados (GAT 1.2): `0` = terreno normal **caminhável**; `1` = **obstruído/impassável**; `5` = "cliff" impassável mas atravessável por projétil ("snipeable"). Valores adicionais aparecem para água (ex.: `3` = água caminhável; `1/2/4/6` = variações não-caminháveis). GAT 1.3 adiciona detecção de água por bitmask `0x08` no campo de tipo.
Fontes: <https://ragnarokresearchlab.github.io/file-formats/gat/>, <https://github.com/rdw-archive/RagnarokFileFormats>

**[FATO]** Mapas de RO chegam tipicamente a ~400×400 células; a dimensão está no próprio GAT.
Fonte: (busca agregada) <https://github.com/Duckwhale/RagnarokFileFormats/blob/master/GND.MD>

**Como repensar o GAT para movimento contínuo:**
- **[HIPÓTESE]** Mantenha o GAT como *fonte de dados de autoria* (por célula: altura dos 4 cantos + flags de terreno), mas **não** use a célula como unidade de movimento. A posição do agente é um `Vec2`/`Vec3` contínuo; a célula vira apenas um **lookup** (qual bioma/altura/custo estou pisando).
- **[HIPÓTESE]** Para colisão sub-tile, suba a resolução da caminhabilidade: cada tile pode ser subdividido (ex.: 2×2 ou 4×4 sub-células), e paredes diagonais podem usar **sub-tiles triangulares** (tile bisseccionado nas diagonais → 4 zonas sólidas/não-sólidas) para apresentar uma superfície diagonal lisa ao detector de colisão. Isto é um padrão descrito pela comunidade para movimento contínuo sobre tiles.
Fonte do padrão sub-tile/triangular e "ausência de collider = caminhável": <https://github.com/Abaudat/browser-city/issues/56>, <https://gamedev.net/forums/topic/178857-smooth-tile-based-movement/>
- **[HIPÓTESE]** As 4 alturas de canto do GAT se traduzem bem para um **heightfield** por tile; a altura contínua do agente vem de interpolação bilinear dentro do tile (que é exatamente como o cliente de RO posiciona unidades) — útil para o "2.5D".

### 2. Caminhabilidade/colisão contínua sobre tiles

**[FATO]** Padrões consolidados para movimento contínuo sobre mundo de tiles:
- **Colisão por swept AABB** contra retângulos ancorados na base, com **sliding** ao bater em quina em ângulo (o agente desliza ao longo da superfície em vez de "agarrar").
- **Layer de colisão** separado (0 = caminhável, 1 = bloqueado); **ausência de collider = caminhável**.
- **Subdivisão do tile** (4 triângulos por diagonal) para superfícies diagonais lisas.
Fontes: <https://github.com/Abaudat/browser-city/issues/56>, <https://github.com/APKiwiOrg/KhaozEngine/issues/1219>, <https://gamedev.net/forums/topic/178857-smooth-tile-based-movement/>, <https://qbmikehawk.neocities.org/articles/tileworld/>

### 3. Pathfinding em mundo contínuo 2.5D: navmesh vs grid A* vs flow fields

**[FATO]** **Navmesh** gera um grafo **esparso** (poucos polígonos cobrem áreas grandes) → A* roda mais rápido e produz caminhos **mais lisos/naturais**; adequado para geometria orgânica/irregular e ambientes majoritariamente estáticos. Suporta centenas de agentes.
Fontes: <https://vav-labs.com/blog/godot-pathfinding-grid-vs-navmesh/>, <https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter27_Tactical_Pathfinding_on_a_NavMesh.pdf>, <https://recited.io/kb/ai-in-game-development/pathfinding-and-navigation/navigation-meshes/>

**[FATO]** **Grid A*** vence em **obstáculos dinâmicos** (marcar célula como sólida é O(1), sem rebake), **agentes multi-tamanho** via clearance maps, falhas de path explicáveis, e movimento travado em tile. Rebake de navmesh escala com a região afetada.
Fonte: <https://vav-labs.com/blog/godot-pathfinding-grid-vs-navmesh/>

**[FATO]** **Flow fields** (3 camadas: cost field → integration field via Dijkstra → flow field de vetores) custam `O(N log N)` **uma vez** e **O(1) por agente** depois. Ideais quando **muitos agentes compartilham o mesmo destino** e o mapa é grid-based. Exigem **evasão local** suplementar (RVO/ORCA, forças de separação), pois só lidam com obstáculos estáticos. A* custa `O(N log N)` **por agente** → multiplica rápido com centenas de unidades.
Fontes: <https://moonjump.com/game-dev-mechanics-flow-fields-how-it-works/>, <https://github.com/Vincent-VD/Flow-Fields-Nav>

**[FATO]** Padrão de produção comum: **híbrido** — A* para destinos únicos + flow fields para movimento em grupo rumo a objetivo compartilhado.
Fontes: <https://moonjump.com/game-dev-mechanics-flow-fields-how-it-works/>, <https://vav-labs.com/blog/godot-pathfinding-grid-vs-navmesh/>

### 4. Separar exploração contínua de combate tático por células

**[FATO]** **Baldur's Gate 3** faz exploração em **tempo real/contínuo** e **transita para turnos** ao iniciar combate ("when the swords are unsheathed things get tactical"); superfícies/elevação viram variáveis táticas. A transição é descrita como "seamless".
Fonte: <https://en.wikipedia.org/wiki/Baldur%27s_Gate_3>

**[FATO]** **Divinity: Original Sin 2** usa os **mesmos mapas isométricos** para combate e exploração; "não lutar sobre um grid visível ajuda a exploração a ser seamless" — i.e., o grid tático é **lógico**, sobreposto ao espaço contínuo, não uma arena separada.
Fonte: <https://www.gamedev.net/forums/topic/681684-tactics-rpgs-with-an-actual-game-world-exploration-etc/>

**[FATO]** **Dofus/Wakfu** usam grid de células com Pontos de Movimento/Ação; crítica conhecida: com o cenário persistindo no combate, pilares/objetos **obscurecem células** e dificultam leitura tática — por isso oferecem um **"modo tático"** que vira o mapa num tabuleiro de xadrez com detalhes removidos. Isto evidencia que o grid tático é uma **camada lógica instanciável** sobre o mundo.
Fontes: <https://krosmoz.fandom.com/wiki/Dofus_(MMO)>, <https://www.wakfu.com/en/forum/8-general-discussion/94355-tactical-mode-superiority>

**[HIPÓTESE]** Padrão recomendado: ao entrar em combate, **instancie um grid tático** (origem, tamanho de célula, extensão) **sobre** o espaço contínuo. Projete posições contínuas → célula (`floor((pos - origem)/cell)`) e o movimento tático passa a snap-ar para centros de célula, reusando a **caminhabilidade por célula** (derivada da mesma fonte de dados da exploração). Exploração e combate compartilham **colisão/altura**, divergem só na **regra de movimento** (contínuo vs discreto por MP). Sem conflito de sistemas porque a camada de dados é única.

### 5. Crates Rust/Bevy

**Colisão 2D / física:**
- **[FATO]** **`avian2d`** (ex-bevy_xpbd): física escrita para Bevy, usa a **ECS diretamente** (sem "mundo de física" separado) → menos overhead e integração nativa; código mais inspecionável.
  Fontes: <https://docs.rs/avian2d>, <https://taintedcoders.com/bevy/physics/avian>, <https://github.com/janhohenheim/avian/blob/main/README.md>
- **[FATO]** **`bevy_rapier2d`** (wrapper do Rapier): **mais maduro e testado**, mantém mundo de física separado sincronizado com a ECS a cada frame; pode rodar fora do Bevy.
  Fontes: <https://www.libhunt.com/compare-avian-vs-rapier>, <https://thebevyflock.github.io/bevy-quickstart-book/2-basics/actual-physics.html>
- **[FATO]** APIs centrais similares → migrar entre os dois é relativamente simples.
  Fonte: <https://docs.rs/avian2d>

**Pathfinding / navmesh:**
- **[FATO]** **`vleue_navigator`** (ex-`bevy_pathmesh`): navmesh para Bevy via **Polyanya**, algoritmo **any-angle** (caminhos ótimos em qualquer ângulo, não travados no grid).
  Fontes: <https://docs.rs/vleue_navigator/latest/vleue_navigator/>, <https://github.com/vleue/polyanya>
- **[FATO]** **`oxidized_navigation`**: geração de navmesh **em runtime, tiled**, estilo Recast, para mundos 3D; consome colliders de entidades com `NavMeshAffector` e gera tiles assíncronos consultáveis por `query::find_path`.
  Fonte: <https://docs.rs/oxidized_navigation/latest/oxidized_navigation/>
- **[FATO]** **`pathfinding`** / **`hierarchical_pathfinding`** (HPA* + Dijkstra hierárquico em grid) / **`bracket-pathfinding`** (A* + Dijkstra, do bracket-lib): opções genéricas em grid.
  Fontes: <https://docs.rs/hierarchical_pathfinding/latest/hierarchical_pathfinding/>, <https://crates.io/crates/bracket-pathfinding/0.8.2>

---

## Alternativas (tabela)

### Representação de caminhabilidade/colisão

| Abordagem | Movimento | Colisão | Prós | Contras |
|---|---|---|---|---|
| Grid puro (estilo GAT/Dofus) | Discreto por célula | Flag por célula (O(1)) | Simples, barato, nativo p/ tático; obstáculos dinâmicos triviais | Não entrega movimento contínuo desejado |
| Tiles + sub-tile walkmap (2×2/4×4 + triângulos) | **Contínuo** | Walkmap de alta-res + swept AABB/sliding | Mantém dados de tile; colisão barata; porta p/ grid tático | Autoria de walkmap; diagonais exigem triângulos |
| Física geométrica (avian2d/rapier) sobre colliders de tile | **Contínuo** | Colliders reais | Sliding/empilhamento "de graça"; dinâmicos | Mais pesado p/ MMO server-side com muitos agentes |

### Navegação / pathfinding

| Abordagem | Crate Rust | Melhor para | Custo com muitos agentes | Dinâmicos |
|---|---|---|---|---|
| Grid A* / HPA* | `pathfinding`, `hierarchical_pathfinding`, `bracket-pathfinding` | Destinos únicos, mundo tileado, blockers em runtime | O(N log N) por agente (HPA* mitiga) | Excelente (O(1) p/ marcar célula) |
| Navmesh (any-angle) | `vleue_navigator` (Polyanya), `oxidized_navigation` | Geometria orgânica, caminhos lisos, mundo estático | Barato (grafo esparso) | Rebake custa por região |
| Flow fields | (custom; sobre grid) | **Muitos agentes → destino comum** | O(N log N) 1×, O(1)/agente | Bom (recomputa campo) |

### Separação exploração × combate

| Padrão | Exemplo | Como | Risco |
|---|---|---|---|
| Arena instanciada separada | Dofus/Wakfu | Carrega cena de combate dedicada | Quebra continuidade; crítica central do projeto |
| Grid tático lógico sobre espaço contínuo | DOS2, BG3 (turnos) | Sobrepõe grid na hora do combate; mesma geometria | Precisa projeção pos↔célula coerente |
| Sem grid tático (combate 100% contínuo) | DOS2 (posicional s/ grid) | Posições livres + alcances em distância | Perde "tático por células" explícito |

---

## Recomendação

**[HIPÓTESE / recomendação de design]**

1. **Camada de dados única, estilo GAT repensado.** Autore o mundo como tiles; por tile guarde **4 alturas de canto** (heightfield p/ o 2.5D, interpolação bilinear) + **flags de terreno/caminhabilidade** (caminhável, bloqueado, água, "cliff atravessável por projétil"). Esta é a mesma fonte de verdade para exploração e combate.

2. **Movimento contínuo com walkmap sub-tile + colisão própria.** Posição é `Vec2/Vec3` contínuo. Resolva colisão com **swept AABB + sliding** contra uma **walkmap de alta resolução** (2×2 ou 4×4 por tile, triângulos nas diagonais). Trate a física pesada (`avian2d`) como opcional p/ objetos dinâmicos — não como base do movimento de todos os agentes no servidor.

3. **Navegação híbrida.** Para NPCs/agentes com destinos únicos: **grid A* hierárquico** (`hierarchical_pathfinding`) sobre a walkmap — barato de atualizar com blockers dinâmicos, casa com a estrutura de tiles. Para movimento em massa rumo a um ponto comum (hordas, invasões): **flow fields** + evasão local (ORCA/separação). Reserve **navmesh** (`vleue_navigator`/Polyanya, any-angle) se/quando a geometria ficar orgânica demais p/ grid; ela dá os caminhos mais lisos.

4. **Combate tático = grid lógico instanciado sobre o contínuo** (padrão BG3/DOS2, não arena separada). Ao entrar em combate, instancie um grid (origem+cell size), projete `pos→célula`, snap-e movimento a centros e reaproveite a caminhabilidade por célula derivada da walkmap. Zero duplicação de dados de mundo; só troca a **regra de movimento**.

5. **Crates:** começar com **`avian2d`** (integração ECS nativa, menos overhead; APIs próximas às do Rapier → migração fácil se precisar da maturidade do `bevy_rapier2d`), **`hierarchical_pathfinding`** p/ A*/HPA* em grid, e manter **`vleue_navigator`** no radar p/ navmesh any-angle.

---

## Questões em aberto

1. **[HIPÓTESE]** **Server-authoritative vs client:** colisão/pathfinding de muitos agentes precisa rodar barato **no servidor**. Física geométrica completa (avian/rapier) por agente pode não escalar; a walkmap + grid A* provavelmente escala melhor — **falta benchmark** no nosso caso (nº de agentes-alvo por mapa).
2. **[FATO/risco]** `oxidized_navigation` é focado em **3D**; e no Godot os agentes de navegação ainda são marcados **Experimental** — sinal de que navmesh dinâmico em engines ainda é terreno instável. Em Bevy/Rust a maturidade de navmesh dinâmico precisa ser validada. Fonte: <https://vav-labs.com/blog/godot-pathfinding-grid-vs-navmesh/>, <https://docs.rs/oxidized_navigation/latest/oxidized_navigation/>
3. **[HIPÓTESE]** **Resolução da walkmap sub-tile** (2×2? 4×4?) e tratamento de **diagonais/altura** (quão íngreme é intransponível) são parâmetros não resolvidos; afetam custo de memória, qualidade de colisão e fidelidade do grid tático.
4. **[HIPÓTESE]** **Projeção pos↔célula no combate:** definir cell size tático, o que acontece com agentes "entre células" ao iniciar combate (snap? arredonda?), e como alcances/áreas de efeito contínuos mapeiam para células.
5. **[HIPÓTESE]** **Multi-tamanho de agentes** (bosses grandes) em grid exige clearance maps; precisa decisão se o mundo terá agentes de tamanhos diversos.

---

## Fontes

**GAT / Ragnarok Online**
- <https://ragnarokresearchlab.github.io/file-formats/gat/>
- <https://openkore.com/wiki/Field_file_format>
- <https://github.com/open-ragnarok/roint/blob/master/gat.c>
- <https://github.com/rdw-archive/RagnarokFileFormats>
- <https://github.com/Duckwhale/RagnarokFileFormats/blob/master/GND.MD>

**Movimento contínuo sobre tiles / colisão sub-tile**
- <https://github.com/Abaudat/browser-city/issues/56>
- <https://github.com/APKiwiOrg/KhaozEngine/issues/1219>
- <https://gamedev.net/forums/topic/178857-smooth-tile-based-movement/>
- <https://qbmikehawk.neocities.org/articles/tileworld/>

**Pathfinding: navmesh / grid / flow fields**
- <https://vav-labs.com/blog/godot-pathfinding-grid-vs-navmesh/>
- <https://moonjump.com/game-dev-mechanics-flow-fields-how-it-works/>
- <https://github.com/Vincent-VD/Flow-Fields-Nav>
- <https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter27_Tactical_Pathfinding_on_a_NavMesh.pdf>
- <https://recited.io/kb/ai-in-game-development/pathfinding-and-navigation/navigation-meshes/>

**Exploração contínua × combate tático**
- <https://en.wikipedia.org/wiki/Baldur%27s_Gate_3>
- <https://www.gamedev.net/forums/topic/681684-tactics-rpgs-with-an-actual-game-world-exploration-etc/>
- <https://krosmoz.fandom.com/wiki/Dofus_(MMO)>
- <https://www.wakfu.com/en/forum/8-general-discussion/94355-tactical-mode-superiority>

**Crates Rust/Bevy**
- <https://docs.rs/avian2d>
- <https://taintedcoders.com/bevy/physics/avian>
- <https://github.com/janhohenheim/avian/blob/main/README.md>
- <https://www.libhunt.com/compare-avian-vs-rapier>
- <https://thebevyflock.github.io/bevy-quickstart-book/2-basics/actual-physics.html>
- <https://docs.rs/vleue_navigator/latest/vleue_navigator/>
- <https://github.com/vleue/polyanya>
- <https://docs.rs/oxidized_navigation/latest/oxidized_navigation/>
- <https://docs.rs/hierarchical_pathfinding/latest/hierarchical_pathfinding/>
- <https://crates.io/crates/bracket-pathfinding/0.8.2>
