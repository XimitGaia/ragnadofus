# Débito técnico

Simplificações **deliberadas** feitas na demo/MVP para avançar rápido, que **devem
ser elevadas depois**. Registrar aqui preserva a distinção entre "decisão final" e
"atalho consciente" — e evita que um atalho vire, por esquecimento, a arquitetura
definitiva.

Legenda: 🔴 crítico (bloqueia qualidade-alvo) · 🟡 importante · 🟢 oportunista

| ID | Débito | Onde | Sev. | Elevar para | Gatilho p/ pagar |
|---|---|---|---|---|---|
| **DT-01** | Netcode da demo usa **replicon puro**, sem client-side prediction/rollback/lag-compensation | RFC 0005 · D4.1 | 🔴 | **Estado da arte**: predição+rollback+interpolação+lag-comp (via `lightyear` ou implementação própria de nível AAA) | Antes de avaliar seriamente a **sensação** de movimento online, ou ao passar de 2 p/ muitos jogadores |
| **DT-02** | Sem **cliente web** na demo (só desktop, via renet/UDP) | RFC 0005 · D4.2 | 🟡 | Cliente WASM no navegador via **QUIC/WebTransport** (`aeronet`+`quinn`) | Quando distribuição via navegador virar requisito |

> **Princípio do usuário:** a camada de engenharia de rede **tem que ser estado da
> arte** — o atalho da demo existe só para provar o loop cliente-servidor, não para
> definir a qualidade final do netcode.
