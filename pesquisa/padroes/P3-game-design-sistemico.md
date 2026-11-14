# P3 — Game Design Sistêmico: Princípios de Design do RagnaDofus

> Brief de fundamentação dos **PRINCÍPIOS DE DESIGN** de um RPG online de mundo contínuo com economia sistêmica profunda.
> Inspiração declarada: **exploração viva de Ragnarok Online + profundidade econômica/tática de Dofus**.
> Pesquisa web multi-fonte (GDC, postmortems, dev blogs, teoria) realizada em **2026-11-14**.
>
> **Convenção de marcação:** cada afirmação é **[FATO/FONTE]** (com URL de fonte primária ou reputável) ou **[OPINIÃO DE DESIGN]** (nossa inferência/posição de design ou opinião de praticante não comprovada como fato objetivo).
>
> Complementa o brief **C2 — Economia & Crafting** (mesma pasta `pesquisa/`), que detalha o loop coleta→craft e a autoridade de servidor; aqui o foco é a **camada de princípios de design**, não implementação.

---

## 1. Contexto

Estamos definindo os pilares de design de um MMORPG 2.5D que combina duas heranças aparentemente conflitantes:

- **Ragnarok Online** — um mundo contínuo e vivo onde você vê outros jogadores farmando, "MVP trains" se formando e eventos acontecendo em tempo real; exploração espacial casual e gregária. **[FATO/FONTE]** https://en.wikipedia.org/wiki/Ragnarok_Online
- **Dofus** — profundidade **econômica** (cadeias de produção interdependentes, mercados player-driven) e **tática** (combate por turnos em grade, com pontos de ação/movimento). **[FATO/FONTE]** https://en.wikipedia.org/wiki/Dofus

O desafio central deste brief é fundamentar **COMO** essas heranças coexistem sem se anularem, e estabelecer um conjunto pequeno e rígido de princípios que mantenham o jogo coerente à medida que cresce. Cinco perguntas de design orientam a pesquisa:

1. Economia de MMO: sinks/sources, player-driven vs NPC, inflação, interdependência, mercados (lições de EVE/Albion/UO).
2. Mundo vivo sistêmico: ecossistemas, recursos, eventos emergentes, simulação sem jogadores.
3. Progressão horizontal vs vertical; classes/builds; grind com intenção.
4. Exploração espacial contínua + profundidade tática de combate sem conflito.
5. Pilares de design: definir identidade e manter coerência ao crescer.

A **lição transversal mais importante** que emergiu da pesquisa: os jogos que entregaram "mundo vivo" durável quase sempre **trapacearam a simulação completa** (LOD, diretores de evento, geração procedural, ou — o padrão mais robusto — fizeram dos *próprios jogadores* a simulação econômica). Simulação autônoma "de verdade" (a ecologia de Ultima Online) é cara e instável. **[OPINIÃO DE DESIGN]** (síntese das fontes abaixo)

---

## 2. Fontes de alta qualidade (URLs)

### Economia de MMO
- GDC 2014 — Vili Lehdonvirta, "Economic Balancing and Improved Monetization Through Clever Sink Design" (PDF): https://media.gdcvault.com/GDC2014/Presentations/Lehdonvirta_Vili_Economic_Balancing_and.pdf
- CCP Economist / "pure capitalist market" (Game Developer): https://www.gamedeveloper.com/game-platforms/ccp-economist-on-i-eve-online-i-s-pure-capitalist-market
- EVE Monthly Economic Report (fonte primária, instrumentação): https://forums.eveonline.com/t/monthly-economic-report-april-2026/511406
- EVE University — Economics / sinks & faucets: https://wiki.eveuniversity.org/Economics
- Bots ameaçam a economia de EVE (PC Gamer): https://www.pcgamer.com/bots-are-threatening-eve-onlines-economy-and-players-are-fed-up/
- Albion Online — "A Player-Driven Economy" (Steam/dev) e postmortem de 10 anos (80.lv): https://store.steampowered.com/news/app/761890/view/4040248138384161369 ; https://80.lv/articles/how-albion-online-devs-built-a-sandbox-mmorpg-across-an-entire-decade
- "Designing Game Economies: Inflation, Resource Management, and Balance": https://medium.com/@msahinn21/designing-game-economies-inflation-resource-management-and-balance-fa1e6c894670

