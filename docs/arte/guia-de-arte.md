# Guia de Arte — ragnadofus (para o artista)

Guia prático para produzir os **sprites de personagem** e o **tileset** do cenário
mínimo. O objetivo é que a arte **caia direto no engine** sem retrabalho. Dúvidas →
falar com a equipe antes de produzir em escala.

> **Propriedade intelectual:** estudar Ragnarok Online / Dofus como **referência de
> estilo**, mas **não copiar** sprites, tiles ou qualquer asset. Tudo original.

---

## 1. Estilo e restrições técnicas (valem para TUDO)

- **Pixel-art** nativo (desenhar em 1×; o engine faz **upscale inteiro**, ~2×). Sem
  anti-aliasing nas bordas, sem cores "borradas" — pixel limpo.
- **Vista 2.5D isométrica 3/4** (tipo Ragnarok), chão em **losango (diamante)**.
- **Amostragem nearest-neighbor** (pixel perfeito) — nada de texturas suaves.
- **Paleta indexada e limitada** (recomendado **16–32 cores** por "material"). Facilita
  consistência e _palette swaps_ futuros (cor de roupa/pele/time). Entregar a paleta
  (`.pal`/`.ase`) junto.
- **Fundo transparente** (PNG RGBA). 
- **Ferramenta recomendada:** Aseprite (entregar o `.ase` editável **e** os PNGs
  exportados).

---

## 2. Grade do mundo (medidas que amarram tudo)

- **Tile de chão (losango iso):** **64 × 32 px** (largura × altura). Essa é a célula.
- **Câmera:** segue o personagem; upscale inteiro + nearest.
- **Movimento:** _click-to-move_ estilo Ragnarok — o personagem anda **célula a célula**
  na grade, em **8 direções** (as 8 diagonais/lados do losango).
- **Ordenação (y-sort):** quem está "mais à frente" (pés mais embaixo na tela) desenha
  por cima. Por isso o **pivot (âncora) nos pés** do personagem é **crítico**.

---

## 3. Personagem jogável (sprite sheets)

### 3.1 Direções
- **Ideal: 8 direções** — `S, SE, E, NE, N, NW, W, SW` (S = de frente para a câmera).
- **Truque do RO (economiza 3):** desenhar só **5 direções** (`S, SE, E, NE, N`) e
  **espelhar horizontalmente** para obter `SW, W, NW`. Entregar as 5; a equipe espelha.
- **Mínimo para o 1º protótipo:** **4 direções** (`SE, SW, NE, NW` — as diagonais, que
  são as mais usadas no iso). Dá para começar e expandir.

### 3.2 Animações (nesta prioridade)
| Anim | Frames sugeridos | FPS sugerido | Observação |
|---|---|---|---|
| **idle** (parado) | 2–4 | 4–6 | respiração leve |
| **walk** (andar) | 6–8 | 8–10 | ciclo de caminhada; loop perfeito |
| **attack** (ataque) | 4–6 | 10–12 | 1 golpe; frame de impacto claro |
| **jump** (salto) | 3–5 | 8 | agachar → subir → ápice |
| **fall** (queda) | 2–3 | 8 | caindo + pouso |
| _(opcional)_ hurt/die | 2–4 | — | se sobrar tempo |

### 3.3 Canvas, pivot e escala
- **Canvas por frame: 64 × 96 px** (mesmo tamanho para TODOS os frames de TODAS as
  animações/direções — não variar o canvas).
- **Pivot (âncora): x = 32, y = 88** — centro horizontal, na **base dos pés**. Os pés do
  personagem devem pisar sempre nesse ponto, em todos os frames (senão "treme"/desalinha
  na célula e quebra o y-sort).
- **Altura visual do corpo:** ~56–72 px (ocupa ~1,5–2 tiles de altura). Humano "normal".
- **Sombra:** um oval escuro translúcido na base ajuda a "assentar" na célula (pode ser
  um sprite à parte ou integrado; preferimos **à parte**, reutilizável).

### 3.4 Organização dos arquivos (muito importante)
Entregar **um PNG por animação**, em **grid**: _linhas = direções, colunas = frames_.
- Nome: `hero_<anim>.png` → `hero_idle.png`, `hero_walk.png`, `hero_attack.png`,
  `hero_jump.png`, `hero_fall.png`.
- Grid: cada célula do grid = 64 × 96 px. Ex.: `hero_walk.png` com 5 linhas (direções
  S,SE,E,NE,N) × 8 colunas (frames) = 320 × 480 px.
