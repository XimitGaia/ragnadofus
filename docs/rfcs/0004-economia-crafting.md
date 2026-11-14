# RFC 0004 — Loop de coleta → crafting (economia mínima)

- **Estado:** Draft
- **Eixo / código do backlog:** C2
- **Prioridade:** P0
- **Autor(es):** equipe + pesquisa (agente)
- **Data:** 2026-11-14
- **Pesquisa-fonte:** `../../pesquisa/C2-economia-crafting.md`
- **Padrões aplicáveis:** `../padroes-engenharia-e-design.md` §3
- **Depende de:** RFC 0005 (autoridade/persistência no servidor)

## 1. Problema e contexto
O loop jogável do MVP é **coleta → crafting**: colher um recurso, fabricar um item,
persistir no servidor e tornar visível a outros jogadores. É a herança sistêmica de
Dofus e deve nascer interdependente, sem virar grind.

## 2. Objetivos e não-objetivos
**Objetivos:** um loop jogável que **gera comércio desde o início** via
interdependência; tudo autoritativo no servidor (anti-cheat).
**Não-objetivos:** combate (C1, P1); árvore completa de profissões; casa de leilões
complexa (decidir mercado vs troca direta — ver §8).

## 3. Pesquisa / evidências
Resumo (detalhe e URLs em `C2-economia-crafting.md`):
- **[DESIGN]** MVP com **2 profissões de coleta + 2 de craft** com **insumos cruzados
  obrigatórios** — nenhuma autossuficiente. Cadeia: Minerador (minério) + Lenhador
  (madeira) → Ferreiro (lâmina) + Entalhador (cabo); item final exige as **duas** cadeias.
  Passos curtos (1–2) p/ evitar grind.
- **[FATO]** Tudo autoritativo no servidor: coleta (claim exclusivo de nó + respawn),
  consumo/output de craft e mutações de inventário como **transações atômicas com
  rollback**; cliente só envia **intenção**; XP só por ação validada; `TransactionLog`
  p/ auditoria. Padrão confirmado por EVE, Albion e literatura anti-cheat.

## 4. Modelo mínimo para o MVP
- **Entidades:** `ResourceNode` (tipo, estado, timer de respawn), `Inventory`,
  `Recipe` (insumos→output), `CraftingStation`.
- **Fluxo:** intenção de colher → servidor valida claim+estado → concede item + XP →
  persiste; intenção de craftar → servidor valida insumos → transação atômica
  (consome/produz) → persiste. `TransactionLog` registra tudo.

## 5. Extensões futuras (P1+)
Mercado/casa de leilões; durabilidade/sinks; mais profissões; qualidade/raridade;
nós contestados vs instanciados.

## 6. Impacto e riscos
Depende inteiramente da autoridade e persistência da RFC 0005.
**Requisito de design (P3):** o *sink* **não é opcional** — "os jogadores são a
simulação econômica", e **todo item craftado precisa de uma razão para ser consumido ou
perdido**, senão o mercado satura (lição de EVE/Albion vs. Ultima Online). O *mecanismo*
do sink pode ser faseado, mas a existência dele é requisito, não ideia.

## 7. Plano de validação
- Dois clientes: um colhe minério, outro madeira; um craft exige ambos → força troca.
- Teste de concorrência: dois jogadores tentando o mesmo nó (claim exclusivo).
- Teste anti-cheat: intenção forjada rejeitada pelo servidor.

## 8. Questões em aberto
1. **Qual mecanismo de sink** (durabilidade, consumo, perda em morte…) e em que fase —
   a *existência* do sink já é requisito (ver §6).
2. **Mercado** na v1 ou só troca direta/visibilidade?
3. Nó de recurso **compartilhado-contestado vs instanciado** por jogador.
> Fontes oficiais de Dofus ficaram parcialmente bloqueadas (402/403) — confirmar
> números na versão final.