### Mundo vivo / sistêmico
- Raph Koster — UO's resource system (fonte primária, parte 1 e 2): https://www.raphkoster.com/2006/06/03/uos-resource-system/ ; https://www.raphkoster.com/2006/06/04/uos-resource-system-part-2/
- GDC 2018 — Classic Game Postmortem: Ultima Online (Garriott, Long, Koster, Vogel): https://gdcvault.com/play/1016629/Classic-Game-Postmortem-Ultima ; recap de Koster: https://www.raphkoster.com/2018/03/28/uo-postmortem-from-gdc2018/
- GDC — "Designing Guild Wars 2 Dynamic Events": https://gdcvault.com/play/1013691/Designing-Guild-Wars-2-Dynamic ; ArenaNet "Designing the Living World": https://www.guildwars2.com/en/news/designing-the-living-world/
- Soren Johnson — "Water Finds a Crack" (jogadores otimizam a diversão para fora): https://www.designer-notes.com/?p=369
- Dwarf Fortress — simulação offline do mundo (Game Developer): https://www.gamedeveloper.com/design/interview-the-making-of-dwarf-fortress ; "simulate the universe one step at a time": https://www.gamedeveloper.com/design/-i-dwarf-fortress-i-figuring-out-how-to-simulate-the-universe-one-step-at-a-time
- RimWorld — AI Storyteller (diretor de eventos): https://www.spacesector.com/blog/2013/10/rimworld-sci-fi-colonization-with-an-ai-storyteller/
- Far Cry 2 — por que NÃO simularam predadores: https://farcry.fandom.com/wiki/Far_Cry_2
- Simulation LOD para IA (tese TCD): https://www.scss.tcd.ie/publications/theses/diss/2016/TCD-SCSS-DISSERTATION-2016-055.pdf
- Instabilidade de modelos predador-presa agent-based: https://arxiv.org/pdf/2606.13639
- WoW phasing (persistência vs presença compartilhada): https://wowpedia.fandom.com/wiki/Phasing

### Progressão / builds / anti-grind
- Raph Koster — *A Theory of Fun* (fun = aprender; "boredom is the opposite of learning"): https://www.raphkoster.com/gaming/gdco12/Koster_Raph_Theory_Fun_10.pdf ; https://game-studies.fandom.com/wiki/A_Theory_of_Fun_for_Game_Design
- Guild Wars 2 — progressão horizontal (wiki Endgame; debate de gear): https://wiki.guildwars2.com/wiki/Endgame ; https://www.pcgamer.com/arenanet-guild-wars-2-gear-debate/
- Path of Exile — árvore passiva e diversidade de builds (Chris Wilson): https://en.wikipedia.org/wiki/Path_of_Exile ; https://www.fandom.com/articles/2050-interview-chris-wilson-on-path-of-exile-2s-origins
- Chris Wilson — "games designed to be fun, not manipulative" (GamesRadar): https://www.gamesradar.com/games/when-i-was-growing-up-games-were-designed-to-be-fun-not-manipulative-path-of-exile-co-creator-shines-a-blacklight-on-the-bulls-t-design-tricks-infesting-all-my-live-service-games/
- Diablo III Loot 2.0 (Josh Mosqueira, GDC 2015): https://www.purediablo.com/josh-mosqueira-diablo-3-presentation-gdc-2015
- GDC Online — retenção exige motivação real (Scott Rigby/SDT): https://www.gamedeveloper.com/game-platforms/gdc-online-player-retention-requires-real-motivation-engagement
- Celia Hodent — "The Gamer's Brain" (curva serrilhada, sentir competência): https://celiahodent.com/gamers-brain-part-3-ux-engagement-immersion-retention-gdc17-talk/
- EVE Online — treino de skill por tempo real/offline: https://wiki.eveuniversity.org/Skills_and_learning ; https://www.engadget.com/2008-11-23-eve-evolved-wheres-the-grind.html
- Dofus Spell Variants / Ankama Waven (diversidade intra-classe): https://dofuswiki.fandom.com/wiki/Class_spell ; https://en.wikipedia.org/wiki/Wakfu
- Albion — "you are what you wear" (classless): https://mmos.com/review/albion-online

