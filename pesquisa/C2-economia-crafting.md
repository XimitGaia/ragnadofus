# C2 — Economia & Crafting: Fundamentação da RFC

> MMORPG inspirado na profundidade econômica de Dofus.
> Loop jogável do MVP: **COLETA → CRAFTING** (colher recurso → fabricar item → persistir no servidor → visível a outros jogadores).
> Data: 2026-11-14.
>
> **Convenção:** cada afirmação é marcada como **[FATO]** (com fonte/URL) ou **[HIPÓTESE/DESIGN]** (opinião de design nossa ou inferência não comprovada por fonte primária).

---

## 1. Problema

Precisamos fundamentar uma RFC de economia/crafting que entregue, no MVP, um loop **coleta → craft** minimamente profundo e **interdependente entre jogadores**, com persistência autoritativa no servidor — sem cair em dois extremos:

- **Raso demais:** craft que é só "aperta o botão", sem cadeia produtiva nem razão para os jogadores dependerem uns dos outros. Perde a alma econômica de Dofus.
- **Tedioso demais:** grind de coleta repetitiva como muralha artificial, que afasta jogadores casuais e trava a economia já no MVP.

As quatro perguntas da RFC: (1) como Dofus estrutura profissões/coleta/receitas/ferramentas e produção interdependente — o que o torna profundo vs. tedioso; (2) princípios de design de economias de MMO (sinks/sources, inflação, player-driven vs NPC-driven, interdependência) — lições de EVE, Albion e Dofus; (3) modelo de dados mínimo e o que precisa ser **autoritativo no servidor** para evitar cheat; (4) como manter o loop interdependente sem virar grind já no MVP.

---

## 2. Achados (com URLs)

### 2.1 Como Dofus estrutura profissões, coleta, receitas e interdependência

- **[FATO]** Dofus tem **profissões divididas em duas categorias: coleta (harvesting) e craft**. Profissões de coleta reúnem matéria-prima no mundo (lenhador corta madeira, fazendeiro colhe cereais); profissões de craft transformam recursos em itens (padeiro, açougueiro, joalheiro, sapateiro, alfaiate, ferreiro de escudo, etc.). Fonte: https://en.wikipedia.org/wiki/Dofus ; https://dofuswiki.fandom.com/wiki/Profession
- **[FATO]** São **19 profissões**, e um mesmo personagem pode subir qualquer uma até o **nível 200**. Fonte (resumo de busca de dofuswiki/kamastrade): https://kamastrade.net/en/guides/dofus-profession-levelling ; https://dofuswiki.fandom.com/wiki/Profession
- **[FATO]** **XP de profissão** vem de ações distintas por tipo: profissões de coleta ganham XP coletando recursos; profissões de craft ganham XP **apenas fabricando itens**. Fonte: https://en.wikipedia.org/wiki/Dofus ; https://dofuswiki.fandom.com/wiki/Profession
- **[FATO]** **Receitas escalam com o nível:** no nível 1 uma receita tem no máximo ~2 ingredientes; slots adicionais de ingredientes se desbloqueiam conforme a profissão sobe. Com os slots corretos desbloqueados, o craft tem **100% de chance de sucesso**. Fonte (resumo de busca): https://kamastrade.net/en/guides/dofus-profession-levelling
- **[FATO]** **Ferramentas são necessárias para coletar** (ex.: machado para lenhador, picareta para minerador). Fonte: https://blog.igv.com/en/dofus-unity-the-ultimate-guide-to-harvesting-professions/
- **[FATO]** A **interdependência emerge da especialização + escassez:** coletores precisam suprir material; crafters transformam em bens úteis; isso força os dois tipos de jogador a interagir (via mercado). Pares clássicos de otimização: **lenhador + entalhador (carver)**, **minerador + ferreiro**, para reduzir o custo de subir a profissão de craft coletando o próprio insumo em vez de comprá-lo. Fonte: https://en.wikipedia.org/wiki/Dofus ; https://blog.igv.com/en/dofus-unity-the-ultimate-guide-to-harvesting-professions/
- **[FATO]** **Mercados (HDV):** há mercados em cidades onde se compra/vende bens e equipamento por uma taxa, com Kamas depositados na conta. Esse "marketplace" sustenta a economia dirigida por jogadores. Fonte: https://en.wikipedia.org/wiki/Dofus
- **[FATO]** **Profundidade de alto nível (Forgemagie):** um sistema de encantamento/transformação de atributos de equipamento que **consome runas**; acessível a partir de certo nível de profissão (ex.: nível 65). Quebrar equipamentos gera runas, e esse consumo de runas sustenta o valor delas — um **sink de itens** que mantém mercados lucrativos. Fonte: https://dafous.app/guides/economie-mmorpg.html ; https://huzounet.fr/guides/forgemagie
- **[HIPÓTESE/DESIGN]** O que torna Dofus **profundo**: cadeias longas de produção (coleta → refino → craft → forgemagie), a necessidade de combinar múltiplas profissões (próprias ou de outros jogadores), e sinks (runas/forgemagie) que dão sentido econômico aos insumos. O que o torna **tedioso**: subir crafting só fabricando itens (que exige volume alto de insumos coletados repetidamente) e a coleta de recursos ponto-a-ponto como grind de respawn. (Inferência a partir das fontes acima + 2.4.)

