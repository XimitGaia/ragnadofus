**VISÃO INICIAL\
DO PROJETO**

Game World • Compact Engine • AI Game Studio

*Consolidação das ideias discutidas até aqui*

**Natureza deste documento:** base de trabalho inicial. Ele organiza
referências, hipóteses e linhas de pesquisa; não representa um escopo
fechado nem decisões definitivas.

# 1. Intenção e forma de trabalho

A proposta é desenvolver um jogo com apoio intensivo de inteligência
artificial, mas sem reduzir o processo a pedir que uma IA gere código ou
conteúdo. A intenção é montar uma equipe --- humana e de agentes ---
capaz de pensar dinâmica, ideias, regras, sistemas e implementação.
Também existe a possibilidade de criar uma plataforma própria para
apoiar o desenvolvimento do jogo.

As ideias serão trazidas progressivamente. O papel deste documento é
acumular e organizar essas ideias, cruzá-las com pesquisa técnica e de
design, identificar relações e riscos e preservar espaço para propostas
ambiciosas, inclusive experimentais. Não se deve fechar prematuramente
engine, mecânicas ou escopo.

# 2. Referências de design

## 2.1 Ragnarok Online

-   Referência positiva para perspectiva/visibilidade, sensação de mundo
    vivo, atmosfera e direção de arte.

-   A combinação visual clássica é híbrida: personagens representados
    por sprites sobre cenários com geometria e elementos 3D.

-   A estrutura técnica observável nos formatos conhecidos separa
    componentes como mapas/terreno, caminhabilidade, objetos, sprites e
    animações.

-   A referência é principalmente a sensação de exploração, leitura
    visual e identidade artística --- não necessariamente copiar seus
    sistemas.

## 2.2 Dofus

-   Referência positiva para profundidade sistêmica: profissões, coleta,
    crafting, recursos, economia e interdependência entre atividades.

-   Ponto de fricção: navegação estruturada em células e sensação de
    mundo menos livre/contínuo do que o desejado.

-   Dofus 3.0 migrou o cliente para Unity; isso não significa, por si
    só, que sua representação visual tenha deixado de ser 2D/isométrica.

-   Uma hipótese a explorar é combinar exploração espacial mais livre
    com sistemas de combate ou interação que possam incorporar
    profundidade tática.

## 2.3 Direção artística

-   Interesse em pixel art, possivelmente com acabamento visual superior
    ou uma combinação inspirada na linguagem visual de Ragnarok.

-   A meta não é simplesmente reproduzir gráficos antigos: é investigar
    como obter personalidade, legibilidade e riqueza visual com
    representação compacta.

-   Ainda falta analisar fichas técnicas e decompor as referências em
    fatores visuais, de câmera, animação, navegação, combate e
    interação.

# 3. Conceito técnico: um universo compacto

Uma das ideias centrais é investigar um princípio semelhante ao conceito
chamado na conversa de "Tiny Universe": um jogo relativamente pequeno em
armazenamento que consiga representar um universo amplo e rico. A
referência exata chamada Tiny Universe ainda não foi identificada com
segurança; o princípio de engenharia, porém, pode ser estudado
independentemente.

## 3.1 Hipótese "50 MB Universe Challenge"

Tratar 50 MB como meta experimental de pesquisa, não como promessa de
que todo o conteúdo imaginado caberá nesse limite. O desafio é medir
quanto mundo e variedade podem ser representados por regras, parâmetros,
seeds e componentes compartilhados, em vez de armazenar cada instância
como um asset exclusivo.

-   Separar as métricas: tamanho do executável, tamanho dos assets,
    dados persistidos do mundo e memória de execução.

-   Investigar geração procedural determinística, paletas indexadas,
    tiles, atlases, texture arrays, deduplicação e reconstrução de
    assets.

-   Armazenar diferenças e mudanças do mundo, em vez de persistir cópias
    completas de regiões estáticas.

-   Usar streaming de chunks/regiões para limitar memória e permitir
    mapas extensos.

-   Versionar regras e seeds para que o mundo possa ser reconstruído de
    maneira consistente ao longo de atualizações.

## 3.2 Técnicas a investigar

-   Paletas indexadas: armazenar índices de cor em vez de RGBA. Exemplo:
    uma imagem 128×128 com 16 cores pode usar 4 bits por pixel,
    totalizando 8 KB antes de paleta e metadados; RGBA de 32 bits
    ocuparia 64 KB.

-   Tile-based world: separar terreno, vegetação, objetos, decoração,
    colisão, elevação, iluminação e efeitos.

-   Geração procedural determinística: biomas, distribuição de recursos,
    vegetação, fauna, estruturas e variações derivadas de seeds.

-   Deduplicação/content-addressable storage: armazenar uma única vez
    recursos idênticos ou compartilhados.

-   Animações compostas: reutilizar partes e movimentos entre corpos,
    cabeças, equipamentos, armas e efeitos.

-   Renderização eficiente: batching, instancing, atlases, texture
    arrays e shaders especializados.

-   Persistência diferencial: guardar apenas alterações relevantes
    feitas por jogadores ou simulação.

