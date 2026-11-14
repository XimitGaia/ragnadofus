# CLAUDE.md

> **As instruções deste repositório vivem em [`AGENTS.md`](./AGENTS.md).**
> Ele é a fonte única de arquitetura de trabalho IA-first (vale para Claude e qualquer
> outro agente). Leia `AGENTS.md` antes de agir.

Lembrete do mais inegociável (detalhe em `AGENTS.md`):

1. **code-intel (MCP) é o carro-chefe para toda operação de código** — grep/sed só para
   texto literal.
2. **Verificação é um gate, não uma afirmação** — build+teste (e evidência visual
   quando couber) antes de dizer "pronto".
3. **Atalho consciente → `docs/debito-tecnico.md`.**
