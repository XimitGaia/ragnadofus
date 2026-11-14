# P6 — Representação Compacta e Geração Procedural (o "50 MB Universe Challenge")

> Pesquisa web multi-fonte realizada em **2026-11-14**. Cada afirmação é marcada como **[FATO]** (com fonte/URL) ou **[OPINIÃO]** (síntese/inferência minha a partir das fontes). Prioridade **P1 (pós-demo)**: as decisões de dados tomadas **agora** não podem inviabilizar esta linha depois.

---

## Contexto

O desafio é representar um **mundo grande** (o "universo" de `ragnadofus`) com **pouquíssimos dados** — a meta simbólica "50 MB Universe": um pacote cliente+regras+mundo compacto, denso em conteúdo por byte, sem cair em repetição sem intenção nem em assets gigantes. A pesquisa separa três camadas que NUNCA devem ser confundidas numa única métrica: **executável/motor**, **assets** (arte/áudio/dados estáticos) e **mundo** (estado persistente + o que é regenerável por seed). A tese central da literatura e da demoscene: **não armazene o que você pode reconstruir deterministicamente**; armazene só a *receita* (seed + regras versionadas) e o *delta* autoral.

Esta linha conversa diretamente com decisões já tomadas no projeto: pixel-art de alta densidade e palette shaders (A2), ECS compartilhado cliente↔servidor e `WorldRepository` como trait trocável (A1/D1). A representação compacta é uma **propriedade de arquitetura de dados**, não um otimizador que se liga no fim.

---

## Fontes de alta qualidade (URLs)

**Literatura PCG (acadêmica, acesso aberto)**
- [FATO] *Procedural Content Generation in Games* — Shaker, Togelius & Nelson (Springer, 2016). **Todos os capítulos em PDF aberto** no site oficial. — http://pcgbook.com/ · https://link.springer.com/book/10.1007/978-3-319-42716-4
  - Cap. 2 Search-based (evolutivo), Cap. 3 Constructive (dungeons/níveis), **Cap. 4 Fractals, noise & agents** (terreno/textura procedural), **Cap. 5 Grammars & L-systems**, Cap. 6 Rules & mechanics, Cap. 8 **ASP (Answer Set Programming / constraint-based)**, Cap. 10 Experience-driven, Cap. 11 Mixed-initiative, **Cap. 12 Evaluating generators** (como medir qualidade). PDFs: `chapter01.pdf`…`chapter12.pdf` em pcgbook.com.
- [FATO] *Deep Learning for Procedural Content Generation* (survey, PCG-via-ML). — https://arxiv.org/pdf/2010.04548
- [FATO] *Practical PCG Through Large Language Models*. — https://arxiv.org/pdf/2305.18243

**Wave Function Collapse (WFC)**
- [FATO] Repositório original `mxgmn/WaveFunctionCollapse` (Maxim Gumin, 2016). — https://github.com/mxgmn/WaveFunctionCollapse
- [FATO] Explicação didática do algoritmo. — https://robertheaton.com/2018/12/17/wavefunction-collapse-algorithm/
- [FATO] *The Open Source Success Story of Wave Function Collapse* (ACM SIGMM Records). — https://records.sigmm.org/?open-source-item=procedural-content-generation-the-open-source-success-story-of-wave-function-collapse
- [FATO] *A Markovian Framing of WFC* (2026) e *Punch Out Model Synthesis* (variante determinística/escalável). — https://arxiv.org/html/2509.09919 · https://arxiv.org/pdf/2501.14786

**Demoscene / size coding**
- [FATO] *How can demoscene productions be so small?* — Ctrl-Alt-Test (a melhor fonte sobre tradeoffs). — https://www.ctrl-alt-test.fr/2017/how-can-demoscene-productions-be-so-small/
- [FATO] `.kkrieger` (FPS jogável em **97.280 bytes**, Farbrausch, 2004). — https://en.wikipedia.org/wiki/.kkrieger · código-fonte: https://github.com/jaromil/kkrieger-werkkzeug3
- [FATO] *How a 64k intro is made* (lofibucket) e kkrunchy/Crinkler (compressores de executável da demoscene). — https://www.lofibucket.com/articles/64k_intro.html

**Content-addressable storage / dedup**
- [FATO] Git é um filesystem endereçado por conteúdo (blobs nomeados pelo hash do conteúdo). — https://devopsbeast.com/courses/git-internals/how-git-works/git-is-content-addressable
- [FATO] *Content Addressable Storage: CAS, Deduplication Explained*. — https://stonefly.com/blog/content-addressable-storage-enterprise-guide/

