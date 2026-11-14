# RFC 0006 — Harness mínimo de desenvolvimento com IA

- **Estado:** Draft
- **Eixo / código do backlog:** E1
- **Prioridade:** P0 (mínimo) / P1 (completo)
- **Autor(es):** equipe + pesquisa (agente)
- **Data:** 2026-11-14
- **Pesquisa-fonte:** `../../pesquisa/E1-harness-ia.md`
- **Depende de:** RFC 0001

## 1. Problema e contexto
O projeto é AI-first. Princípio inegociável: **"um agente afirmar que concluiu não
substitui evidência de que a build funciona e a experiência visual/interativa está
correta."** Precisamos de um loop: agente propõe → gera artefatos → build/testes →
screenshot/logs → **evidência**.

## 2. Objetivos e não-objetivos
**Objetivos:** gate automatizado que **bloqueia entrega** sem build+teste verdes;
evidência visual anexada a mudanças de render/UI; inspeção de estado do jogo por agente.
**Não-objetivos:** AI Game Studio como produto (E2, P2); visual-diff sofisticado no MVP.

## 3. Pesquisa / evidências
Resumo (detalhe e URLs em `E1-harness-ia.md`):
- **[FATO]** Build+teste **headless** (`cargo build && cargo test` com `app.update()` +
  `TimeUpdateStrategy`, sem `DefaultPlugins`) como **gate** que bloqueia em falha.
- **[FATO]** Screenshot headless pela API nativa `Screenshot`/`ScreenshotCaptured` →
  `save_to_disk`, anexado a toda mudança de render/UI.
- **[FATO]** **BRP** (Bevy Remote Protocol: `RemotePlugin` + `RemoteHttpPlugin`) ligado
  em dev → query/mutação/snapshot do ECS por agentes.
- **[FATO]** Determinismo mínimo: gameplay em `FixedUpdate` + RNG semeado; gravar
  seed+inputs+versão p/ reprodutibilidade.
- **Princípio (regolo.ai):** verificação é um **gate no control-flow**, não texto de
  prompt; e os critérios do gate devem ser **revelados ao agente**.

## 4. Componentes do harness mínimo
1. **Gate build+teste headless** — pré-condição de qualquer entrega.
2. **Captura de screenshot headless** — artefato obrigatório em mudança visual.
3. **BRP em dev** — inspeção/mutação do ECS.
4. **Núcleo determinístico** — `FixedUpdate` + seed; sessão reproduzível (seed+inputs+versão).

## 5. Decisão proposta
Implementar os 4 componentes acima como esqueleto executável **desde o início do MVP**.
Fases seguintes (P1): visual-diff contra baseline com tolerância; `bevy_mcp_harness`/
MCP p/ injeção de input e playtest; revisor L3.

## 6. Impacto e riscos
Usa o Bevy headless (RFC 0001) — mesmo runtime do servidor (RFC 0005). **Incertezas:**
(a) não há crate Rust canônico de visual-diff integrado a Bevy → provável implementação
própria, risco de flakiness GPU/driver; (b) headless+GPU (wgpu) em CI pode exigir
software rendering, ameaçando baselines → precisa PoC; (c) determinismo não é garantido
por subsistema (física/partículas divergem com seed) → definir o "núcleo determinístico"
verificável.

## 7. Plano de validação
- PoC de CI headless com screenshot estável (ou software rendering).
- Um gate real: PR que quebra build/teste é **bloqueado** automaticamente.
- Reproduzir um bug a partir de seed+inputs gravados.

## 8. Questões em aberto
Visual-diff próprio e sua tolerância; estabilidade de headless+GPU no CI; fronteira do
núcleo determinístico; alinhar versão de Bevy do tooling (mira 0.19–0.20) com a do projeto.
