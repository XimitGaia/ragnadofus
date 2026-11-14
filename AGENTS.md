# AGENTS.md — arquitetura de trabalho IA-first

Documento canônico para **qualquer agente** (Claude ou outro) que trabalhe neste
repositório. O projeto é **AI-first**: humanos e agentes pensam design, regras,
sistemas e implementação juntos, com **verificação dentro do jogo**. Leia isto antes
de agir.

## 1. O que é o projeto
`ragnadofus` — RPG online 2.5D de mundo contínuo e economia sistêmica, pixel-art de
alta densidade, motor compacto, desenvolvido com agentes. Três frentes independentes:
**Game World** (universo), **Compact Engine** (motor), **AI Game Studio** (plataforma).
Visão completa em `README.md` e `docs/sintese.md`.

## 2. Mapa de documentos (leia nesta ordem)
| Arquivo | Para quê |
|---|---|
| `docs/sintese.md` | O que estamos construindo + o MVP decidido |
| `docs/pontos-chave.md` | Backlog de RFCs por eixo e prioridade |
| `docs/rfcs/` | Decisões técnicas/design (uma por arquivo) + processo |
| `docs/plano-decisoes.md` | Log vivo das decisões (percorrido pergunta a pergunta) |
| `docs/demo-01-spec.md` | Spec consolidada da demo atual |
| `docs/debito-tecnico.md` | Atalhos deliberados que **devem** ser elevados |
| `pesquisa/` | Briefs de pesquisa com fontes citadas |
| `docs/visao/` | Fontes originais — **não editar** |

## 3. Regras inegociáveis
1. **code-intel (MCP) é o carro-chefe para TODA operação de código.** Achar/contar
   referências, renomear, mover, extrair, deletar símbolos, mudar assinatura,
   inspecionar e editar com verificação → sempre via code-intel. **grep/sed só para
   TEXTO LITERAL.** Em Rust, usar `verify_build`; `blast_radius` antes de edição ampla;
   `safe_delete` para remover. Nunca refatorar símbolo com grep/sed.
2. **Verificação é um gate, não uma afirmação.** Um agente dizer "terminei" não vale
   nada sem evidência: build+teste verdes, e screenshot/log quando a mudança for
   visual/interativa (ver RFC 0006). O gate bloqueia a entrega; não se contorna.
3. **Distinguir sempre:** hipótese de design · hipótese de engenharia · meta
   experimental · decisão · requisito confirmado. Não fechar escopo prematuramente.
4. **Atalho consciente vira débito registrado.** Simplificou para andar rápido?
   Registre em `docs/debito-tecnico.md` com gatilho para pagar. Não deixe um atalho
   virar, por esquecimento, a arquitetura final.
5. **Nada de copiar IP:** estudar Ragnarok/Dofus como referência, **sem** reutilizar
   assets, código ou propriedade intelectual.

## 4. Como decidir (fluxo)
Decisões abertas são tratadas **uma de cada vez, com opções e uma recomendação**, e
registradas em `docs/plano-decisoes.md`. Pontos menores podem receber default
provisório marcado 🔵. Decisões técnicas relevantes viram RFC (`docs/rfcs/`), embasadas
por pesquisa com fontes (`pesquisa/`).

## 5. Stack (ver RFCs 0001/0005 para versões)
- **Rust + Bevy 0.20.x** como base; renderer próprio como módulo substituível (não
  reescrever a engine). Mesmo ECS compartilhado cliente↔servidor.
- Cliente desktop; servidor **Bevy headless**. Rede: `bevy_replicon` (+`lightyear`
  quando o netcode subir de nível — DT-01). Física: `avian2d`. Persistência atrás de um
  trait `WorldRepository` (backend trocável).
- Deploy de dev: **home cluster k8s**, servidor via **MetalLB LoadBalancer UDP**.

## 6. Convenções de código e Git
- Workspace Cargo com crates separados (ex.: `shared/`, `client/`, `server/`).
- Código novo imita o estilo ao redor (nomes, idioma, densidade de comentários).
- **Conventional Commits.** Commitar/pushar **só quando o usuário pedir**. Mensagens de
  commit terminam com:
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`
- Gate de CI mínimo: `cargo build && cargo test` (headless) bloqueia entrega.

## 7. O que NÃO fazer sem pedir
Fechar decisões marcadas como hipótese; introduzir dependências pesadas sem RFC;
editar `docs/visao/` (fontes); apagar arquivos do usuário; pushar para remoto.
