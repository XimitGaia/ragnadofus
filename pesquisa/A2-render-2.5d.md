# A2 — Pipeline de Render 2.5D (pixel-art isométrico oblíquo em Rust/Bevy)

> Brief de fundamentação para a RFC do pipeline de render do MVP (zona feita à mão: tiles + sprites, câmera isométrica oblíqua, estilo Ragnarok Online).
> Data: 2026-11-14. Legenda: **[FATO]** = afirmação com fonte/URL verificável; **[HIPÓTESE]** = inferência/design não comprovado por fonte direta.

---

## 1. Problema

Precisamos desenhar uma zona 2.5D pixel-art com:

- **Terreno em tiles** com múltiplas camadas (terreno base, vegetação, água, objetos, sombras), eficiente na GPU.
- **Sprites 2D** (personagens, props) posicionados *sobre* o terreno com **elevação**, exigindo ordenação de profundidade (y-sort/depth) correta e oclusão plausível — o modelo clássico de RO (terreno com altura + sprites "billboard" voltados à câmera).
- **Câmera isométrica oblíqua** com pixel-art **nítida**: sem blur, sem shimmer ao mover, escala inteira.
- Opcionalmente **palette swapping** na GPU (recolor de equipamentos/variantes) sem duplicar assets.

As decisões de maior risco: (a) como representar/renderizar o tilemap multicamada; (b) qual critério de depth sort resolve oclusão com elevação sem artefatos; (c) como manter nitidez pixel-perfect numa câmera que pode ter zoom/pan; (d) se palette shader compensa o custo.

---

## 2. Achados (com URLs)

### 2.1 Tilemap como textura de índices na GPU

- **[FATO]** O crate **`wgpu-tilemap` v0.1.1** (set/2023) desenha **cada tilemap como um único quad**, então a contagem de vértices independe do tamanho do mapa. O fragment shader faz essencialmente **duas leituras de textura**: uma na *textura de tilemap* (que guarda índices) e outra na *texture array do tileset* (imagem real de cada tile). Tipos de API: `TilemapPipeline`, `TilemapRef` (índices), `TilesetRef` (atlas em array), `TilemapDrawData`, `TilemapNoise`. **Descarta fragments totalmente transparentes**, então múltiplas camadas podem ser aceleradas com depth buffer. Depende de `wgpu ^0.17`. — https://docs.rs/wgpu-tilemap/latest/wgpu_tilemap/ e https://github.com/aweinstock314/wgpu-tilemap
- **[FATO]** O artigo de paavohtl descreve a técnica em profundidade: mundo armazenado como **array plano de índices** enviado à GPU como **texture array** (`RG16UI`, dois índices 16-bit por célula — floor/wall), layout de memória idêntico CPU↔GPU (transferência sem conversão). O fragment shader reconstrói a aparência amostrando os índices e indexando a **array texture** (preferida a atlas por suportar mipmapping correto). Resultado: **1 draw call**, independente do tamanho do mundo; ~200–300 µs/frame para 512³ voxels a 3440×1440 numa RTX 2080 Super (~3000–5000 FPS). — https://blog.paavo.me/gpu-tilemap-rendering/
- **[FATO]** Técnica canônica documentada em **GPU Gems 2, cap. 12 "Tile-Based Texture Mapping"** (tilemap de índices + tiles reais via indireção na GPU). — https://developer.nvidia.com/gpugems/gpugems2/part-ii-shading-lighting-and-shadows/chapter-12-tile-based-texture-mapping

### 2.2 Crate Bevy para tilemap eficiente

- **[FATO]** **`bevy_ecs_tilemap` v0.20** (alinhado ao Bevy 0.20) renderiza por **chunking**: tiles divididos em chunks, cada um com sua própria mesh enviada à GPU; **meshing feito no vertex shader** com vértices calculados em múltiplas threads; **frustum/chunk culling** (tiles fora da tela não renderizam); **animações via GPU**. Suporta **orthogonal, isométrico (diamond e staggered) e hexagonal**. Suporta explicitamente **camadas e mapas esparsos** (exemplos `layers`, `chunking`, `frustum_cull_test`). Modelo ECS: **cada tile é uma entidade** (conveniente, mas pesado em memória). — https://github.com/StarArawn/bevy_ecs_tilemap e https://docs.rs/bevy_ecs_tilemap
- **[FATO]** O próprio Bevy tem exemplo oficial **"Tilemap Chunk"** (2D rendering), indicando suporte nativo crescente a chunks de tilemap. — https://bevy.org/examples-webgpu/2d-rendering/tilemap-chunk/

### 2.3 Depth sorting / y-sort isométrico com elevação