### Exploração + combate tático
- Dofus/Wakfu combate tático (AP/MP, fase de posicionamento, aggro): https://wakfu.wiki.gg/wiki/Combat ; https://dofuswiki.fandom.com/wiki/Aggro
- Baldur's Gate 3 — transição explorar→turnos + toggle manual; porquê turn-based: https://en.wikipedia.org/wiki/Baldur%27s_Gate_3 ; https://bg3.wiki/wiki/Turn-based_mode ; https://www.rpgsite.net/interview/9494-baldurs-gate-iii-interview-discussing-combat-and-gameplay-with-larian-studios-designer-matt-holland
- Divinity: Original Sin 2 — "designing drama into turn-based combat" (Game Developer): https://www.gamedeveloper.com/design/designing-drama-into-the-turn-based-combat-of-i-divinity-original-sin-2-i-
- XCOM — deep dive em "pods" (mover = gatilho de encontro): https://www.gamedeveloper.com/design/a-deep-dive-into-xcom-and-xcom-2
- Chrono Trigger — combate on-map sem transição (contra-modelo): https://en.wikipedia.org/wiki/Chrono_Trigger

### Pilares / identidade
- GDC — Ian Schreiber, "The Four One-Page Design Docs" (Vision, Pillars, Loops, Resource Flow): https://gdcvault.com/play/1035120/The-Four-One-Page-Design
- Mark Rosewater — "Twenty Years, Twenty Lessons" (restrições geram criatividade; color pie): https://magic.wizards.com/en/news/making-magic/twenty-years-twenty-lessons-part-3-2016-06-13 ; https://www.gdcvault.com/play/1023186/
- CCP — "Three Design Pillars for Sandbox MMOs" (Social, Goal-driven, Emergent): https://www.gamedeveloper.com/design/ccp-online-s-three-design-pillars-for-sandbox-mmos
- Pilares como "firewall de scope creep" + non-goals: https://www.universityxp.com/blog/2025/2/13/game-design-constraints-and-scope-creep
- Star Wars Galaxies NGE (identidade destruída da noite para o dia): https://massivelyop.com/2015/08/22/the-game-archaeologist-star-wars-galaxies-nge/ ; defesa de Koster: https://www.raphkoster.com/2011/06/24/swg-is-shutting-down/
- "Um jogo para todos vira um jogo para ninguém" (crise de identidade de WoW): https://eu.forums.blizzard.com/en/wow/t/the-identity-crisis-of-retail-wow-midnight/638547

---

## 3. Princípios que adotamos

> Cada princípio sintetiza evidência e já aponta uma decisão de design acionável para o RagnaDofus. A numeração P3-x é citável em RFCs.

### Tema A — Economia sistêmica profunda

**P3-1. Os jogadores SÃO a simulação econômica; destruição/consumo é o motor.**
A economia saudável = demanda contínua por itens craftados (consumo/destruição) + sinks de moeda que escalam + especialização que impede autossuficiência total. EVE prova que a economia viva pode ser uma propriedade *emergente* da produção/destruição dos jogadores, não um faucet simulado que o dev opera. **[FATO/FONTE]** https://www.gamedeveloper.com/game-platforms/ccp-economist-on-i-eve-online-i-s-pure-capitalist-market ; Albion (full-loot cria demanda real) https://80.lv/articles/how-albion-online-devs-built-a-sandbox-mmorpg-across-an-entire-decade
→ **Decisão:** cada item craftado precisa de uma razão para ser *consumido* ou *perdido* (degradação/quebra em forgemagie, uso em receitas superiores, risco de perda). Ver sink de runas no C2 §2.1.

