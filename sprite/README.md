# Sprites do ragnadofus

Os PNGs são os assets para o jogo. Arquivos `.ase` são projetos editáveis do Aseprite; podem ser abertos para retocar pixels e animação. As matrizes de alta resolução usadas na conversão estão em `source/`.

| Pasta | Conteúdo |
|---|---|
| `hero/` | Personagem detalhado: 5 animações × 8 direções; caminhada com 8 poses, ataque com 5 poses, idle com respiração; sombra e metadados |
| `npc/` | Variante de paleta do personagem, mesmas animações |
| `monster/` | Criatura da floresta e animação idle |
| `castle/` | 8 perspectivas externas e interior de 1 cômodo |
| `terrain/ground/` | Grama, terra e pedra, 3 variações cada, losango 64×32 |
| `terrain/elevation/` | Topos, faces e rampas |
| `architecture/` | Parede e casa mínima em peças |
| `props/trees/`, `props/plants/`, `props/rocks/` | Variações de cenário |

As sheets do herói têm células de **320×512 px**, linhas `S, SE, E, NE, N, NW, W, SW` e pivot (160, 480). Os pixels e cores vêm das matrizes detalhadas em `source/`, sem redução para a antiga escala 64×96 e sem quantização para 32 cores. As sheets do NPC ainda seguem o guia antigo de 64×96 px. Os `.ase` do herói têm uma linha de tempo real: cada frame mostra as oito direções em uma coluna de 320×4096 px.

`castle/castle.json` lista as vistas externas em ordem de rotação, o pivot e o interior. O interior é uma imagem de ambiente com telhado removido; ao entrar, o jogo deve alternar para ela. Os assets ainda não estão conectados à renderização atual do cliente.

Para reproduzir o herói detalhado: `python3 sprite/hero/build_hd.py`, `python3 sprite/hero/export_hd_ase.py`, `python3 sprite/validate.py`. Execute a partir da raiz do repositório. O pipeline antigo (`upgrade_from_concepts.py` + `organize.py`) ainda gera os outros elementos em baixa resolução; para manter o herói detalhado, execute `build_hd.py` por último.