- **[FATO]** Fórmula padrão de depth em iso: `depth = cartX + cartY`; **com elevação**, `depth = cartX + cartY + z`. Desenho **back-to-front** (x+y crescente) e **bottom-to-top** (z crescente). Y-sort = ordenar por posição Y da tela. — https://gamedevelopment.tutsplus.com/tutorials/isometric-depth-sorting-for-moving-platforms--cms-30226 e https://www.evozon.com/two-unity-tricks-isometric-games/
- **[FATO]** A fórmula simples `x+y+z` **falha** em casos de oclusão complexos. Shaun LeBron demonstra que o correto é **ordenação topológica**: construir grafo dirigido (aresta = "A está atrás de B"), DFS para a ordem. Detecção de overlap achatando caixas via `isoX=x+z`, `isoY=y+z`. **Ciclos de oclusão** (A atrás de B, B atrás de C, C atrás de A) quebram o topo-sort — detectáveis por Tarjan (SCC) — e exigem **segmentar sprites** ou **clipping/depth mask**. — https://shaunlebron.github.io/IsometricBlocks/
- **[FATO]** Em Bevy, o idioma usual é **y-sort via `transform.translation.z = f(y)`** (ex.: z proporcional ao -y). Crate **`extol_sprite_layer`** oferece camadas explícitas + y-sort automático (z = z da camada + offset em [0,1) pela posição y-sorted). **Armadilha conhecida**: mudar `translation.z` no `Update` nem sempre atualiza a ordem — precisa do code path certo (ver issue bevy #26362); escrever z no `PostUpdate` é o padrão recomendado. — https://docs.rs/extol_sprite_layer , https://github.com/bevyengine/bevy/issues/1275 , https://github.com/bevyengine/bevy/issues/26362
- **[FATO]** RO: personagens vivem num ambiente 3D (terreno GND com altura/lightmaps, colisão GAT por célula com altitude, cena RSW), mas são **sprites 2D** com facings (frente/costas/lados/diagonais) — sprites voltados à câmera sobre terreno 3D. Formatos: GAT (altitude+walkability), GND (ground, lightmaps, água), RSW (modelos, luzes, som), SPR/ACT (sprites). — https://github.com/oldinroplayer/ro-map-viewer e https://en.wikipedia.org/wiki/Ragnarok_Online

### 2.4 Câmera isométrica oblíqua com pixel-art nítida

- **[FATO]** Pixel-art borra porque o filtro default (bilinear) mistura vizinhos. Correção: **nearest-neighbor** + **escala inteira** (2x/3x/4x…). Câmera em coordenada fracionária (ex. x=100.37) causa **shimmer/jiggle**; mitigar **arredondando a posição da câmera para inteiro todo frame** antes de renderizar. Rotação em incrementos não-múltiplos de 90° é problemática em pixel-art. — https://www.slowrush.dev/news/pixel-perfect-rendering/ , https://leahayes.co.uk/2015/03/02/pixel-perfection-in-2d-game-projects.html , https://spritesheetgenerator.online/blog/pixel-perfect-scaling-nearest-neighbor
- **[FATO]** Abordagem robusta: **render para buffer de baixa resolução** (ex. 320×180) off-screen, depois **upscale por fator inteiro** para preencher a janela. — https://www.slowrush.dev/news/pixel-perfect-rendering/
- **[FATO]** Crates Bevy prontos: **`bevy_pixel_camera`** (configura `Camera2d` com `ScalingMode::WindowSize`, escala inteira, ajusta zoom/viewport ao target res; componentes `PixelZoom`, `PixelViewport`); **`bevy_pixcam`** (fork mantido/atomicptr, mesma ideia); **`bevy_pixel_perfect`** (efeito pós-processamento, estilo Astortion). — https://docs.rs/bevy_pixel_camera , https://github.com/atomicptr/bevy_pixcam , https://github.com/HazelTheWitch/bevy_pixel_perfect

### 2.5 Palette shaders / indexed-color na GPU

- **[FATO]** Abordagem **correta/barata (indexed)**: o sprite guarda **índices** (não RGB); o shader usa o índice como coordenada numa **textura de paleta** (LUT) — 1 lookup extra por fragment. Trocar de paleta = trocar a LUT ligada; interpolar índice de paleta (ex. 0.5) mistura paletas. — https://godotshaders.com/shader/palette-swap-no-recolor-recolor/ e https://forum.gamemaker.io/index.php?threads/how-to-swap-the-color-palette-using-shader-and-texture.8453/
- **[FATO]** Abordagem **por distância de cor** (sprite em RGB, loop comparando com paleta de entrada via `distance()`): funciona sem pré-processar os assets, mas faz **múltiplos lookups por fragment** e **degrada com mais cores/precisão** — "troca custo por conveniência". — https://godotshaders.com/shader/sprite3d-indexed-color-palette-swapping-shader/
- **[FATO]** Survey acadêmico de técnicas de mapeamento de paleta na GPU (ILUT single-pass, lookup texture, indexação direta por canal). — https://www.researchgate.net/publication/336310827_Techniques_for_GPU-based_Color_Palette_Mapping
- **[FATO]** Exemplo prático de produção (Rhythm Quest devlog 15) usa palette shaders com texturas indexadas e LUT de paleta. — https://ddrkirbyisq.medium.com/rhythm-quest-devlog-15-palette-shaders-173d697ae053 *(nota: página retornou 403 na coleta automática; citada pelo título/ranking da busca, não verificada integralmente)*

---

## 3. Técnicas / Alternativas

### 3.1 Tilemap

| Alternativa | Como funciona | Prós | Contras | Veredito MVP |
|---|---|---|---|---|
| `bevy_ecs_tilemap` 0.20 | Chunks com mesh por chunk, meshing no vertex shader, culling, GPU-anim; camadas nativas; iso/hex/orto | Integra com ECS/Bevy; camadas e iso prontos; mantido | Entidade por tile (memória); acoplado à versão do Bevy | **Recomendado** p/ MVP |
| Tilemap-de-índices custom (estilo `wgpu-tilemap`/GPU Gems 2) | 1 quad/camada, textura de índices + texture array; shader reconstrói | Menor overhead; 1 draw call/camada; controle total | Precisa escrever shader/pipeline `wgpu` à mão; integração manual no Bevy | Fallback/otimização futura |
| Mesh por tile (naïve) | Um quad por tile | Trivial | Muitos vértices/draws; não escala | Rejeitado |

### 3.2 Depth sort

| Alternativa | Critério | Prós | Contras | Veredito |
|---|---|---|---|---|
| Y-sort por `z=f(y)` (+ z de camada) | z por posição (y da base + elevação) | Simples, O(n log n), idioma Bevy | Casos de oclusão complexos podem errar | **Base recomendada** |
| Depth `x+y+z` iso | fórmula direta | Barato | Falha em overlaps/ciclos | Insuficiente sozinho |
| Topological sort (grafo + Tarjan) | ordena por relação de oclusão | Correto p/ casos difíceis | Caro, complexo; ciclos exigem split/clip | Só se artefatos aparecerem |
| Depth buffer na GPU | z-test por fragment | Hardware resolve oclusão; combina com tilemap que descarta transparentes | Sprites 2D precisam z coerente; blending/transparência exige cuidado | **Complemento forte** |

### 3.3 Câmera pixel-perfect

| Alternativa | Como | Prós | Contras | Veredito |
|---|---|---|---|---|
| Render-to-texture low-res + upscale inteiro | render 320×180-ish → upscale NxN | Nitidez máxima, controle total | Zoom só em passos inteiros; menos flexível | **Recomendado** |
| `bevy_pixel_camera`/`bevy_pixcam` | escala inteira via ScalingMode + viewport | Pronto, pouco código | Preso à política do crate; versão Bevy | Recomendado p/ começar |
| Câmera "livre" com nearest + snap | arredonda posição/frame | Simples | Zoom/rotação não-inteiros reintroduzem blur/shimmer | Aceitável c/ snapping |

### 3.4 Palette

| Alternativa | Como | Prós | Contras | Veredito |
|---|---|---|---|---|
| Indexed + LUT de paleta | sprite guarda índices; 1 lookup na LUT | Barato (1 texel extra); recolor instantâneo; variantes sem duplicar assets | Exige assets indexados (pipeline de arte) | **Recomendado se** recolor for requisito |
| Distância de cor em runtime | loop comparando RGB | Não precisa indexar assets | Vários lookups/fragment; escala mal | Evitar |
| Sem palette (texturas duplicadas) | 1 textura por variante | Zero custo de shader | Explosão de assets/VRAM | Default até precisar de recolor |

---

## 4. Recomendação

Para o MVP, recomendo:

1. **Tilemap: `bevy_ecs_tilemap` (0.20)** com uma camada por tipo (terreno, vegetação, água, objetos, sombras). Ele já entrega chunking, culling, GPU-animation e suporte isométrico/camadas, minimizando código de pipeline. Manter como *fallback* planejado a técnica de **tilemap-de-índices custom em `wgpu`** (GPU Gems 2 / `wgpu-tilemap`) caso o custo de "entidade por tile" ou os draws de muitas camadas virem gargalo.
2. **Depth sort: y-sort por `z = f(base_y + elevação)` + z-base por camada**, escrito no `PostUpdate`, **combinado com depth buffer** da GPU (aproveitando o descarte de fragments transparentes). Tratar oclusão difícil só reativamente (segmentar sprites problemáticos ou clipping) — **não** investir em topo-sort completo no MVP.
3. **Câmera: render-to-texture em baixa resolução + upscale por fator inteiro**, com **snapping da câmera para pixel inteiro por frame**; começar usando `bevy_pixel_camera`/`bevy_pixcam` para encurtar caminho. **Proibir zoom/rotação não-inteiros** no MVP (ou limitar zoom a passos inteiros e rotação a 0°, dado o estilo oblíquo fixo de RO).
4. **Palette: adiar.** Usar texturas normais; só introduzir **indexed + LUT** quando recolor/variantes virarem requisito concreto (é a opção certa, de custo baixo, mas carrega um custo de *pipeline de arte* indexada).

---

## 5. Questões em aberto

1. **Elevação real do terreno vs. "fake height".** RO usa GND com altura por célula; o nosso MVP vai ter terreno com elevação geométrica (afetando projeção e sorting) ou apenas camadas 2D empilhadas com offset? Isso muda radicalmente o depth sort. **[HIPÓTESE]** começar com elevação só como offset de y/z (2.5D "fake") simplifica o MVP.
2. **`bevy_ecs_tilemap` aguenta N camadas + mapa handcrafted + animação sem virar gargalo?** O modelo entidade-por-tile preocupa em mapas grandes; falta benchmark nosso. Precisamos medir antes de descartar a rota custom `wgpu`.
3. **Interação depth buffer × transparência/anti-alias de sprites.** Sprites pixel-art com bordas alpha + depth test podem gerar halos/ordem errada em semi-transparências (sombras, água). Falta validar a política de blending/alpha-cutoff.
4. **Pixel-perfect com câmera que precisa seguir o player suavemente.** Snapping inteiro brigam com movimento sub-pixel suave (ver proposta Godot #6389). Qual compromisso (sub-pixel scroll no upscale? camera motion por inteiros?).
5. **Versões/compatibilidade.** `bevy_ecs_tilemap` acompanha Bevy de perto, mas crates de câmera pixel-perfect (`bevy_pixel_camera`) podem atrasar em relação à versão do Bevy que adotarmos — risco de manutenção.

---

## 6. Fontes

**Tilemap GPU / crates**
- https://docs.rs/wgpu-tilemap/latest/wgpu_tilemap/ (wgpu-tilemap v0.1.1)
- https://github.com/aweinstock314/wgpu-tilemap
- https://blog.paavo.me/gpu-tilemap-rendering/
- https://developer.nvidia.com/gpugems/gpugems2/part-ii-shading-lighting-and-shadows/chapter-12-tile-based-texture-mapping (GPU Gems 2, cap. 12)
- https://github.com/StarArawn/bevy_ecs_tilemap (v0.20)
- https://docs.rs/bevy_ecs_tilemap
- https://bevy.org/examples-webgpu/2d-rendering/tilemap-chunk/

**Depth sorting / y-sort / RO**
- https://gamedevelopment.tutsplus.com/tutorials/isometric-depth-sorting-for-moving-platforms--cms-30226
- https://www.evozon.com/two-unity-tricks-isometric-games/
- https://shaunlebron.github.io/IsometricBlocks/
- https://docs.rs/extol_sprite_layer
- https://github.com/bevyengine/bevy/issues/1275
- https://github.com/bevyengine/bevy/issues/26362
- https://github.com/oldinroplayer/ro-map-viewer
- https://en.wikipedia.org/wiki/Ragnarok_Online

**Pixel-perfect / câmera**
- https://www.slowrush.dev/news/pixel-perfect-rendering/
- https://leahayes.co.uk/2015/03/02/pixel-perfection-in-2d-game-projects.html
- https://spritesheetgenerator.online/blog/pixel-perfect-scaling-nearest-neighbor
- https://docs.rs/bevy_pixel_camera
- https://github.com/atomicptr/bevy_pixcam
- https://github.com/HazelTheWitch/bevy_pixel_perfect
- https://github.com/godotengine/godot-proposals/issues/6389

**Palette shaders**
- https://godotshaders.com/shader/palette-swap-no-recolor-recolor/
- https://godotshaders.com/shader/sprite3d-indexed-color-palette-swapping-shader/
- https://forum.gamemaker.io/index.php?threads/how-to-swap-the-color-palette-using-shader-and-texture.8453/
- https://www.researchgate.net/publication/336310827_Techniques_for_GPU-based_Color_Palette_Mapping
- https://ddrkirbyisq.medium.com/rhythm-quest-devlog-15-palette-shaders-173d697ae053 *(403 na coleta; não verificada integralmente)*