**Paleta indexada / compressão de textura**
- [FATO] *Indexed color* (índice por pixel → economia de bytes; 4-bit empacota 2 px/byte). — https://en.wikipedia.org/wiki/Indexed_color
- [FATO] *Techniques for GPU-based Color Palette Mapping* (ILUT no shader). — https://www.researchgate.net/publication/336310827_Techniques_for_GPU-based_Color_Palette_Mapping
- [FATO] VRAM/texturas/paletas no PS1 (CLUT) — exemplo canônico de indexação. — https://polygonengine.com/blog/ps1-vram-texture-pages-and-cluts
- [FATO] *Understanding BCn Texture Compression Formats*. — https://www.reedbeta.com/blog/understanding-bcn-texture-compression-formats/

**Seed determinístico / persistência diferencial**
- [FATO] No Man's Sky: **seed de 64 bits → ~1,8×10¹⁹ variações**; "computado sob demanda a partir da seed, nada é salvo entre visitas". — https://en.wikipedia.org/wiki/No_Man's_Sky
- [FATO] Reprodutibilidade por seed exige PRNG semeado (não `rand` do sistema), **ordem determinística** de consumo; versão de engine/float/regras quebra a reprodução se não for versionada. — https://relishgames.com/journal/procedural-content-generation-in-2d-games/ · https://gamedev.net/blogs/entry/2259964-generating-seeds/

**Paper-doll / animação composível**
- [FATO] Sistema paper-doll: camadas separadas (corpo/cabelo/roupa/arma) com layouts idênticos, sincronizadas ao relógio de animação do corpo para não "driftar"; ordem de desenho (frente/trás) muda por pose. — https://github.com/shanjiechen/PartRig · https://seliel-the-shaper.itch.io/character-base

---

## Princípios que adotamos

### 1. Representação compacta (o que guardar vs. o que reconstruir)

- **[OPINIÃO] Regra de ouro:** só é dado o que é **autoral/imprevisível**. Tudo que é **derivável** (terreno base, distribuição de recursos, variações de tile, texturas processadas) vira **receita** (seed + grafo de operações versionado), não bytes. Essa é a lição estrutural comum a kkrieger e No Man's Sky. [FATO: textura em `.kkrieger` é guardada como *histórico de criação*, não per-pixel — https://en.wikipedia.org/wiki/.kkrieger]
- **[FATO→adoção] Paleta indexada.** Pixel = índice (1 byte, ou 2 px/byte em 4-bit) + uma LUT pequena. Casa com os palette shaders já previstos (A2): a arte guarda *índices*; a cor vem de paletas trocáveis no shader (ILUT). Benefício duplo: menos bytes **e** recoloração/variação procedural barata (palette-swap). — https://en.wikipedia.org/wiki/Indexed_color · https://www.researchgate.net/publication/336310827_Techniques_for_GPU-based_Color_Palette_Mapping
- **[FATO→adoção] Tiles reutilizáveis + atlas.** Mundo como índices em tilesets, não bitmaps completos; é o pré-requisito para WFC e para dedup de tiles. (Modelo CLUT/atlas: PS1, Bevy texture atlas de A1.)
- **[FATO→adoção] Content-addressable storage (CAS) para dedup de assets.** Nomear cada asset (sprite, tile, mesh, som) pelo **hash do seu conteúdo**. Conteúdo idêntico → mesmo hash → armazenado uma vez; dedup é consequência do endereçamento, não feature extra. Serve também como **verificação de integridade** (re-hash) e **versionamento** natural. — https://stonefly.com/blog/content-addressable-storage-enterprise-guide/ · https://devopsbeast.com/courses/git-internals/how-git-works/git-is-content-addressable
- **[FATO→adoção] Animação composível (paper-doll).** Equipamento/cabelo/efeitos como camadas sincronizadas ao corpo, não como spritesheets inteiros pré-combinados. N partes + M variações = N×M combinações a partir de N+M assets (explosão combinatória *gerada*, não *armazenada*). — https://github.com/shanjiechen/PartRig

### 2. PCG determinística reprodutível

- **[FATO→adoção] PRNG semeado, consumo em ordem determinística.** Nada de relógio/`thread_rng`/ordem de HashMap no caminho de geração. Em Rust: PRNG explícito e estável (ex. família `rand`/`StdRng` com algoritmo fixado, ou um xoshiro/pcg versionado) — a *escolha do algoritmo do PRNG* é parte do contrato e **não pode mudar silenciosamente**. — https://relishgames.com/journal/procedural-content-generation-in-2d-games/
- **[FATO→adoção] Versionamento de regras.** Toda geração carrega `(seed, rules_version, engine_version)`. Mudou a regra → nova versão; o mundo antigo continua reproduzível pela versão antiga, ou migra via delta. Sem isso, "determinístico" é ilusão a longo prazo. — https://gamedev.net/blogs/entry/2259964-generating-seeds/
- **[FATO→adoção] WFC para layout local coerente** (dungeons, interiores, biomas tileados). WFC é um **constraint solver** (observação da célula de menor entropia → colapso → propagação de adjacência via AC-4). Dois modelos: *overlapping* (aprende padrões NxN de um exemplo) e *simple tiled* (adjacências explícitas por tile). — https://github.com/mxgmn/WaveFunctionCollapse · https://robertheaton.com/2018/12/17/wavefunction-collapse-algorithm/
  - **[FATO] Limites do WFC:** é NP-hard, pode gerar **contradição** (volta vazio) e não garante propriedades **globais** (conectividade, "existe caminho do A ao B") sem restrições extras; tilesets "fáceis" dão resultado sem coerência global. — https://github.com/mxgmn/WaveFunctionCollapse
