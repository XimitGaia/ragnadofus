# RFC 0002 — Pipeline de render 2.5D (tiles + sprites, câmera isométrica)

- **Estado:** Draft
- **Eixo / código do backlog:** A2
- **Prioridade:** P0
- **Autor(es):** equipe + pesquisa (agente)
- **Data:** 2026-11-14
- **Pesquisa-fonte:** `../../pesquisa/A2-render-2.5d.md`
- **Depende de:** RFC 0001

## 1. Problema e contexto
Desenhar uma zona feita à mão (tiles + sprites) numa câmera isométrica oblíqua, estilo
Ragnarok Online (sprites 2D sobre terreno com profundidade), com pixel-art nítida e
poucas draw calls.

## 2. Objetivos e não-objetivos
**Objetivos:** render legível e performático da zona do MVP; profundidade correta
(sprite atrás/na frente de objetos); pixel-art sem artefatos.
**Não-objetivos:** representação compacta de assets / 50 MB Challenge (A3, P1);
mundo procedural (B1, P1).

## 3. Pesquisa / evidências
Resumo (detalhe e URLs em `A2-render-2.5d.md`):
- **Tilemap** via `bevy_ecs_tilemap` 0.20 (chunking, culling, GPU-anim, camadas,
  isométrico prontos). **Fallback planejado:** tilemap-de-índices custom em `wgpu`
  (GPU Gems 2 / `wgpu-tilemap`) se "entidade por tile" ou muitas camadas virarem gargalo.
- **Profundidade:** y-sort (`z = f(base_y + elevação)` no `PostUpdate`) + depth buffer;
  topo-sort completo (Shaun LeBron/Tarjan) só reativamente.
- **Câmera:** render-to-texture low-res + upscale **inteiro** com pixel-snapping por
  frame (`bevy_pixel_camera`/`bevy_pixcam`); proibir zoom/rotação não-inteiros.
- **Palette shaders:** adiar; indexed+LUT (1 lookup) só quando recolor for requisito.

## 4. Alternativas / técnicas
| Técnica | Quando | Nota |
|---|---|---|
| `bevy_ecs_tilemap` | Agora (MVP) | Pronto; camadas, iso, GPU-anim |
| Tilemap-de-índices em wgpu | Fallback | Se gargalo de entidade-por-tile/camadas |
| y-sort + depth buffer | Agora | Fonte de verdade = `base_y + elevação` |
| Palette/LUT shader | P1 | Só com requisito de recolor |

## 5. Decisão proposta
MVP com **`bevy_ecs_tilemap` + y-sort + câmera render-to-texture com upscale inteiro**.
Manter a técnica de tilemap-de-índices em wgpu documentada como fallback (e ponte para
o renderer próprio da RFC 0001). Palette shaders ficam para quando houver recolor.

## 6. Impacto e riscos
Fonte de `elevação`/`base_y` vem do modelo de dados de mundo (RFC 0003 — alturas de
canto do tile). **Riscos/incertezas:** (1) elevação geométrica real vs "fake height"
2.5D muda o sorting; (2) `bevy_ecs_tilemap` com N camadas + animação sem benchmark
nosso; (3) depth buffer × alpha dos sprites pode gerar halos (sombras/água).

## 7. Plano de validação
- Cena de teste: terreno com elevação + personagem passando atrás/na frente de objetos.
- Medir draw calls e FPS com o nº de camadas-alvo; validar nitidez pixel em zoom/pan.
- Decidir "fake height" vs elevação real **antes** de fechar o modelo de dados (RFC 0003).

## 8. Questões em aberto
- Elevação real (geométrica) vs 2.5D "fake height"?
- Nº de camadas que `bevy_ecs_tilemap` sustenta no nosso caso.
- Tratamento de transparência/halos no depth buffer.