## 3.3 Exemplo de conteúdo para o experimento

-   5 biomas e cerca de 20 tipos de terreno.

-   30 tipos de criaturas e 10 NPCs/personagens-base.

-   Centenas de combinações de equipamentos a partir de componentes
    reutilizáveis.

-   Coleta, crafting e produção interdependente.

-   Ciclo de dia e noite, água, vegetação, uma pequena cidade e pontos
    de interesse.

Esses números são apenas uma proposta inicial para dar forma ao
experimento. Devem ser ajustados conforme os primeiros protótipos e
medições.

# 4. Mundo, câmera e gameplay: hipóteses iniciais

As opções abaixo são hipóteses de trabalho, não decisões confirmadas:

  -----------------------------------------------------------------------
  Tema                    Hipótese inicial        Observação
  ----------------------- ----------------------- -----------------------
  Perspectiva             Isométrica oblíqua      Inspirada na leitura
                                                  visual de
                                                  Ragnarok/Dofus.

  Arte                    Pixel art de alta       Pode combinar técnicas
                          qualidade               2D e 3D.

  Renderização            2.5D híbrida            Sprites, tiles,
                                                  geometria e objetos 3D.

  Personagens             Sprites animados        Investigar composição e
                                                  reutilização de partes.

  Cenários                Tiles, geometria e      Escolher representação
                          objetos 3D              por custo e
                                                  legibilidade.

  Movimentação            Livre e contínua        Diferenciar de
                                                  navegação estritamente
                                                  por células.

  Mundo                   Regiões conectadas +    Mundo amplo sem exigir
                          streaming               mapa infinito em
                                                  memória.

  Combate                 Tempo real com          Ainda em aberto.
                          elementos táticos       
                          possíveis               

  Economia                Crafting e produção     Aproveitar a
                          interdependente         profundidade sistêmica
                                                  de Dofus.

  Conteúdo                Procedural + assets     Equilibrar identidade
                          autorais                artística e
                                                  compactação.

  Desenvolvimento         AI-first com agentes    Agentes auxiliam
                          especializados          design, implementação e
                                                  QA.
  -----------------------------------------------------------------------

# 5. Arquitetura: três frentes relacionadas

## 5.1 Game World

A camada que define o universo e suas regras:

-   Biomas, regiões, geografia e pontos de interesse.

-   Criaturas, NPCs, personagens, atributos e progressão.

-   Coleta, recursos, profissões, crafting e cadeias produtivas.

-   Economia, comércio, oferta/demanda e circulação de itens.

-   Combate, navegação, interação, eventos e simulação.

-   Persistência, estados dinâmicos e evolução do mundo.

## 5.2 Compact Engine

A infraestrutura de execução e representação:

-   Renderização 2D/2.5D/3D, câmera, iluminação e shaders.

-   Tiles, sprites, animação, atlas, texturas e materiais.

-   Física, colisão, navegação e streaming de regiões.

-   Formato de dados compacto, compressão, deduplicação e reconstrução.

-   Ferramentas de inspeção, profiling, validação e benchmarks.

## 5.3 AI Game Studio

Uma possível plataforma para que designers, artistas, engenheiros e
agentes criem e alterem partes do universo com rastreabilidade. Por
exemplo, uma descrição de uma floresta poderia gerar uma proposta
estruturada de bioma, incluindo regras, assets, recursos, criaturas,
pontos de interesse e testes.

-   Agentes especializados podem propor sistemas, gerar conteúdo,
    implementar mudanças e revisar resultados.

-   O trabalho dos agentes deve produzir artefatos verificáveis:
    especificações, dados, código, assets, testes e versões jogáveis.

-   A engine e o jogo devem continuar utilizáveis sem depender da
    plataforma; a plataforma é uma frente independente.

# 6. Escolha de tecnologia: Rust e engines

Existe interesse em construir um motor gráfico próprio, possivelmente em
Rust, com foco em performance, reaproveitamento de pixels e
representação inteligente. A recomendação inicial é não começar
escrevendo uma engine inteira do zero. É mais seguro usar uma base
existente e isolar experimentos de renderer.

## Opções discutidas

-   Bevy: opção principal para investigar como base de
    runtime/ECS/renderização em Rust; permite evoluir módulos e
    experimentar pipelines.

-   Macroquad: alternativa simples para protótipos 2D/3D e experimentos
    rápidos de renderer.

-   wgpu: camada gráfica de mais baixo nível e multiplataforma, caso
    seja necessário controlar diretamente recursos e pipelines da GPU.

-   wgpu_tilemap: exemplo de abordagem de tilemap com renderização
    eficiente na GPU.

Direção provisória: começar com Bevy como infraestrutura e manter um
renderer próprio como módulo experimental, substituindo componentes
gradualmente quando houver evidência de que isso agrega valor. Macroquad
pode acelerar protótipos isolados; wgpu oferece controle quando a
pesquisa exigir.

# 7. Harness e fluxo de desenvolvimento com IA

A IA deve participar de um processo de engenharia controlado, não apenas
produzir código em conversas isoladas. Um harness pode coordenar agentes
especializados e validar cada mudança no jogo.