- **[FATO→adoção] Ruído + fractais** (Perlin/simplex/fBm) para terreno/altura/distribuição contínua; **grammars/L-systems** para estruturas hierárquicas (vegetação, estradas, assentamentos). — pcgbook cap. 4 e 5: http://pcgbook.com/
- **[OPINIÃO] Equilíbrio procedural × autoral.** O procedural dá *volume*; o autoral dá *intenção*. Padrão recomendado: **constructive + autoral como restrições sobre o gerador** (pontos fixos, "landmarks" feitos à mão que o WFC/ruído respeita), em vez de 100% aleatório. Para não cair em repetição sem intenção, a demo deve ter alguns locais **assinados à mão** que ancoram a percepção de mundo. Mixed-initiative (pcgbook cap. 11) é o enquadramento: o humano guia, o gerador preenche. — http://pcgbook.com/

### 3. Persistência diferencial (mundo > dados)

- **[FATO→adoção] Gerar o estático pela seed; guardar só o delta.** O estado persistido = `baseline(seed, rules_version)` + **conjunto de mudanças** que divergem do gerado (recurso coletado, prédio construído, item dropado, tile alterado). Reconstrução = regenerar o baseline e aplicar o delta. É o modelo No Man's Sky ("nada é salvo entre visitas" exceto o que o jogador mudou). — https://en.wikipedia.org/wiki/No_Man's_Sky
- **[OPINIÃO] Casamento com o `WorldRepository` (D1/A1):** o trait de persistência deve expor o mundo como *baseline regenerável + log de deltas*, não como um dump monolítico. Isso mantém o backend trocável (DT) e o save pequeno (cresce com a *ação* do jogador, não com o tamanho do mundo). O delta provavelmente quer ser **append-only / event-sourced** para auditabilidade e replay.

### 4. Lição da demoscene — o que é aplicável, o que é truque

- **[FATO] `.kkrieger`: 97.280 bytes no disco; caberia em "200–300 MB" no modo convencional.** Textura/mesh/áudio todos procedurais via `.werkkzeug3` (meshes = sólidos básicos deformados; áudio = synth V2 a partir de MIDI). — https://en.wikipedia.org/wiki/.kkrieger
- **Aplicável (estrutural)** — adotar:
  - Guardar **grafo de operações / histórico de criação** em vez de bytes finais (texturas, meshes). — kkrieger
  - **Síntese procedural de áudio** (synth node-graph, estilo 64klang/V2) em vez de WAV/OGG grandes. — https://en.wikipedia.org/wiki/.kkrieger
  - **Compressão de executável** (kkrunchy opera sobre binários standalone — vale para Rust). Ganho real no *tamanho do executável*. — https://www.ctrl-alt-test.fr/2017/how-can-demoscene-productions-be-so-small/
  - Converter dados de config para forma compacta embutida; evitar libs de 50 MB (já é a filosofia "motor compacto" do projeto). — Ctrl-Alt-Test
- **NÃO replicável (truque de contexto)** — [FATO] a própria Ctrl-Alt-Test diz que os tradeoffs extremos só fazem sentido sob a restrição artística de tamanho fixo, **não** para jogos persistentes que exigem flexibilidade, biblioteca rica de conteúdo e iteração rápida. — https://www.ctrl-alt-test.fr/2017/how-can-demoscene-productions-be-so-small/
  - **Tempo de carga:** kkrieger gera **todos os assets no load** → tempo de carga longo. Para um MMORPG persistente isso é inaceitável como está (precisa geração incremental/cache). — https://en.wikipedia.org/wiki/.kkrieger
  - **Blowup de RAM:** 97 KB em disco → centenas de MB em memória após geração. **A métrica de disco mente sobre a métrica de memória** (ver "Como medimos"). — https://en.wikipedia.org/wiki/.kkrieger
  - **Experiência linear/one-shot** e **pouca variedade de conteúdo**: a demoscene troca conteúdo por bytes; um RPG de mundo contínuo faz o trade **oposto** em muitos eixos.
  - **Reaproveitar código de OS/driver/DirectX** para não contar no budget: truque de *medição*, não de densidade real.