**P3-2. Sinks devem escalar com a riqueza; instrumentar, não ditar.**
Sinks de valor fixo ficam irrelevantes conforme a riqueza acumula — use sinks percentuais (taxas de mercado, broker fees, reparo indexado ao valor). E, como CCP, o trabalho do dev numa economia viva é **medir** (relatórios econômicos, faucets/sinks, velocidade da moeda), não impor preços. **[FATO/FONTE]** GDC Lehdonvirta (PDF acima); https://forums.eveonline.com/t/monthly-economic-report-april-2026/511406
→ **Decisão:** taxas de HDV/mercado percentuais desde o MVP + log de transações para um "mini-MER" interno. (Autoridade de servidor já coberta no C2 §2.3.)

**P3-3. Interdependência por especialização regional e de profissão — nunca autossuficiência total.**
Albion usa mercados *locais* + bônus de produção por região para forçar comércio e especialização. Dofus usa cadeias longas (coleta→refino→craft→forgemagie) que exigem combinar profissões próprias ou de outros. **[FATO/FONTE]** https://store.steampowered.com/news/app/761890/view/4040248138384161369 ; C2 §2.1-2.2
→ **Decisão:** nenhum jogador deve conseguir fechar sozinho a cadeia best-in-slot sem custo de oportunidade enorme; regionalizar recursos/bônus para criar rotas de comércio.

### Tema B — Mundo vivo sem cair na armadilha de Ultima Online

**P3-4. Não simule ecologia autônoma completa — "trapaceie" o mundo vivo.**
A ecologia de UO (modelo de necessidades/recursos real) morreu por **custo computacional** (pathfinding + buscas radiais escalam não-linearmente) antes mesmo do balanço, e os jogadores aceleraram o colapso caçando tudo e fazendo hoarding. Modelos predador-presa agent-based são matematicamente propensos a colapso/extinção sem tuning deliberado. Os jogos que entregaram "mundo vivo" durável trapacearam: LOD de simulação (Dwarf Fortress), diretor de eventos lendo o estado do jogador (RimWorld, GW2), geração procedural, ou fizeram dos jogadores a simulação (EVE/Albion). Far Cry 2 deliberadamente **não** simulou predadores pelo mesmo motivo. **[FATO/FONTE]** https://www.raphkoster.com/2006/06/03/uos-resource-system/ ; https://arxiv.org/pdf/2606.13639 ; https://farcry.fandom.com/wiki/Far_Cry_2 ; https://www.spacesector.com/blog/2013/10/rimworld-sci-fi-colonization-with-an-ai-storyteller/
→ **Decisão:** recursos e spawns do mundo governados por **timers/regras baratas com LOD** (simulação cara só perto de jogadores, abstração estatística longe), e eventos dinâmicos por um **diretor de eventos** que lê o estado do servidor — não um ecossistema autônomo. Combinar com P3-1 (os jogadores são a dinâmica econômica viva).

**P3-5. Surface a consequência; não gaste compute em simulação invisível.**
A armadilha recorrente (Koster: urso levando minutos para achar o mel que ninguém via; Adams: sistemas-base invisíveis) é gastar simulação no que o jogador não percebe. GW2 mostra o outro lado: eventos dinâmicos valem porque **encadeiam e deixam efeitos persistentes** — mas o excesso de marcadores vira sobrecarga de legibilidade, e persistência individual (phasing do WoW) custa presença compartilhada. **[FATO/FONTE]** https://gdcvault.com/play/1013691/Designing-Guild-Wars-2-Dynamic ; https://www.guildwars2.com/en/news/designing-the-living-world/ ; https://wowpedia.fandom.com/wiki/Phasing
→ **Decisão:** todo sistema simulado precisa de um *output visível e legível* no jogo (preço mudando, recurso esgotando num ponto visitável, evento encadeado). Como herança Ragnarok, priorizar que o mundo vivo seja **socialmente visível** (ver outros jogadores agindo), protegendo a presença compartilhada contra phasing agressivo.