### 2.2 Design de economias de MMO: sinks/sources, inflação, player-driven vs NPC-driven

- **[FATO]** Numa economia fechada, o **balanço entre sinks e faucets (sources) define a taxa de inflação** e, portanto, o poder de compra da moeda. Em EVE, os sinks de ISK incluem taxas de transação de mercado, broker fees e seguro de navios. Fonte: https://www.engadget.com/2010-10-24-eve-evolved-isk-sinks-and-faucets.html ; https://wiki.eveuniversity.org/Economics
- **[FATO]** **Sinks devem escalar com a riqueza do jogador** — sinks de valor fixo ficam irrelevantes à medida que os jogadores acumulam riqueza. Fonte: https://medium.com/@msahinn21/designing-game-economies-inflation-resource-management-and-balance-fa1e6c894670
- **[FATO]** EVE é **player-driven**: a maior parte dos itens é fabricada por jogadores a partir de minerais minerados por jogadores e vendidos num mercado ativo; o controle quase total da economia pelos jogadores é mecanismo básico do jogo. CCP mantém **economista profissional** e publica **relatórios econômicos mensais**. Fonte: https://wiki.eveuniversity.org/Economics ; https://en.wikipedia.org/wiki/Dofus (comparativo)
- **[FATO]** **Destruição é o principal motor da economia de EVE** (o maior sink de itens). Em janeiro/2021, **55,34 trilhões de ISK em navios/estruturas destruídos**, forçando recompras; mineração ~20,4 tri e produção ~109,6 tri de ISK em valor — ou seja, **a destruição estimula demanda por mineração e produção**. Fonte: https://tagn.wordpress.com/2021/02/19/the-january-eve-online-economic-report-shows-destruction-and-mineral-prices-soaring/
- **[FATO]** **Albion Online tem economia totalmente player-driven:** praticamente **todo equipamento é fabricado por jogadores**, em construções feitas por jogadores, a partir de recursos coletados por jogadores (coleta → refino → craft → venda). Fonte: https://en.wikipedia.org/wiki/Albion_Online ; https://albiononline.com/home
- **[FATO]** Albion usa **mercados locais** (um item posto à venda num local só aparece naquele local) e **bônus de produção local** por cidade/região, criando deliberadamente loops de "coletar faz sentido numa região, refinar/craftar vale mais na cidade com bônus de outra região" — o que **força comércio entre mercados e especialização regional**. Fonte (resumo de busca + medium): https://medium.com/@kokopellim/albion-online-craft-conquer-and-create-your-own-adventure-in-this-player-driven-mmorpg-a3ddc10cfee0
- **[FATO]** Albion tem **full-loot drop na morte em zonas vermelhas/pretas** — a destruição/perda de gear é um **sink de itens** que sustenta a demanda contínua por craft. Fonte: https://en.wikipedia.org/wiki/Albion_Online
- **[HIPÓTESE/DESIGN]** Lição transversal (EVE + Albion + Dofus): **economia saudável = fonte contínua de demanda por itens craftados** (destruição/consumo) + **sinks de moeda que escalam** + **especialização que impede autossuficiência total**. Sem um sink de itens (consumo/destruição), o craft satura e os preços colapsam; sem sink de moeda, infla.