### 5. PCG-via-ML (horizonte, não agora)

- **[OPINIÃO/P2+]** Modelos (incl. LLMs) podem gerar/afinar conteúdo offline (ex.: propor tiles, nomes, layouts), mas a saída deve ser **destilada em seed+regras determinísticas** versionadas antes de entrar no mundo — senão perde reprodutibilidade e vira dado grande de novo. ML como *ferramenta de autoria*, não como gerador em runtime do estado canônico. — https://arxiv.org/pdf/2010.04548 · https://arxiv.org/pdf/2305.18243

---

## O que NÃO inviabilizar agora (restrições sobre as decisões de dados atuais)

> Nada aqui precisa ser **construído** para a demo; tudo aqui precisa **não ser bloqueado**.

1. **Toda geração aleatória DEVE passar por um PRNG semeado explícito e determinístico.** Proibir no caminho de geração: relógio, `rand::random()` global, iteração de `HashMap` (ordem não determinística), floats dependentes de plataforma em caminho crítico. Custo agora: ~zero; custo de consertar depois: reescrever geração. — https://relishgames.com/journal/procedural-content-generation-in-2d-games/
2. **Carregar `(seed, rules_version)` junto de qualquer coisa gerada desde já**, mesmo que a demo só use um valor fixo. Dá o gancho para persistência diferencial e reprodutibilidade sem refatorar o save depois.
3. **Mundo como índices (tiles/paleta), não como bitmaps/estado denso.** Mantém o caminho para WFC, dedup e palette shader aberto. Já alinhado com A2 (palette shaders) — só não "achatar" a arte em RGBA final cedo demais.
4. **IDs de asset por hash de conteúdo (CAS) desde o pipeline.** Mesmo que a demo tenha poucos assets, nomear por conteúdo evita um rebatismo global depois e dá dedup de graça. — https://stonefly.com/blog/content-addressable-storage-enterprise-guide/
5. **`WorldRepository` deve conseguir expressar `baseline + delta`**, não só `get/put` de estado monolítico. É uma restrição de *forma* do trait, não de implementação (que pode ser trivial na demo). — (A1/D1)
6. **Sprites em camadas (paper-doll) desde o layout da arte.** Não pré-combinar corpo+equipamento em spritesheets fixas; manter camadas alinhadas e sincronizáveis. Reverter depois é recortar arte à mão. — https://github.com/shanjiechen/PartRig
7. **Separar budgets desde já** (ver abaixo): não deixar um atalho de "empacota tudo junto" esconder qual camada está crescendo. Atalho consciente → `docs/debito-tecnico.md`.

---

## Como medimos (densidade de conteúdo por byte — métricas separadas)

A medida honesta de "50 MB Universe" **nunca** é um número só. Separar sempre quatro eixos (lição direta do blowup de RAM do kkrieger):

| Métrica | O que mede | Unidade | Armadilha |
|---|---|---|---|
| **Executável/motor** | binário do cliente+motor (pós-kkrunchy) | MB em disco | não misturar com assets; compressão de exe só conta aqui |
| **Assets** | arte/áudio/dados **após dedup (CAS)** e paleta indexada | MB em disco | medir *após* dedup, senão duplicatas inflam |
| **Mundo (estático)** | seed + regras versionadas que **geram** o baseline | **bytes** (seed) + KB (regras) | este é o número "mágico" — mundo gigante, dado minúsculo |
| **Mundo (dinâmico)** | delta persistido (ações do jogador) | MB, **cresce com o jogo** | separar de tudo acima; é o único que escala com uso |
| **Memória (runtime)** | pico de RAM após geração/descompressão | MB/GB | **a métrica que o disco esconde** — medir sempre |

- **[OPINIÃO] "Densidade" útil = conteúdo percebido ÷ (executável + assets + mundo-estático).** O mundo dinâmico fica *fora* do numerador de densidade porque é consequência do jogo, não da representação.
- **[FATO→adoção] Qualidade da geração também é métrica**, não só tamanho: usar os critérios do pcgbook cap. 12 (expressividade do gerador, controlabilidade, ausência de repetição óbvia, taxa de contradição do WFC). Densidade sem qualidade = ruído compactado. — http://pcgbook.com/ (chapter12.pdf)
- **[OPINIÃO] Instrumentar cedo:** um comando de build que reporta os cinco números acima por release, para que a densidade seja um gate observável e não uma afirmação.

---

### Resumo de uma linha

Guarde a **receita** (seed + regras versionadas + grafo de operações) e o **delta autoral**, não os bytes finais; indexe assets por **conteúdo (CAS)** e por **paleta**; componha animação em camadas; e **meça quatro budgets separados** — executável, assets, mundo-estático e mundo-dinâmico — com a RAM de runtime sempre à vista, porque é ela que o tamanho em disco esconde.