-   Orquestrador: divide objetivos, mantém contexto e coordena agentes.

-   Game design agent: transforma ideias em regras, especificações e
    casos de teste.

-   Engineering agent: implementa sistemas e ferramentas.

-   Art/content agents: propõem assets, variações, animações e conteúdo
    procedural.

-   QA/reviewer agents: verificam regras, regressões, consistência e
    qualidade.

-   Engine/game runtime: oferece ferramentas de inspeção e comandos
    controláveis por agentes.

-   Verificação automatizada: builds, testes, screenshots, logs,
    métricas de performance e sessões reproduzíveis.

A qualidade final depende de ciclos curtos de implementação e validação
dentro do jogo. Um agente afirmar que concluiu uma tarefa não substitui
evidência de que a build funciona e de que a experiência visual e
interativa está correta.

# 8. Pesquisa e referências técnicas já levantadas

As referências abaixo foram identificadas na pesquisa inicial. São
pontos de partida; documentação comunitária e repositórios devem ser
tratados como fontes técnicas, não como autorização para reutilizar
assets protegidos.

-   **Ragnarok Online --- descrição da apresentação visual híbrida ---**
    Steam: https://store.steampowered.com/app/215100/Ragnarok_Online/

-   **Formatos de recursos de Ragnarok (GRF, SPR, ACT, GND, GAT, RSW)
    ---** Documentação e ferramentas comunitárias; investigar cada
    formato antes de implementar importadores.

-   **ragx --- exportação/inspeção de recursos ---** GitHub:
    https://github.com/

-   **Dofus 3.0 --- migração do cliente para Unity ---** Ankama:
    https://www.dofus.com/

-   **Bevy Engine ---** https://bevyengine.org/

-   **Macroquad ---** https://macroquad.rs/

-   **wgpu ---** https://wgpu.rs/

-   **Procedural generation / kkrieger ---** Referência histórica para
    explorar geração procedural e representação compacta.

-   **Revisões acadêmicas sobre geração de assets para jogos ---**
    Pesquisar revisões sistemáticas sobre procedural generation,
    graphical game assets e IA generativa.

# 9. Riscos e princípios de decisão

-   Não confundir tamanho do executável com tamanho total do jogo:
    assets, dados do mundo, memória e conteúdo baixado precisam ser
    medidos separadamente.

-   Procedural não significa automaticamente barato, bonito ou variado;
    regras ruins podem produzir repetição e mundos sem intenção
    artística.

-   Um renderer próprio pode virar um projeto longo e desviar energia do
    jogo. Manter protótipos comparáveis e critérios objetivos.

-   A referência estética deve ser estudada sem copiar assets, código ou
    propriedade intelectual de outros jogos.

-   Mundo aberto não exige mundo infinito nem ausência de carregamento:
    streaming de chunks pode sustentar uma experiência espacialmente
    contínua.

-   Separar exploração, combate, economia e simulação em sistemas
    testáveis ajuda a preservar liberdade para mudar o design.

-   Distinguir sempre hipótese, experimento, decisão e requisito
    confirmado.

# 10. Próximos passos sugeridos

1.  Continuar registrando ideias sem exigir que todas sejam decididas
    agora.

2.  Montar fichas comparativas de Ragnarok e Dofus: câmera, escala,
    navegação, combate, animação, mundo, economia, crafting e
    arquitetura visual.

3.  Definir um pequeno protótipo de câmera + terreno + personagem para
    testar leitura visual e movimentação livre.

4.  Criar um benchmark de representação compacta: mesmo cenário
    implementado com sprites/tiles tradicionais e com técnicas de
    paleta/procedural.

5.  Medir Bevy, Macroquad e uma abordagem direta com wgpu em um teste
    controlado.

6.  Desenhar um primeiro formato declarativo de bioma e mundo, com seed
    determinística e dados versionados.

7.  Prototipar um harness mínimo: agente propõe mudança, gera artefatos,
    executa build/testes e apresenta evidências.

# 11. Registro de decisões

Até este ponto, nenhuma das hipóteses de câmera, combate, engine,
tamanho final ou arquitetura deve ser tratada como decisão fechada.

  -----------------------------------------------------------------------
  Categoria               Estado                  Como tratar
  ----------------------- ----------------------- -----------------------
  Referências             Identificadas           Ragnarok Online e Dofus
                                                  como fontes de
                                                  inspiração e análise.

  Pixel art / 2.5D        Hipótese                Investigar alternativas
                                                  visuais e custo de
                                                  produção.

  Mundo livre e contínuo  Preferência de design   Definir navegação e
                          em exploração           limites por protótipos.

  Rust / Bevy             Direção técnica         Validar com benchmark;
                          provisória              não assumir engine
                                                  definitiva.

  50 MB Universe          Meta experimental       Medir e revisar; não
                                                  prometer capacidade
                                                  antecipadamente.

  AI Game Studio          Possibilidade           Explorar separadamente
                          estratégica             do jogo e da engine.
  -----------------------------------------------------------------------

**FIM DA VERSÃO 0.1**
