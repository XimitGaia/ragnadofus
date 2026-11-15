# RFCs

Uma **RFC** (Request for Comments) registra uma decisão técnica ou de design:
o problema, as alternativas pesquisadas, o trade-off e a escolha. Ela existe para
transformar uma *hipótese aberta* da visão numa *decisão rastreável*.

## Processo

1. **Rascunho** — copie `0000-template.md` para `NNNN-titulo-curto.md`. Estado `Draft`.
2. **Pesquisa** — preencha com a segunda pesquisa (fontes em `../../pesquisa/`).
3. **Discussão** — comentários, alternativas, riscos.
4. **Decisão** — estado `Accepted` (ou `Rejected` / `Superseded by NNNN`).
5. **Implementação** — a RFC vira referência; divergências viram nova RFC.

## Estados

`Draft` · `In Review` · `Accepted` · `Rejected` · `Superseded`

## Índice

| # | Título | Estado | Prio | Backlog |
|---|---|---|---|---|
| [0001](0001-base-de-engine.md) | Base de engine: Bevy + renderer próprio | In Review | P0 | A1 |
| [0002](0002-render-2.5d.md) | Pipeline de render 2.5D | Draft | P0 | A2 |
| [0003](0003-movimento-navegacao.md) | Movimento contínuo, colisão e navegação | Draft | P0 | B3 |
| [0004](0004-economia-crafting.md) | Loop de coleta → crafting | Draft | P0 | C2 |
| [0005](0005-cliente-servidor.md) | Cliente-servidor autoritativo + persistência | In Review | P0 | D1/B2 |
| [0006](0006-harness-ia.md) | Harness mínimo de dev com IA | Draft | P0 | E1 |
| [0007](0007-escalabilidade-mundo-rede.md) | Escalabilidade: chunking, interesse de rede, culling | Draft | P1 | Escala |

_Backlog completo (P1/P2 ainda sem RFC): ver `../pontos-chave.md`._

> A numeração das RFCs **não** precisa seguir os códigos A1/B2 do backlog; o backlog
> organiza por eixo, as RFCs são numeradas em ordem de criação.
