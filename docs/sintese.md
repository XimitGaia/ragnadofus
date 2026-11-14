# Síntese: o que vamos construir

Documento de leitura rápida que destila a visão (ver `visao/01-visao-inicial-v0.1.md`)
em uma frase, um recorte de MVP e a tensão central do projeto. É a ponte entre a
visão aberta e as RFCs concretas (`pontos-chave.md` + `rfcs/`).

## 1. Em uma frase

Um RPG online 2.5D de **mundo contínuo e explorável** (sensação de Ragnarok) com
**profundidade econômica e tática** (herança de Dofus), executado por um **motor
compacto** que representa um mundo grande com pouquíssimos dados, desenvolvido por
uma **equipe humano+IA** com verificação dentro do jogo.

## 2. As três ambições e como elas se relacionam

1. **Game World** — o universo jogável (fim).
2. **Compact Engine** — a tecnologia que o representa e executa (meio técnico e
   diferencial de engenharia).
3. **AI Game Studio** — como produzimos tudo isso em escala com agentes (meio de
   produção).

Elas são **independentes por design**: o jogo tem que rodar sem a plataforma, e a
plataforma é uma frente que pode evoluir em paralelo. O motor pode começar sobre
uma base pronta (Bevy) e ir sendo substituído por partes só quando houver evidência.

## 3. A tensão central (e como resolvê-la)

O documento de visão é deliberadamente **aberto**: nada decidido. Isso é saudável
para acumular ideias, mas **RFCs exigem recortes**. O risco real, já nomeado na
própria visão, é o motor gráfico virar um projeto infinito e drenar o jogo.

Resolução proposta: **um MVP que seja um corte vertical fino**, não um dos três
produtos inteiro. O MVP prova o *princípio diferenciador* (mundo contínuo + densidade
compacta) com o mínimo de sistemas, e deixa combate/economia/MMO/studio como RFCs
posteriores.

## 4. Recorte de MVP (decidido 2026-11-14)

> **"Uma fatia jogável, online, de coleta e crafting num mundo contínuo."**

Decisões tomadas:

- **Fatia jogável** (não corte puramente técnico): há um **loop de jogo real**.
- **Online desde o início**: cliente-servidor com estado autoritativo no servidor.
- **Bevy como base** + um **renderer compacto próprio como módulo experimental**
  substituível.

O MVP demonstra, numa **zona feita à mão** (ainda não procedural):

- Um **personagem (sprite animado)** que se move **livre e continuamente** numa
  câmera isométrica oblíqua, com colisão/caminhabilidade.
- Um **loop de coleta → crafting** (herança sistêmica de Dofus): colher um recurso,
  fabricar um item, e isso **persistir no servidor** e ser visível a outros clientes.
- **Multiplayer autoritativo**: o servidor é dono do mundo; clientes sincronizam.
- Rodando sobre um **harness de dev com IA** que faz build, roda testes, tira
  screenshot e apresenta evidência a cada mudança.

**Loop escolhido = coleta/crafting** (e não combate) por ser o diferencial herdado
de Dofus, exercitar naturalmente persistência e sincronização online, e evitar o
poço sem fundo de balanceamento de combate. É **trocável** por "combate simples" se
preferir — ver RFC C1/C2.

O que fica **de propósito para depois** (P1+): representação compacta de assets e o
*50 MB Universe Challenge* (A3), bioma declarativo + procedural (B1), streaming de
chunks (B2 — a zona do MVP é pequena e cabe em memória), progressão de classes,
AI Game Studio como produto. **Nada disso é abandonado** — são a alma técnica do
projeto, apenas não bloqueiam a primeira fatia jogável. As decisões de dados do MVP
devem ser tomadas para **não inviabilizá-las**.

## 5. Caminho crítico do MVP (reconciliado)

```
Bevy (base) + módulo de render 2.5D  ──►  Zona à mão (tiles+sprites) + câmera iso
         │                                          │
         ▼                                          ▼
   Movimento contínuo + colisão (B3)   ──►   Loop coleta→crafting (C2)
         │                                          │
         ▼                                          ▼
   Servidor autoritativo + sync (D1)   ──►   Persistência do estado (recurso/item)
         │
         ▼
   Harness de IA (build/test/screenshot) (E1)
```

## 6. Princípios de decisão (herdados da visão)

- Medir separadamente: executável, assets, dados de mundo, memória de runtime.
- Procedural ≠ barato/bonito automaticamente: regras ruins = repetição sem intenção.
- Estudar a estética de referência **sem copiar assets, código ou IP**.
- Mundo aberto ≠ mundo infinito: streaming de chunks sustenta continuidade espacial.
- Sempre distinguir: hipótese · experimento · decisão · requisito confirmado.

---

**Próximo:** `pontos-chave.md` traduz esse recorte em pontos-chave de pesquisa e
num backlog de RFCs.
