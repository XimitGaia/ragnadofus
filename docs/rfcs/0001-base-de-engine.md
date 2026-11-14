# RFC 0001 — Base de engine: Bevy + renderer próprio substituível

- **Estado:** In Review
- **Eixo / código do backlog:** A1
- **Prioridade:** P0
- **Autor(es):** equipe + pesquisa (agente)
- **Data:** 2026-11-14
- **Pesquisa-fonte:** `../../pesquisa/A1-base-engine.md`
- **Padrões aplicáveis:** `../padroes-engenharia-e-design.md` §1

## 1. Problema e contexto
Precisamos de uma base de runtime (ECS, janela, input, asset pipeline, render) para
o cliente — e, idealmente, reaproveitável no servidor headless — sem escrever uma
engine inteira do zero. A visão quer um **renderer compacto próprio** como diferencial
de engenharia, mas sem que ele consuma o jogo. A questão é: sobre o que construímos e
como preservamos a opção de substituir a camada de render.

## 2. Objetivos e não-objetivos
**Objetivos:** base madura em Rust; caminho real para render próprio; mesmo ECS no
cliente e servidor (ver RFC 0005); pixel-art/tilemap de primeira classe.
**Não-objetivos:** escolher a arquitetura final de render compacto (RFC 0002/A3);
provar escala de MMO agora (é risco, não objetivo do MVP).

## 3. Pesquisa / evidências
Resumo (detalhe e URLs em `A1-base-engine.md`):
- **[FATO]** Bevy em 2026 (v0.20.1) é a engine Rust mais madura com ECS + janela +
  input + assets + render WGSL coesos; governança via Foundation.
- **[FATO]** Desde **Bevy 0.17** o renderer é *drop-in substituível* sem `bevy_render`;
  **Tiny Glade** e **Tunnet** shiparam em produção com "ECS-only + renderer próprio" —
  exatamente a estratégia desejada (nosso módulo = wgpu no lugar de `bevy_render`).
- **[FATO]** Macroquad tem soundness pendente (RUSTSEC-2025-0035, sem fix) e não tem
  ECS; wgpu puro exige reconstruir janela/input/assets.
- **[FATO]** Networking: `bevy_replicon` (servidor-autoritativo) + `aeronet`, com
  `lightyear` para prediction/rollback (ver RFC 0005).

## 4. Alternativas consideradas
| Alternativa | Prós | Contras | Custo |
|---|---|---|---|
| **Bevy base + renderer próprio (módulo)** | Maduro; ECS cliente+servidor; render substituível comprovado; ecossistema net | Churn pré-1.0; manter módulo acoplado ao core | Médio |
| Macroquad | Simples p/ protótipo 2D | Sem ECS; soundness RUSTSEC-2025-0035; não escala p/ o projeto | Baixo→becobeco |
| wgpu puro desde já | Controle total da representação compacta | Reconstruir janela/input/assets/ECS; lento | Alto |

## 5. Decisão proposta
**Confirmar Bevy como base**, com o render próprio entrando como **módulo plugável**
no lugar de `bevy_render` (padrão Tiny Glade/Tunnet), não como reescrita de engine.
Começar usando o render padrão do Bevy + `bevy_ecs_tilemap` (RFC 0002) e substituir
por partes só com evidência de ganho.

**Padrão de arquitetura (P1):** workspace com **grafo de crates unidirecional**
(`client → shared`, `server → shared`; nunca `shared → *` nem `client ↔ server`).
`shared` é o ECS do jogo em Bevy puro, **sem `bevy_render`**; o **servidor compila sem
renderer**, provando o desacoplamento a cada build (apoiado no decoupling de
`bevy_render` desde Bevy 0.17). Verificar o grafo com `cargo deny`/`cargo tree` no CI.

## 6. Impacto e riscos
Condiciona RFC 0002 (render), 0005 (mesmo ECS no servidor) e 0006 (BRP/headless do Bevy).
**Riscos:** (1) escala de MMO não comprovada em Bevy → exige benchmark próprio cedo;
(2) custo de manter renderer acoplado a cada release; (3) churn pré-1.0 → orçar
migração ~trimestral, não congelar versão.

## 7. Plano de validação
- Fixar **Bevy 0.20.x** e abrir um "spike" de atualização trimestral orçado.
- Benchmark de escala cedo (ver RFC 0005): N agentes/zona no servidor headless.
- PoC do padrão "ECS-only + renderer custom" só **depois** do MVP jogável, para não
  drenar o jogo.

## 8. Questões em aberto
- Quando (se) extrair o renderer próprio — critério objetivo de disparo.
- Alvo de nº de jogadores/agentes por zona que a base precisa sustentar.