- Junto, um **`hero.json`** (ou planilha) com, por animação: nº de frames, fps, ordem das
  direções, pivot. (Modelo no fim do doc.)

---

## 4. Tileset (chão + elevação + paredes)

Todos os tiles de chão no formato **losango 64 × 32**, encaixáveis lado a lado.

### 4.1 Chão
- **Grama**, **terra/caminho**, **pedra** (3 materiais base). 2–3 variações de cada para
  não repetir visualmente.
- _(Opcional)_ tiles de **transição/borda** entre materiais (grama→terra etc.).

### 4.2 Elevação (altura real)
O mundo tem **altura** (platôs e rampas). Para cada nível de elevação precisamos:
- **Topo** do platô (mesmo losango de chão).
- **Faces laterais** (a "parede de terra/pedra" que aparece na frente do degrau) — a
  altura de 1 degrau no engine corresponde a ~**16 px** verticais. Entregar faces para
  grama/terra/pedra.
- **Rampa** (tiles inclinados ligando dois níveis).

### 4.3 Paredes e Casa
- **Segmentos de parede** iso: horizontal, vertical, e os 4 **cantos**.
- **Casa mínima:** parede externa, **porta** (vão + batente), **telhado** (uma peça que
  **some/fica translúcida quando o personagem entra** — ver decisão D1.4), **piso
  interno** (um material de chão distinto).

### 4.4 Props (cenário mínimo)
- **Árvore**, **pedra/rocha**, **arbusto** (1–2 variações cada). Mesmo pivot-nos-pés para
  entrarem no y-sort.

---

## 5. Lista mínima de assets (checklist do cenário)

| # | Asset | Formato | Status |
|---|---|---|---|
| 1 | Personagem `hero` — idle/walk/attack/jump/fall × direções | PNG grid + json | ☐ |
| 2 | NPC — pode ser o `hero` com **paleta trocada** (ou sprite simples) | PNG grid | ☐ |
| 3 | Sombra do personagem (oval translúcido) | PNG | ☐ |
| 4 | Chão iso: grama / terra / pedra (+2 variações cada) | PNG losango 64×32 | ☐ |
| 5 | Elevação: topo + faces (grama/terra/pedra) + rampa | PNG | ☐ |
| 6 | Paredes: horizontal, vertical, 4 cantos | PNG | ☐ |
| 7 | Casa: parede, porta, telhado (fade ao entrar), piso interno | PNG | ☐ |
| 8 | Props: árvore, pedra, arbusto | PNG | ☐ |
| 9 | Paleta compartilhada | `.ase`/`.pal` | ☐ |

Com 1, 3, 4 e 7 já dá para montar a **primeira cena jogável** (personagem andando numa
zona com chão variado e uma casa onde dá para entrar).

---

## 6. Entrega (checklist técnico)

- [ ] PNG RGBA, fundo transparente, **sem** anti-aliasing.
- [ ] Personagem: **canvas 64×96 fixo**, **pivot (32, 88)** consistente em todos os frames.
- [ ] Tiles de chão: **losango 64×32**, encaixáveis.
- [ ] Um PNG por animação em **grid (direções × frames)**.
- [ ] Paleta indexada (16–32 cores por material) entregue junto.
- [ ] Arquivos `.ase` editáveis + PNGs exportados.
- [ ] `hero.json` com metadados das animações (modelo abaixo).

### Modelo `hero.json`
```json
{
  "canvas": { "w": 64, "h": 96 },
  "pivot": { "x": 32, "y": 88 },
  "directions": ["S", "SE", "E", "NE", "N"],
  "mirror_for": { "SW": "SE", "W": "E", "NW": "NE" },
  "animations": {
    "idle":   { "frames": 4, "fps": 5,  "loop": true  },
    "walk":   { "frames": 8, "fps": 9,  "loop": true  },
    "attack": { "frames": 5, "fps": 11, "loop": false },
    "jump":   { "frames": 4, "fps": 8,  "loop": false },
    "fall":   { "frames": 3, "fps": 8,  "loop": false }
  }
}
```

---

## 7. Onde isso encaixa no engine (contexto)

- Hoje há **placeholders** (retângulos coloridos + tiles de losango gerados) só para
  provar o movimento/rede. A arte real **substitui** esses placeholders.
- O engine posiciona o sprite na **célula** (projeção iso exata do tilemap) e ordena por
  **y** (pés) — por isso o **pivot nos pés** é inegociável.
- Troca de cor (paleta) é usada para variar NPCs/jogadores sem redesenhar — por isso a
  **paleta indexada**.
