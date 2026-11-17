# Direção visual — referências RO + Dofus + FFT (levantamento)

> **Regra de IP (inegociável):** este documento analisa a **direção de arte** (estética:
> perspectiva, proporção, paleta, luz, humor) dessas três referências para guiar a criação
> de **arte 100% ORIGINAL** no mesmo espírito. **Nunca** copiar/ripar assets, sprites,
> mapas ou personagens de Ragnarok Online, Dofus ou Final Fantasy Tactics. Estilo não se
> copia — se interpreta.

## 1. Por que essas três (e por que Kenney não serve)

O usuário rejeitou o Kenney: é **low-poly/voxel "ocidental" limpo e geométrico** — bom,
mas frio e sem o charme pintado/fofo das três referências. RO/Dofus/FFT compartilham um
DNA: **2.5D fofo-medieval, pintado à mão, legível, acolhedor.** É esse DNA que buscamos.

## 2. Análise das referências

### Ragnarok Online (RO)
- **Técnica:** **2.5D híbrido** — terreno/estruturas em **3D** (tiles) + personagens e
  monstros em **sprite 2D (billboard)** que sempre encaram a câmera.
- **Câmera:** tilt fixo olhando de cima, **rotação em passos**, perspectiva leve.
- **Personagem:** **chibi** (~2–3 cabeças), sprite desenhado com **contorno suave**,
  paleta **pastel saturada**, expressões fofas.
- **Humor:** aconchegante, nostálgico, luz macia, sem sombras duras.
- **→ Nosso engine JÁ é isso:** mundo 3D + sprites billboard + câmera orbital em volta do
  personagem. Arquitetura casada.

### Dofus
- **Técnica:** **2D puro** vetorial/pintado (origem Flash), grade **isométrica (losango)**.
- **Direção forte:** **formas ousadas, contorno grosso, cores chapadas porém ricas**,
  cartunesco, **silhuetas limpas**, alta legibilidade. Personagens expressivos (cabeção,
  exagero). Cenários **pintados**.
- **Paleta:** viva, quente, bom contraste.
- **→ Pra nós:** referência de **legibilidade e contorno** — silhueta clara, outline,
  cor que "canta".

### Final Fantasy Tactics (FFT)
- **Técnica:** **2.5D isométrico tático** — terreno em **blocos 3D low-poly** + personagens
  em **sprite 2D**. Rotação da câmera em 90°.
- **Personagem:** chibi, sprite pequeno **detalhado**, paleta **terrosa/medieval** mais
  sóbria. Cenários pintados (direção do Yoshida), atmosféricos.
- **Humor:** fantasia medieval séria, cores ricas e suaves, clima.
- **→ Pra nós:** referência de **mundo em blocos/tiles com textura pintada** (não cinza
  chapado) e **paleta medieval** com profundidade.

## 3. Denominador comum → NOSSA direção

| Eixo | Direção adotada |
|---|---|
| **Formato** | **2.5D**: mundo 3D (tiles/estruturas) + **personagens em SPRITE 2D billboard** (NÃO modelo voxel/3D — por isso Kenney Blocky não serve pro herói) |
| **Câmera** | isométrica/oblíqua, rotação livre (já temos), perspectiva suave/quase ortográfica (já é ortográfica) |
| **Proporção do boneco** | **chibi ~2,5 cabeças**, cabeção, membros curtos |
| **Contorno** | outline suave (RO) a médio (Dofus) — silhueta sempre legível |
| **Paleta** | **quente, saturada mas macia**; verdes/terras pro mundo, acentos vivos; evitar cinza "placeholder" |
| **Iluminação** | luz ambiente alta + direcional macia; **sem sombra dura**; clima aconchegante |
| **Texturas do mundo** | **pintadas/estilizadas** (não flat liso), com ruído/variação sutil |
| **Legibilidade** | silhuetas limpas, ícones/props reconhecíveis à primeira vista |

## 4. Implicações práticas pros assets

- **Personagem = SPRITE 2D** (RO/FFT/Dofus), não modelo 3D. Então:
  - manter o **billboard** que já temos;
  - trocar os placeholders por sprites chibi **com mais acabamento** (contorno + sombreado +
    8 direções + andar/atacar). Fontes possíveis: arte **comissionada**, geração **original**
    (minha, evoluída) ou base **LPC** (ciente da licença CC-BY-SA, só placeholder).
- **Mundo/props:** aqui o CC0 low-poly (Kenney/KayKit) **destoa** da pegada pintada. Opções:
  - tiles/props **pintados** (texturas estilizadas em malha 3D simples) — dá pra fazer sob
    medida com textura, mantendo o engine;
  - ou aceitar um low-poly **mais quente** (KayKit com paleta ajustada) como meio-termo.
- **Paleta oficial (rascunho):** definir ~16 cores-chave (grama, terra, pedra, madeira,
  pele, cabelos, acentos) pra consistência — próximo passo.

## 5. Próximos passos sugeridos

1. Fechar a **paleta** (16 cores) e um **mood board original** (referências só como norte).
2. Decidir a **origem do sprite do personagem**: comissionar vs. eu evoluir o original vs.
   LPC-placeholder.
3. Protótipo de **1 tile pintado + 1 prop pintado** no engine, pra validar a pegada antes
   de produzir em massa.
4. Registrar licenças em `creditos-assets.md` conforme entrarem assets de terceiros.

> Resumo: nosso **engine (2.5D: mundo 3D + sprite billboard + câmera orbital)** já é
> estruturalmente o de RO/FFT. O salto de qualidade é de **arte** (sprites chibi pintados +
> mundo com textura estilizada quente), não de arquitetura.