### 2.3 Modelo de dados e autoridade do servidor (anti-cheat)

- **[FATO]** Regra fundamental de MMO: **o servidor é autoritativo.** O cliente envia *intenções* ("coletei este nó", "craftei este item"); o servidor valida contra o estado do jogo (regras, cooldowns, inventário, posição, timing) antes de aceitar. Fonte: https://nhimg.org/faq/how-should-game-teams-validate-player-actions-to-prevent-multiplayer-cheating ; https://www.metaplay.io/comparisons/server-authoritative-vs-client-authoritative
- **[FATO]** "**Todo cheat que já funcionou rastreia-se a uma decisão: deixar o cliente decidir algo que o servidor deveria ter decidido.** Se o cliente valida as próprias mutações de inventário, você tem trainers de 'spawnar qualquer item'." Mutações de economia/inventário devem ser **server-side e atômicas**; cada mudança de moeda, concessão de item, craft e troca é uma **transação que o servidor possui**. Fonte: https://crux.supercraft.host/blog/server-authoritative-anti-cheat-backend/
- **[FATO]** Inventário/craft server-authoritative na prática: o servidor decide **consumo de itens no craft, disponibilidade de recurso antes da transação, sucesso/falha do craft e todas as mudanças de inventário**; operações de craft devem ser **atômicas** (completam totalmente ou revertem) para evitar race conditions de "craftar múltiplos itens com o mesmo recurso". Fonte: https://gamecoderstudios.com/how-we-built-a-worldwide-inventory-sync-for-a-crafting-heavy-mmo/
- **[FATO]** **Nós de recurso precisam de exclusividade/claim no servidor:** o servidor deve impedir que dois jogadores coletem o mesmo nó, exigindo verificação server-side antes de concluir a aquisição. Recomenda-se **log de transações** para auditoria e propagação quase em tempo real para evitar estado obsoleto explorável. Fonte: https://gamecoderstudios.com/how-we-built-a-worldwide-inventory-sync-for-a-crafting-heavy-mmo/
- **[FATO]** **Client-side prediction + reconciliation:** cliente pode prever o resultado para fluidez, mas se a conta do servidor divergir, **o servidor sobrepõe e "snap-back" ao estado validado**. Fonte: https://devforum.roblox.com/t/release-a-free-solution-to-ping-for-server-authoritative-data-introducing-mirage/4757963

### 2.4 Como evitar que o loop vire grind tedioso

- **[FATO]** Padrão comum de falha: tratar **craft tedioso como "feature"** (se doer o bastante, casuais não participam e poucos dedicados lucram) — mas o grind "molda quem participa da economia" e afasta jogadores. "Juntar pedacinhos para fazer mais pedacinhos e combinar no mesmo item que todo mundo faz" é descrito como "tédio encarnado". Fonte: https://forums.mmorpg.com/discussion/423279 ; https://finalboss.io/most-mmo-players-arent-crafting-to-get-rich
- **[FATO]** O que os jogadores de craft **realmente querem** (não é ficar rico): **(1) autossuficiência** ("fazer meu próprio best-in-slot sem depender de RNG de drop"); **(2) identidade/expressão** (cosméticos, ver o nome na criação); **(3) satisfação do processo** (a transformação material → item útil). Recomendações: manter itens craftados **relevantes por várias tiers**, permitir customização sem pré-requisitos grindy, e tornar o **engajamento inicial recompensador**. Fonte: https://finalboss.io/most-mmo-players-arent-crafting-to-get-rich
- **[FATO]** Recomendação de pacing: **espalhar a coleta por uma variedade de ações** em vez de repetir uma só; dividir o tempo entre combate e coleta/farming para evitar o tédio que gera a sensação de grind. Fonte (resumo de busca): https://forums.mmorpg.com/discussion/423279
- **[HIPÓTESE/DESIGN]** Para o MVP: a interdependência (não a repetição) deve ser a fonte de profundidade. Cadeias **curtas mas cruzadas** (o craft de A exige insumo coletado por B) já criam comércio sem exigir dezenas de horas de grind.