### Tema C — Progressão

**P3-6. Progressão horizontal como espinha dorsal; vertical rasa e controlada.**
Fun = aprender e dominar padrões; "boredom is the opposite of learning" — grind é repetição *depois* que o aprendizado acabou. Progressão que só empilha números não ensina decisões novas. GW2 escolheu horizontal (teto de poder baixo, cresce em *opções*: masteries, specs, cosméticos) e provou ser viável no mainstream. Diablo III Loot 2.0 mostra o loop "com intenção": recompensas devem mudar *como você joga*, não só o número, e não invalidar o esforço anterior em bloco (o erro-cautela do treadmill de WoW). **[FATO/FONTE]** https://www.raphkoster.com/gaming/gdco12/Koster_Raph_Theory_Fun_10.pdf ; https://wiki.guildwars2.com/wiki/Endgame ; https://www.purediablo.com/josh-mosqueira-diablo-3-presentation-gdc-2015
→ **Decisão:** poder de equipamento com teto; crescimento sustentado via **largura** (novas profissões, builds, receitas, cosméticos). Cada nova recompensa deve introduzir uma *decisão* nova, não só +stat.

**P3-7. Diversidade de builds via restrição deliberada; evitar "one true build".**
PoE trata a árvore gigante como identidade (sinergias a descobrir); Ankama alcança diversidade intra-classe com Spell Variants (escolher base OU variante, estilo "deck") e evoluiu para mix-and-match (Waven). Albion é classless ("you are what you wear"), permitindo experimentação sem custo de respec. O risco a evitar (Soren Johnson): existir uma estratégia dominante que os jogadores otimizam até sugar a diversão. **[FATO/FONTE]** https://en.wikipedia.org/wiki/Path_of_Exile ; https://dofuswiki.fandom.com/wiki/Class_spell ; https://www.designer-notes.com/?p=369
→ **Decisão:** nenhuma build deve ser dominante em todas as situações (decisões "situacionalmente certas"); permitir respec/flexibilidade barata para incentivar experimentação. Decidir cedo o eixo classe vs classless (recomendação: classes com forte diversidade intra-classe, estilo Spell Variants).

**P3-8. Progresso deve ser *sentido* como competência, não como relógio — mas desacoplar progressão de horas puras onde fizer sentido.**
Retenção exige competência significativa, não recompensas arbitrárias; curva "serrilhada" e comparar habilidade antiga vs nova fazem o jogador *sentir* o avanço. EVE desacopla progressão de horas (treino por tempo real, até offline) — ótimo anti-grind e pró-casual, mas cuidado: jogadores pesados sentem o esforço "não recompensado". **[FATO/FONTE]** https://www.gamedeveloper.com/game-platforms/gdc-online-player-retention-requires-real-motivation-engagement ; https://celiahodent.com/gamers-brain-part-3-ux-engagement-immersion-retention-gdc17-talk/ ; https://wiki.eveuniversity.org/Skills_and_learning
→ **Decisão:** espalhar o progresso por ações variadas (não repetir uma só; ver C2 §2.4); explicitar ganhos de competência na UI. Considerar componentes de progressão por tempo/presença para profissões, como colchão anti-grind.

### Tema D — Exploração contínua + combate tático

