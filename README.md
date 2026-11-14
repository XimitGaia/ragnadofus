# ragnadofus

Projeto de um **RPG online de mundo persistente, exploração contínua e economia
sistêmica**, com estética pixel art de alta densidade e arquitetura gráfica
compacta. Inspirado no que Ragnarok Online e Dofus fazem de melhor — sem copiar
nenhum dos dois.

O desenvolvimento é **AI-first**: uma equipe de humanos e agentes de IA pensando
design, regras, sistemas e implementação, com verificação dentro do jogo.

## Três frentes relacionadas (mas tecnicamente independentes)

| Frente | O que é |
|---|---|
| **Game World** | O universo: biomas, criaturas, classes, economia, crafting, combate, progressão, persistência. |
| **Compact Engine** | O motor: renderização 2.5D, representação compacta de assets, streaming, física, animação. |
| **AI Game Studio** | A plataforma de produção: ferramentas para designers, artistas, engenheiros e agentes criarem o universo. |

> Regra-mestra do projeto: **não deixar a ambição do motor consumir o jogo, nem o
> jogo travar a pesquisa do motor.** Sempre distinguir *hipótese de design*,
> *hipótese de engenharia* e *meta experimental mensurável*.

## Mapa do repositório

```
docs/
  visao/
    00-ideias-brutas.md        Transcrição crua da conversa original (não editar)
    01-visao-inicial-v0.1.md   Consolidação v0.1 (ex-.docx) (não editar)
  sintese.md                   O que vamos construir + definição de MVP
  pontos-chave.md              Pontos-chave → backlog de RFCs + perguntas de pesquisa
  rfcs/
    README.md                  Processo de RFC + índice
    0000-template.md           Modelo de RFC
pesquisa/                       Saídas da pesquisa técnica/design
```

## Estado atual

Fase de **organização e pesquisa**. Nenhuma decisão de engine, câmera, combate,
escopo ou tamanho final está fechada. Ver `docs/sintese.md` e `docs/rfcs/`.