---

## 3. Modelo mínimo para o MVP

**Escopo do loop mínimo (recomendado):** 2 profissões de coleta + 2 de craft, com **cruzamento obrigatório** de insumos (o craft de uma profissão consome recurso coletado por outra), 1 estação de craft, mercado assíncrono simples, tudo persistido e autoritativo no servidor.

### 3.1 Entidades de dados (mínimas)

| Entidade | Campos essenciais | Autoridade |
|---|---|---|
| **ResourceNode** | `id`, `type`, `position`, `state` (disponível/esgotado), `respawn_at` (timestamp), `tool_required`, `yield` | **Servidor** — estado e respawn só no servidor |
| **InventoryStack** | `owner_id`, `item_id`, `qty` (stack por conta) | **Servidor** — débito/crédito atômico |
| **ItemType** | `id`, `name`, `stackable`, `tags` (recurso bruto / refinado / equipamento) | Servidor (definição) |
| **Recipe** | `id`, `profession`, `level_req`, `inputs[{item_id, qty}]`, `outputs[{item_id, qty}]`, `station_type` | Servidor (catálogo imutável no cliente) |
| **CraftingStation** | `id`, `type`, `position`, (opcional) `owner_id` | Servidor |
| **ProfessionProgress** | `player_id`, `profession`, `level`, `xp` | **Servidor** — XP só concedido por ação validada |
| **TransactionLog** | `id`, `actor`, `action` (harvest/craft/trade), `delta`, `timestamp` | **Servidor** — auditoria/anti-cheat |

### 3.2 O que PRECISA ser autoritativo no servidor (anti-cheat)

- **[FATO-fundamentado]** **Coleta:** o cliente só envia "quero coletar nó X". O servidor valida: nó existe, está disponível, jogador tem a ferramenta, está no alcance, e então **marca o nó como esgotado, agenda respawn e credita o inventário** — atomicamente. Claim exclusivo evita dois jogadores colhendo o mesmo nó.
- **[FATO-fundamentado]** **Craft:** cliente envia "craftar receita R na estação E". Servidor valida nível de profissão, presença/qtd dos inputs, proximidade da estação; **consome inputs e credita outputs numa transação atômica** (rollback total em falha). Sucesso/falha decidido no servidor.
- **[FATO-fundamentado]** **Inventário e moeda:** toda mutação é transação server-side; nada que apareça do cliente é aceito sem validação. Cliente pode prever para fluidez, mas servidor faz **snap-back** se divergir.
- **[FATO-fundamentado]** **XP de profissão:** só concedido como efeito de uma ação de coleta/craft validada no servidor.
- **[HIPÓTESE/DESIGN]** **Mercado assíncrono mínimo** (listar/comprar): também autoritativo, com escrow do item/moeda no servidor; pode ser um "balcão" simples no MVP, sem leilão.

### 3.3 Recomendação do loop mínimo

**[HIPÓTESE/DESIGN]** 1 cadeia cruzada concreta para o MVP:
`Minerador coleta Minério` + `Lenhador coleta Madeira` → `Ferreiro (precisa de Minério refinado) craft Lâmina` + `Entalhador (precisa de Madeira) craft Cabo` → `item final = Minério + Madeira`. Nenhuma profissão é autossuficiente: um item final exige insumos de **duas cadeias de coleta distintas**, gerando comércio desde o minuto 1, com **cadeias curtas** (1–2 passos) para evitar grind.

---

## 4. Extensões futuras (pós-MVP)