**P3-9. A transição explorar→combate é um recurso DESENHADO e legível, não uma fricção.**
A tensão central é "custo de troca de modo vs profundidade tática". Dofus faz mode-switch *duro* (combate instanciado em grade) mas o torna legível: aggro telegrafado e level-gated + fase de posicionamento ~60s + economia tática clara (6 AP / 3 MP, 1 MP por tile). BG3 deixa o *jogador dirigir* a transição (auto em combate + toggle manual). XCOM funde mover=gatilho (pods), mas isso pode punir a experimentação espacial. Chrono Trigger prova que dá para combate on-map sem transição — ao custo de profundidade tática. **[FATO/FONTE]** https://wakfu.wiki.gg/wiki/Combat ; https://dofuswiki.fandom.com/wiki/Aggro ; https://bg3.wiki/wiki/Turn-based_mode ; https://www.gamedeveloper.com/design/a-deep-dive-into-xcom-and-xcom-2
→ **Decisão (seguir Dofus):** combate tático instanciado em grade, com **transição telegrafada** (aggro previsível + fase de posicionamento) e economia AP/MP legível. Isso resolve o conflito: exploração contínua (herança Ragnarok, social e espacial) e tática profunda (herança Dofus) ocupam *camadas distintas conectadas por um ritual legível*, em vez de competir no mesmo loop.

**P3-10. Faça a EXPLORAÇÃO alimentar a TÁTICA — posicionamento prévio como payload.**
DOS2 e BG3 tornam o ambiente explorável a própria variável tática (terreno/superfícies reativas, altura, verticalidade, reconhecimento e pré-posicionamento antes do "lock"). Os mesmos inimigos jogam de forma completamente diferente conforme o level design. Encontros visíveis/evitáveis (não aleatórios) combinam com mundo aberto e dão agência de pacing. **[FATO/FONTE]** https://www.gamedeveloper.com/design/designing-drama-into-the-turn-based-combat-of-i-divinity-original-sin-2-i- ; https://www.rpgsite.net/interview/9494-baldurs-gate-iii-interview-...-matt-holland
→ **Decisão:** terreno da grade de combate deriva do ponto do mundo onde o encontro começou (obstáculos, elevação, posições iniciais), recompensando exploração e escolha de *onde* lutar. Encontros visíveis e evitáveis, com pacing em "picos e vales".

### Tema E — Identidade e coerência

**P3-11. Poucos pilares rígidos + non-goals explícitos = firewall anti-scope-creep.**
Pilares são "tiebreakers" curtos e memorizáveis (2-4) contra os quais cada feature é medida; combinados com um conjunto explícito do que **não** faremos, transformam o GDD num firewall de escopo. CCP ancora EVE em três (Social, Goal-driven, Emergent). Restrições geram criatividade (Rosewater); o color pie mantém MTG coerente por 30 anos. SWG (NGE) e a crise de WoW ("um jogo para todos vira um jogo para ninguém") são os contra-exemplos de identidade perdida. **[FATO/FONTE]** https://gdcvault.com/play/1035120/The-Four-One-Page-Design ; https://www.gamedeveloper.com/design/ccp-online-s-three-design-pillars-for-sandbox-mmos ; https://magic.wizards.com/en/news/making-magic/twenty-years-twenty-lessons-part-3-2016-06-13 ; https://massivelyop.com/2015/08/22/the-game-archaeologist-star-wars-galaxies-nge/
→ **Decisão — pilares propostos do RagnaDofus (rascunho, 4):**
> 1. **Mundo contínuo e socialmente vivo** (herança Ragnarok): você sempre vê outros jogadores agindo; o mundo reage e persiste.
> 2. **Economia interdependente player-driven** (herança Dofus/Albion/EVE): quase tudo é feito e destruído por jogadores; ninguém é autossuficiente.
> 3. **Combate tático legível** (herança Dofus): profundidade por posicionamento e AP/MP, com transição telegrafada.
> 4. **Progressão em largura, aprendizado em primeiro lugar** (Koster/GW2): crescer em opções e maestria, não em números vazios.
>
> Toda feature nova deve servir a ≥1 pilar; manter uma lista viva de **non-goals** (ex.: "não teremos gear treadmill vertical", "não simularemos ecologia autônoma", "não faremos combate real-time twitch").

---

## 4. Riscos de design

