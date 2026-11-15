# Sprites do ragnadofus

Os PNGs são os assets para o jogo. Arquivos `.ase` são projetos editáveis do Aseprite; podem ser abertos para retocar pixels e animação. As matrizes de alta resolução usadas na conversão estão em `source/`.

| Pasta | Conteúdo |
|---|---|
| `hero/` | 5 animações × 5 direções; caminhada com 8 poses, ataque com 5 poses e idle com respiração; sombra e metadados |
| `npc/` | Variante de paleta do personagem, mesmas animações |
| `monster/` | Criatura da floresta e animação idle |
| `castle/` | 8 perspectivas externas e interior de 1 cômodo |
| `terrain/ground/` | Grama, terra e pedra, 3 variações cada, losango 64×32 |
| `terrain/elevation/` | Topos, faces e rampas |
| `architecture/` | Parede e casa mínima em peças |
| `props/trees/`, `props/plants/`, `props/rocks/` | Variações de cenário |

As sheets do herói e NPC têm células de 64×96 px, linhas `S, SE, E, NE, N` e colunas de frames. O pivot é (32, 88); `SW`, `W` e `NW` são espelhamentos. Os `.ase` dessas animações têm uma linha de tempo real: cada frame mostra as cinco direções em uma coluna de 64×480 px. O PNG exportado continua em grid para leitura pelo engine.

`castle/castle.json` lista as vistas externas em ordem de rotação, o pivot e o interior. O interior é uma imagem de ambiente com telhado removido; ao entrar, o jogo deve alternar para ela. Os assets ainda não estão conectados à renderização atual do cliente.

Para reproduzir: `python3 sprite/upgrade_from_concepts.py`, `python3 sprite/export_ase.py`, `python3 sprite/organize.py`, `python3 sprite/validate.py`. Execute a partir da raiz do repositório.