- **[HIPÓTESE/DESIGN]** **Sink de itens / destruição** (lição EVE+Albion): consumo/quebra de equipamento, durabilidade, ou um análogo da **Forgemagie** (consumir runas obtidas ao quebrar itens) para sustentar demanda contínua por craft e evitar saturação de mercado.
- **[HIPÓTESE/DESIGN]** **Sinks de moeda que escalam** com a riqueza (taxas de mercado, broker fees, impostos de craft) para controlar inflação — padrão EVE.
- **[HIPÓTESE/DESIGN]** **Mercados locais + bônus de produção regional** (padrão Albion) para forçar logística e especialização geográfica.
- **[HIPÓTESE/DESIGN]** **Forgemagie/encantamento** e receitas com variância de qualidade (RNG controlado) para profundidade de alto nível e identidade do crafter.
- **[HIPÓTESE/DESIGN]** **Relatórios econômicos internos** (dashboard de sinks/faucets) desde cedo — baratos de instrumentar via `TransactionLog` e essenciais para tunar inflação (padrão CCP/EVE).
- **[HIPÓTESE/DESIGN]** Expandir de 4 para as ~19 profissões ao estilo Dofus, com slots de ingredientes desbloqueáveis por nível.

---

## 5. Questões em aberto

1. **[INCERTEZA]** **Qual é o sink de itens do MVP?** Sem destruição/consumo, o mercado sature e preços colapsam (lição EVE/Albion). Vale introduzir durabilidade/quebra já no MVP, ou fica para pós-MVP aceitando saturação temporária?
2. **[INCERTEZA]** **Mercado no MVP: existe ou não?** A visibilidade "visível a outros jogadores" é obrigatória, mas troca/mercado econômico talvez não. Interdependência sem mercado (ex.: troca direta) é suficiente para o loop, ou precisamos de um balcão assíncrono mínimo já na v1?
3. **[INCERTEZA]** **Modelo de nó de recurso: instanciado por jogador ou compartilhado/contestado?** Nó compartilhado (claim exclusivo + respawn) cria escassez e interdependência reais, mas introduz contenção e necessidade de balanceamento de respawn; nó instanciado remove tédio/competição mas enfraquece a economia. Qual escolher no MVP?
4. **[INCERTEZA]** Fontes primárias de Dofus (devblog oficial, wiki) ficaram parcialmente bloqueadas (HTTP 402/403); números específicos (ex.: nível 65 da Forgemagie, 19 profissões) vêm de wikis/guias comunitários e resumos de busca — **confirmar contra documentação oficial antes de citar na RFC final**.

---

## 6. Fontes

**Dofus — profissões/craft/economia**
- https://en.wikipedia.org/wiki/Dofus
- https://dofuswiki.fandom.com/wiki/Profession
- https://kamastrade.net/en/guides/dofus-profession-levelling
- https://blog.igv.com/en/dofus-unity-the-ultimate-guide-to-harvesting-professions/
- https://dafous.app/guides/economie-mmorpg.html
- https://huzounet.fr/guides/forgemagie

**Economia de MMO — sinks/sources/inflação/player-driven**
- https://www.engadget.com/2010-10-24-eve-evolved-isk-sinks-and-faucets.html
- https://wiki.eveuniversity.org/Economics
- https://tagn.wordpress.com/2021/02/19/the-january-eve-online-economic-report-shows-destruction-and-mineral-prices-soaring/
- https://medium.com/@msahinn21/designing-game-economies-inflation-resource-management-and-balance-fa1e6c894670
- https://en.wikipedia.org/wiki/Albion_Online
- https://albiononline.com/home
- https://medium.com/@kokopellim/albion-online-craft-conquer-and-create-your-own-adventure-in-this-player-driven-mmorpg-a3ddc10cfee0

**Autoridade do servidor / anti-cheat / modelo de dados**
- https://crux.supercraft.host/blog/server-authoritative-anti-cheat-backend/
- https://gamecoderstudios.com/how-we-built-a-worldwide-inventory-sync-for-a-crafting-heavy-mmo/
- https://nhimg.org/faq/how-should-game-teams-validate-player-actions-to-prevent-multiplayer-cheating
- https://www.metaplay.io/comparisons/server-authoritative-vs-client-authoritative
- https://devforum.roblox.com/t/release-a-free-solution-to-ping-for-server-authoritative-data-introducing-mirage/4757963

**Grind vs. engajamento**
- https://finalboss.io/most-mmo-players-arent-crafting-to-get-rich
- https://forums.mmorpg.com/discussion/423279/the-core-of-crafting-is-customizing-or-gather-materials-grind