- **R1 — Colapso/instabilidade do mundo vivo (armadilha UO).** Qualquer tentativa de ecologia autônoma tende a custo de compute explosivo e colapso de população. *Mitigação:* P3-4/P3-5 (LOD + diretor de eventos + jogadores como simulação; outputs visíveis).
- **R2 — Inflação ou colapso de preços.** Sink de moeda insuficiente → inflação; sink de itens insuficiente → saturação de craft e colapso de preços (visto em Albion quando a coleta é farta demais). *Mitigação:* P3-1/P3-2 (destruição/consumo + sinks percentuais + instrumentação).
- **R3 — "Water finds a crack": jogadores otimizam a diversão para fora.** Uma build/estratégia/rota econômica dominante será grindada até o tédio. *Mitigação:* P3-7 (sem estratégia dominante; decisões situacionais). https://www.designer-notes.com/?p=369
- **R4 — Grind sem intenção / treadmill.** Progressão que só empilha número e invalida esforço anterior mata retenção e acessibilidade (cautela WoW). *Mitigação:* P3-6/P3-8 (horizontal, recompensa muda o jogo, competência sentida).
- **R5 — Fricção de troca de modo.** Combate instanciado pode parecer um "tranco" se a transição for abrupta. *Mitigação:* P3-9 (aggro telegrafado + fase de posicionamento legível).
- **R6 — Scope creep / perda de identidade.** Adicionar sistemas sem filtro dilui o jogo (NGE/SWG, homogeneização de WoW). *Mitigação:* P3-11 (pilares + non-goals como firewall).
- **R7 — Bots corroendo a economia emergente.** Economia player-driven é tão saudável quanto a premissa de que a produção é humana. *Mitigação:* enforcement anti-automação ativo + autoridade de servidor (C2 §2.3). https://www.pcgamer.com/bots-are-threatening-eve-onlines-economy-and-players-are-fed-up/
- **R8 — Persistência vs presença compartilhada.** Mundo que muda por jogador (phasing) pode esconder amigos — conflita com o pilar "socialmente vivo". *Mitigação:* preferir mudanças de mundo compartilhadas/por servidor a phasing individual.
- **R9 — Simulação invisível (desperdício).** Compute gasto no que o jogador não percebe. *Mitigação:* P3-5 (todo sistema precisa de output legível).

---

## 5. Perguntas abertas

1. **Classe vs classless?** Spell Variants (Dofus, classes com largura intra-classe) vs "you are what you wear" (Albion, classless). Qual serve melhor nossos pilares de identidade e diversidade de build? (Relaciona P3-7.)
2. **Eixo da progressão por tempo.** Adotamos algum componente estilo EVE (progressão por tempo/presença) para profissões, como colchão anti-grind, aceitando o risco de "esforço não recompensado" para jogadores pesados? (P3-8.)
3. **Granularidade do diretor de eventos.** Que estado do servidor o diretor lê para disparar eventos dinâmicos, e como garantimos que eles deixem efeitos persistentes *legíveis* sem sobrecarga de marcadores (lição GW2)? (P3-4/P3-5.)
4. **Full-loot ou não.** Full-loot (Albion) é o sink de itens mais forte, mas colide com acessibilidade/herança Ragnarok (mais casual). Qual nível de perda/destruição de itens a economia precisa para não saturar? (P3-1.)
5. **Escala de mundo contínuo.** Ragnarok "seamless" muitas vezes é um mosaico de mapas carregados, não um mundo único. Qual é nosso alvo técnico de continuidade e como isso interage com instâncias de combate? (Relaciona com os briefs B3-movimento e D1-cliente-servidor.)
6. **Validação dos pilares.** Precisamos de um mecanismo de governança (ex.: polling de jogadores estilo OSRS, ou revisão interna contra pilares) para manter coerência ao crescer? (P3-11.)
7. **Regionalização econômica.** Quão agressiva deve ser a especialização regional de recursos/bônus (Albion) sem tornar o comércio uma barreira logística frustrante? (P3-3.)

---

*Fim do brief P3. Complementa C2 (economia/crafting), A1/A2 (engine/render), B3 (movimento), D1 (cliente-servidor).*
